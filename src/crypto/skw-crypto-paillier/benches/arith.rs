use bencher::{benchmark_group, benchmark_main, Bencher};
use skw_crypto_curv::arithmetic::traits::*;

use skw_crypto_paillier::*;

mod helpers;
use helpers::*;

pub fn bench_mul(b: &mut Bencher) {
    let p: &BigInt = &BigInt::from_str_radix(P2048, 10).unwrap();
    let q: &BigInt = &BigInt::from_str_radix(Q2048, 10).unwrap();

    b.iter(|| {
        let _ = p * q;
    });
}

pub fn bench_mulrem(b: &mut Bencher) {
    let p: &BigInt = &BigInt::from_str_radix(P2048, 10).unwrap();
    let q: &BigInt = &BigInt::from_str_radix(Q2048, 10).unwrap();
    let n: &BigInt = &BigInt::from_str_radix(N2048, 10).unwrap();

    b.iter(|| {
        let _ = (p * q) % n;
    });
}

pub fn bench_modarith(b: &mut Bencher) {
    let p: &BigInt = &BigInt::from_str_radix(P2048, 10).unwrap();
    let q: &BigInt = &BigInt::from_str_radix(Q2048, 10).unwrap();
    let n: &BigInt = &BigInt::from_str_radix(N2048, 10).unwrap();

    b.iter(|| {
        let _ = BigInt::mod_pow(p, q, n);
    });
}

benchmark_group!(
    group,
    self::bench_mul,
    self::bench_mulrem,
    self::bench_modarith
);

benchmark_main!(group);
