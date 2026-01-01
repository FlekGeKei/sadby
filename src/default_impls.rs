#[cfg(feature = "uuid")]
mod uuid;

pub use super::*;

macro_rules! sadaby_ints {
    ($( $type:ty ),*) => {
        $(
            impl Sadaby for $type {
                fn se_bytes(&self) -> Vec<u8> {
                    self.to_le_bytes().into()
                }
                fn de_bytes(input: &[u8]) -> Result<Self, SadabyError> {
                    Ok(Self::from_le_bytes(<[u8; const { std::mem::size_of::<$type>() }]>::de_bytes(&input[0..const { std::mem::size_of::<$type>() }])?))
                }
            }
        )*
    };
}

sadaby_ints!(u16, u32, u64, u128, usize, i16, i32, i64, i128, isize, f32, f64);

impl Sadaby for u8 {
    fn se_bytes(&self) -> Vec<u8> {
        vec![*self]
    }
    fn de_bytes(input: &[u8]) -> Result<Self, SadabyError> {
        Ok(input[0])
    }
}
impl Sadaby for i8 {
    fn se_bytes(&self) -> Vec<u8> {
        vec![self.to_le_bytes()[0]]
    }
    fn de_bytes(input: &[u8]) -> Result<Self, SadabyError> {
        Ok(Self::from_le_bytes([input[0]]))
    }
}
impl Sadaby for char {
    fn se_bytes(&self) -> Vec<u8> {
        vec![*self as u8]
    }
    fn de_bytes(input: &[u8]) -> Result<Self, SadabyError> {
        Ok(input[0] as Self)
    }
}
/*
 * Waiting for:
 * - https://github.com/rust-lang/rust/issues/42721
 * - https://github.com/rust-lang/rust/issues/31844
impl<const N: usize, T: Sadaby + Default + Copy> SerDeBytes for [T; N] {
    fn se_bytes(&self) -> Vec<u8> {
        let mut buf = Vec::<u8>::new();
        for item in self {
            buf.append(&mut item.to_bytes());
        }
        buf
    }
    fn de_bytes(input: &[u8]) -> Result<Self, SadabyError> {
        let mut slice: [T; N] = [const { T::default() }; N];
        slice.copy_from_slice(&Vec::<T>::de_bytes(&input)?);
        Ok(slice)
    }
}
impl<const N: usize, T: Sadaby + Default + Clone + !Copy> SerDeBytes for [T; N] {
    fn se_bytes(&self) -> Vec<u8> {
        let mut buf = Vec::<u8>::new();
        for item in self {
            buf.append(&mut item.to_bytes());
        }
        buf
    }
    fn de_bytes(input: &[u8]) -> Result<Self, SadabyError> {
        let mut slice: [T; N] = [const { T::default() }; N];
        slice.clone_from_slice(&Vec::<T>::de_bytes(&input)?);
        Ok(slice)
    }
}
*/

