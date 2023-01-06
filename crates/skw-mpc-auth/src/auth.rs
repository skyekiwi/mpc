/// A light implementation of Google Authenticator

use hmacsha1::hmac_sha1;
use crate::utils::{base32_decode, get_time};
use crate::types::{CODE_LEN, MpcAuthError, Timestamp, SECRET_LEN};

pub struct BaseAuth {}

impl BaseAuth {
    pub fn get_code(
        secret: &str,
        time: Timestamp
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

    pub fn verify_code_inner(
        secret: &str,
        code: &[u8; CODE_LEN],
        time_discrepancy: Timestamp,
        time: Timestamp
    ) -> Result<(), MpcAuthError> {
        let _ = base32_decode(secret)?;
        let t = get_time(time);
        let lower_bound = t.saturating_sub(time_discrepancy);
        let upper_bound = t.saturating_add(time_discrepancy);

        for tm in lower_bound..upper_bound {
            if let Ok(c) = BaseAuth::get_code(secret, tm) {
                if c == *code {
                    return Ok(())
                }
            }
        }

        Err(MpcAuthError::BadCode)
    }

    pub fn verify_code(
        secret: &str,
        code: &[u8; CODE_LEN],
        time_discrepancy: Timestamp,
        time: Timestamp
    ) -> bool {
        if let Ok(_) = BaseAuth::verify_code_inner(
            secret, code, time_discrepancy, time
        ) {
            true
        } else {
            false
        }
    }

    pub fn verify_code_raw(
        secret_code: &[u8; SECRET_LEN],
        code: &[u8; CODE_LEN],
        time_discrepancy: Timestamp,
        time: Timestamp
    ) -> bool {
        if let Ok(secret) = std::str::from_utf8(&secret_code[..]) {
            BaseAuth::verify_code(secret, code, time_discrepancy, time)
        } else {
            false
        }
    }
}

#[test]
fn basic_auth_works() {
    let code = BaseAuth::get_code("H6ORCEULNB4LSP2XXYZFPC4NPADXEEC6", 0).unwrap();
    let verify = BaseAuth::verify_code("H6ORCEULNB4LSP2XXYZFPC4NPADXEEC6", 
        &code.clone(), 1, 0);

    println!("{:?} {:?}", code, verify);
}