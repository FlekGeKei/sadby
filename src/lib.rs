mod default_impls;

pub use sadaby_macro::Sadaby;

#[derive(Debug, PartialEq, Eq)]
pub enum SadabyError {
    UnexpectedToken,
}

pub trait Sadaby: Sized {
    fn se_bytes(&self) -> Vec<u8>;
    fn de_bytes(input: &[u8]) -> Result<Self, SadabyError>;
}
