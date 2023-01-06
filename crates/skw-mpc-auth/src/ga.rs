use serde::{Serialize, Deserialize};
use blake2::{Blake2s256, Digest};
use crate::{types::{Timestamp, MpcAuthError}, utils::base32_encode, auth::BaseAuth, AuthCode};
use crate::utils::{get_time};

/// Google Authenticator authentication protocol
/// 
/// 
const EXPIRATION_DISCREPANCY: Timestamp = 30; // 30 seconds, default by GA

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GaAuth {
    random_seed: [u8; 32],

    init_time: Timestamp,
}

impl GaAuth {
    pub fn new(
        random_seed: [u8; 32],
        init_time: Timestamp
    ) -> Self {
        let t = get_time(init_time);
        Self {
            random_seed, 
            init_time: t,
        }
    }

    pub fn get_secret(&self) -> String {
        let mut h = Blake2s256::new();
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
    use super::GaAuth;

    #[test]
    fn gen_code_and_verify() {
        let a = GaAuth::new(
            rand::random::<[u8; 32]>(),
            0
        );
        
        let auth = a.get_code(None).unwrap();

        println!("{:?}", auth);

        assert_eq!(auth.validate(), true);
    }

}