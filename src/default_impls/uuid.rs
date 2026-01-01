use super::*;

use ::uuid::Uuid;

impl Sadby for Uuid {
    fn se_bytes(&self) -> Vec<u8> {
        self.to_bytes_le().into()
    }
    fn de_bytes(input: &[u8]) -> Result<Self, SadbyError> {
        Ok(Uuid::from_bytes_le(<[u8; 16]>::de_bytes(&input[0..16])?))
    }
}
