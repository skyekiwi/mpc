use serde::{Serialize, Deserialize};
use blake2::{Blake2s256, Digest};
use crate::{types::{Timestamp, MpcAuthError}, utils::base32_encode, auth::BaseAuth, AuthCode};
use crate::utils::{get_time};

/// Email authentication protocol
/// 
/// 

const EXPIRATION_DISCREPANCY: Timestamp = 300; // 5mins

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmailAuth {
    email: String,
    random_seed: [u8; 32],

    init_time: Timestamp,
    expires: Timestamp,
}

impl EmailAuth {
    pub fn new(
        email: &str,
        random_seed: [u8; 32],

        init_time: Timestamp
    ) -> Self {
        let t = get_time(init_time);
        Self {
            email: email.to_string(),
            random_seed, 
            init_time: t,
            expires: t.saturating_add(EXPIRATION_DISCREPANCY)
        }
    }

    pub fn get_secret(&self) -> String {
        let mut h = Blake2s256::new();
        h.update(&self.email[..]);
        h.update(&self.random_seed[..]);

        let hash = h.finalize();
        base32_encode(&hash.into())[..32].to_string()
    }
 
    pub fn get_code(&self, t: Option<Timestamp>) -> Result<AuthCode, MpcAuthError> {
        let secret = self.get_secret();

        let time = t.unwrap_or(0);
        let code = BaseAuth::get_code(&secret, time)?;

        Ok(AuthCode::new(&secret, code, time, EXPIRATION_DISCREPANCY))
    }

}

#[cfg(test)]
mod test {
    use super::EmailAuth;

    #[test]
    fn gen_code_and_verify() {
        let a = EmailAuth::new(
            "test@skye.kiwi",
            rand::random::<[u8; 32]>(),
            0
        );
        
        let auth = a.get_code(None).unwrap();

        println!("{:?}", auth);

        assert_eq!(auth.validate(), true);
    }

}