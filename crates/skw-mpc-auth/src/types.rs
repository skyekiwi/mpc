pub const SECRET_LEN: usize = 32;
pub const CODE_LEN: usize = 6;

pub type Timestamp = u64;

#[derive(Debug)]
pub enum MpcAuthError {
    WrongSecretSize,
    InvalidBase32Encode,
    BadCode,
}
