use std::time::{SystemTime, UNIX_EPOCH};
use crate::types::{SECRET_LEN, MpcAuthError, Timestamp};

pub fn base32_decode(secret: &str) -> Result<Vec<u8>, MpcAuthError> {
    if secret.len() != SECRET_LEN {
        return Err(MpcAuthError::WrongSecretSize);
    }
    match base32::decode(base32::Alphabet::RFC4648 { padding: true }, secret) {
        Some(s) => Ok(s),
        _ => Err(MpcAuthError::InvalidBase32Encode),
    }
}

pub fn base32_encode(s: &[u8; 32]) -> String {
    base32::encode(base32::Alphabet::RFC4648 { padding: true }, &s[..])
}

pub fn get_time(time: Timestamp) -> Timestamp {
    if time == 0 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs()
            / 30
    } else {
        time
    }
}
