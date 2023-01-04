/// A light implementation of Google Authenticator

use hmacsha1::hmac_sha1;
use std::time::{SystemTime, UNIX_EPOCH};

pub struct MpcAuth {}

const SECRET_LEN: usize = 32;
const CODE_LEN: usize = 6;

#[derive(Debug)]
pub enum MpcAuthError {
    WrongSecretSize,
    InvalidBase32Encode,
}

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

pub fn get_time(time: u64) -> u64 {
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

impl MpcAuth {

    pub fn get_code(
        secret: &str,
        time: u64
    ) -> Result<[u8; CODE_LEN], MpcAuthError> {
        let s = base32_decode(secret)?;
        
        let t = get_time(time);

        let hash = hmac_sha1(&s, &t.to_be_bytes());
        let offset = hash[hash.len() - 1] & 0x0F;
        let mut truncated_hash: [u8; 4] = Default::default();

        truncated_hash.copy_from_slice(&hash[offset as usize..(offset + 4) as usize]);
        let mut code = i32::from_be_bytes(truncated_hash);
        code &= 0x7FFFFFFF;
        code %= 10_i32.checked_pow(
            u32::try_from(CODE_LEN).expect("never fails")
        ).expect("code length overflow");
        
        // NOTE: sCODE_LEN is hardcoded here
        let res: Vec<u8> = format!("{:0>6}", code)
            .to_string()
            .chars()
            .map(|c| u8::try_from(
                    c.to_digit(10).expect("digit conversion error")
                ).expect("all digits should be within u8 size")
            )
            .collect();

        Ok(res.try_into().expect("output code should always be 6 digits long"))
    }

    pub fn verify_code(
        secret: &str,
        code: &[u8; CODE_LEN],
        time_discrepancy: u64,
        time: u64
    ) -> bool {
        let sc = base32_decode(secret) ;

        match sc {
            Ok(_) => {
                let t = get_time(time);
                let lower_bound = t.saturating_sub(time_discrepancy);
                let upper_bound = t.saturating_add(time_discrepancy);

                for tm in lower_bound..upper_bound {
                    if let Ok(c) = MpcAuth::get_code(secret, tm) {
                        if c == *code {
                            return true
                        }
                    }
                }
                false
            },
            _ => false
        }
    }
}

#[test]
fn e2e() {
    let code = MpcAuth::get_code("H6ORCEULNB4LSP2XXYZFPC4NPADXEEC6", 0).unwrap();
    let verify = MpcAuth::verify_code("H6ORCEULNB4LSP2XXYZFPC4NPADXEEC6", 
        &code.clone(), 1, 0);

    println!("{:?} {:?}", code, verify);
}