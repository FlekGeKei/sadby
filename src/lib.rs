mod default_impls;

pub use sadby_macro::Sadby;

#[derive(Debug, PartialEq, Eq)]
pub enum SadbyError {
    UnexpectedToken,
}

pub trait Sadby: Sized {
    fn se_bytes(&self) -> Vec<u8>;
    fn de_bytes(input: &[u8]) -> Result<Self, SadbyError>;
}
