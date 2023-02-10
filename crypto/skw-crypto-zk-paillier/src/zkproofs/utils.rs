use std::borrow::Borrow;

use skw_crypto_curv::arithmetic::traits::*;
use skw_crypto_curv::BigInt;

use digest::Digest;
use sha2::Sha256;

pub fn compute_digest<IT>(it: IT) -> BigInt
where
    IT: Iterator,
    IT::Item: Borrow<BigInt>,
{
    let mut hasher = Sha256::new();
    for value in it {
        let bytes: Vec<u8> = value.borrow().to_bytes();
        hasher.update(&bytes);
    }

    let result_bytes = hasher.finalize();
    BigInt::from_bytes(&result_bytes[..])
}