impl<const N: usize> Sadaby for [u8; N] {
    fn se_bytes(&self) -> Vec<u8> {
        let mut buf = Vec::<u8>::new();
        buf.extend_from_slice(self);
        buf
    }
    fn de_bytes(input: &[u8]) -> Result<Self, SadabyError> {
        let mut slice: [u8; N] = [0u8; N];
        slice.copy_from_slice(&input[0..N]);
        Ok(slice)
    }
}
impl<const N: usize> Sadaby for [char; N] {
    fn se_bytes(&self) -> Vec<u8> {
        self.iter().map(|c| *c as u8).collect::<Vec<u8>>()
    }
    fn de_bytes(input: &[u8]) -> Result<Self, SadabyError> {
        let mut slice = ['\u{0}'; N];
        slice.copy_from_slice(&input.iter().map(|b| *b as char).collect::<Vec<char>>());

        Ok(slice)
    }
}
impl<const N: usize> Sadaby for [f32; N] {
    fn se_bytes(&self) -> Vec<u8> {
        let mut buf = Vec::<u8>::new();
        for float in self {
            buf.extend_from_slice(&float.to_le_bytes());
        }
        buf
    }
    fn de_bytes(input: &[u8]) -> Result<Self, SadabyError> {
        let mut slice = [0.; N];
        let mut buf = Vec::<f32>::new();

        let mut current = 0;
        let mut next = const { std::mem::size_of::<f32>() };

        while next <= (const { std::mem::size_of::<f32>() * N }) {
            buf.push(f32::de_bytes(&input[current..next])?);
            current = next;
            next += const { std::mem::size_of::<f32>() };
        }

        slice.copy_from_slice(&buf);

        Ok(slice)
    }
}
impl<T: Sadaby> Sadaby for Option<T> {
    fn se_bytes(&self) -> Vec<u8> {
        let mut buf = Vec::<u8>::new();
        match self {
            Some(s) => {
                buf.push(b'S');
                buf.extend_from_slice(&s.se_bytes());
            }
            None => buf.push(b'N'),
        }

        buf
    }
    fn de_bytes(input: &[u8]) -> Result<Self, SadabyError> {
        match input[0] {
            b'S' => Ok(Some(T::de_bytes(&input[1..])?)),
            b'N' => Ok(None),
            _ => Err(SadabyError::UnexpectedToken),
        }
    }
}

impl<T: Sadaby> Sadaby for Vec<T> {
    fn se_bytes(&self) -> Vec<u8> {
        let mut buf = Vec::<u8>::new();

        for item in self {
            let mut v_i = item.se_bytes();

            buf.push(v_i.len() as u8);
            buf.append(&mut v_i);
        }

        buf
    }
    fn de_bytes(input: &[u8]) -> Result<Self, SadabyError> {
        let mut output = Vec::<T>::new();

        #[allow(unused_assignments)]
        let mut current = 0usize;
        let mut next: isize = -1;

        while next as usize <= (input.len() - 1) {
            current = unsafe { (next + 1).try_into().unwrap_unchecked() };
            next = current as isize + input[current] as isize;
            current += 1;

            output.push(T::de_bytes(
                &input[current..=unsafe { next.try_into().unwrap_unchecked() }],
            )?);
        }

        Ok(output)
    }
}
impl<T: Sadaby> Sadaby for Box<[T]> {
    fn se_bytes(&self) -> Vec<u8> {
        let mut buf = Vec::<u8>::new();

        for item in self {
            let mut v_i = item.se_bytes();

            buf.push(v_i.len() as u8);
            buf.append(&mut v_i);
        }

        buf
    }
    fn de_bytes(input: &[u8]) -> Result<Self, SadabyError> {
        Ok(Vec::<T>::de_bytes(input)?.into())
    }
}
impl Sadaby for bool {
    fn se_bytes(&self) -> Vec<u8> {
        vec![*self as u8]
    }
    fn de_bytes(input: &[u8]) -> Result<Self, SadabyError> {
        match input[0] {
            0 => Ok(false),
            1 => Ok(true),
            _ => Err(SadabyError::UnexpectedToken),
        }
    }
}
impl Sadaby for String {
    fn se_bytes(&self) -> Vec<u8> {
        self.as_bytes().to_vec()
    }
    fn de_bytes(input: &[u8]) -> Result<Self, SadabyError> {
        Ok(input.iter().map(|b| *b as char).collect::<String>())
    }
}
impl<T: Sadaby, Y: Sadaby> Sadaby for (T, Y) {
    fn se_bytes(&self) -> Vec<u8> {
        let mut buf = Vec::new();
        let mut i = self.0.se_bytes();
        buf.push(i.len() as u8);
        buf.append(&mut i);
        buf.append(&mut self.1.se_bytes());
        buf
    }
    fn de_bytes(input: &[u8]) -> Result<Self, SadabyError> {
        let middle = input[0] as usize;
        Ok((
            T::de_bytes(&input[..middle])?,
            Y::de_bytes(&input[middle..])?,
        ))
    }
}
