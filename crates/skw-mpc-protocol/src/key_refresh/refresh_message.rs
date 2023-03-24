use super::{
    add_party_message::JoinMessage,
    error::{FsDkrError, FsDkrResult}, PAILLIER_KEY_SIZE
};
use crate::utilities::mta::range_proofs::AliceProof;
use crate::utilities::zk_pdl_with_slack::{PDLwSlackProof, PDLwSlackStatement, PDLwSlackWitness};

use skw_crypto_curv::{arithmetic::{BitManipulation, Samplable, Zero}, elliptic::curves::{Secp256k1, Point, Scalar}};
use skw_crypto_curv::cryptographic_primitives::secret_sharing::feldman_vss::{
    ShamirSecretSharing, VerifiableSS,
};
use skw_crypto_curv::BigInt;

use crate::gg20::state_machine::keygen::LocalKey;

pub use skw_crypto_paillier::DecryptionKey;
use skw_crypto_paillier::{
    Add, Decrypt, Encrypt, EncryptWithChosenRandomness, EncryptionKey, KeyGeneration, Mul,
    Paillier, Randomness, RawCiphertext, RawPlaintext,
};
use skw_crypto_zk_paillier::zkproofs::{DLogStatement, NiCorrectKeyProof, SALT_STRING};

use serde::{Deserialize, Serialize};
use zeroize::Zeroize;

// Everything here can be broadcasted
#[derive(Clone, Deserialize, Serialize)]
pub struct RefreshMessage {
    pub(crate) party_index: u16,
    pdl_proof_vec: Vec<PDLwSlackProof<Secp256k1>>,
    range_proofs: Vec<AliceProof>,
    coefficients_committed_vec: VerifiableSS<Secp256k1>,
    pub(crate) points_committed_vec: Vec<Point<Secp256k1>>,
    points_encrypted_vec: Vec<BigInt>,
    dk_correctness_proof: NiCorrectKeyProof,
    pub(crate) dlog_statement: DLogStatement,
    pub(crate) ek: EncryptionKey,
    pub(crate) remove_party_indices: Vec<u16>,
    pub(crate) public_key: Point<Secp256k1>,
}

impl RefreshMessage {
    pub fn distribute(local_key: &LocalKey<Secp256k1>) -> (Self, DecryptionKey) {
        let secret = local_key.keys_linear.x_i.clone();
        // secret share old key
        let (vss_scheme, secret_shares) =
            VerifiableSS::share(local_key.t, local_key.n, &secret);

        // commit to points on the polynomial
        let points_committed_vec: Vec<_> = (0..secret_shares.len())
            .map(|i| Point::generator() * secret_shares[i].clone())
            .collect();

        //encrypt points on the polynomial using Paillier keys
        let (points_encrypted_vec, randomness_vec): (Vec<_>, Vec<_>) = (0..secret_shares.len())
            .map(|i| {
                let randomness = BigInt::sample_below(&local_key.paillier_key_vec[i].n);
                let ciphertext = Paillier::encrypt_with_chosen_randomness(
                    &local_key.paillier_key_vec[i],
                    RawPlaintext::from(secret_shares[i].to_bigint()),
                    &Randomness::from(randomness.clone()),
                )
                .0
                .into_owned();
                (ciphertext, randomness)
            })
            .unzip();

        // generate PDL proofs for each {point_committed, point_encrypted} pair
        let pdl_proof_vec: Vec<_> = (0..secret_shares.len())
            .map(|i| {
                let witness = PDLwSlackWitness {
                    x: secret_shares[i].clone(),
                    r: randomness_vec[i].clone(),
                };
                let statement = PDLwSlackStatement {
                    ciphertext: points_encrypted_vec[i].clone(),
                    ek: local_key.paillier_key_vec[i].clone(),
                    Q: points_committed_vec[i].clone(),
                    G: Point::generator().into(),
                    h1: local_key.h1_h2_n_tilde_vec[i].g.clone(),
                    h2: local_key.h1_h2_n_tilde_vec[i].ni.clone(),
                    N_tilde: local_key.h1_h2_n_tilde_vec[i].N.clone(),
                };
                PDLwSlackProof::prove(&witness, &statement)
            })
            .collect();

        let range_proofs = (0..secret_shares.len())
            .map(|i| {
                AliceProof::generate(
                    &secret_shares[i].to_bigint(),
                    &points_encrypted_vec[i],
                    &local_key.paillier_key_vec[i],
                    &local_key.h1_h2_n_tilde_vec[i],
                    &randomness_vec[i],
                )
            })
            .collect();

        let (ek, dk) = Paillier::keypair_with_modulus_size(PAILLIER_KEY_SIZE).keys();
        let dk_correctness_proof = NiCorrectKeyProof::proof(&dk, None);

        (
            RefreshMessage {
                party_index: local_key.i,
                pdl_proof_vec,
                range_proofs,
                coefficients_committed_vec: vss_scheme,
                points_committed_vec,
                points_encrypted_vec,
                dk_correctness_proof,
                dlog_statement: local_key.h1_h2_n_tilde_vec[(local_key.i - 1) as usize].clone(),
                ek,
                remove_party_indices: Vec::new(),
                public_key: local_key.y_sum_s.clone(),
            },
            dk,
        )
    }

    pub fn validate_collect(refresh_messages: &[Self], t: u16, n: u16) -> FsDkrResult<()> {
        // check we got at least threshold t refresh messages
        if refresh_messages.len() <= t as usize {
            return Err(FsDkrError::PartiesThresholdViolation {
                threshold: t as u16,
                refreshed_keys: refresh_messages.len() as u16,
            });
        }

        // check all vectors are of same length
        let reference_len = refresh_messages[0].pdl_proof_vec.len();

        for (k, refresh_message) in refresh_messages.iter().enumerate() {
            let pdl_proof_len = refresh_message.pdl_proof_vec.len();
            let points_commited_len = refresh_message.points_committed_vec.len();
            let points_encrypted_len = refresh_message.points_encrypted_vec.len();

            if !(pdl_proof_len == reference_len
                && points_commited_len == reference_len
                && points_encrypted_len == reference_len)
            {
                return Err(FsDkrError::SizeMismatchError {
                    refresh_message_index: k as u16,
                    pdl_proof_len,
                    points_commited_len,
                    points_encrypted_len,
                });
            }
        }

        for refresh_message in refresh_messages.iter() {
            for i in 0..n {
                //TODO: we should handle the case of t<i<n
                if refresh_message
                    .coefficients_committed_vec
                    .validate_share_public(&refresh_message.points_committed_vec[i as usize], i + 1)
                    .is_err()
                {
                    return Err(FsDkrError::PublicShareValidationError);
                }
            }
        }

        Ok(())
    }

    pub(crate) fn get_ciphertext_sum<'a>(
        refresh_messages: &'a [Self],
        party_index: u16,
        parameters: &'a ShamirSecretSharing,
        ek: &'a EncryptionKey,
    ) -> (RawCiphertext<'a>, Vec<Scalar<Secp256k1>>) {
        // TODO: check we have large enough qualified set , at least t+1
        //decrypt the new share
        // we first homomorphically add all ciphertext encrypted using our encryption key
        let ciphertext_vec: Vec<_> = (0..refresh_messages.len())
            .map(|k| refresh_messages[k].points_encrypted_vec[(party_index - 1) as usize].clone())
            .collect();

        let indices: Vec<u16> = (0..parameters.threshold + 1)
            .map(|i| refresh_messages[i as usize].party_index - 1)
            .collect();

        // optimization - one decryption
        let li_vec: Vec<_> = (0..parameters.threshold as usize + 1)
            .map(|i| {
                VerifiableSS::<Secp256k1>::map_share_to_new_params(
                    &parameters.clone(),
                    indices[i],
                    &indices,
                )
            })
            .collect();

        let ciphertext_vec_at_indices_mapped: Vec<_> = (0..(parameters.threshold + 1) as usize)
            .map(|i| {
                Paillier::mul(
                    ek,
                    RawCiphertext::from(ciphertext_vec[i].clone()),
                    RawPlaintext::from(li_vec[i].to_bigint()),
                )
            })
            .collect();

        let ciphertext_sum = ciphertext_vec_at_indices_mapped.iter().fold(
            Paillier::encrypt(ek, RawPlaintext::from(BigInt::zero())),
            |acc, x| Paillier::add(ek, acc, x.clone()),
        );

        (ciphertext_sum, li_vec)
    }

    pub fn replace(
        new_parties: &[JoinMessage],
        key: &mut LocalKey<Secp256k1>,
    ) -> FsDkrResult<(Self, DecryptionKey)> {
        for join_message in new_parties.iter() {
            let party_index = join_message.get_party_index() as usize;
            key.paillier_key_vec[party_index - 1] = join_message.ek.clone();
            key.h1_h2_n_tilde_vec[party_index - 1] = join_message.dlog_statement_base_h1.clone();
        }

        Ok(RefreshMessage::distribute(key))
    }

    pub fn collect(
        refresh_messages: &[Self],
        mut local_key: &mut LocalKey<Secp256k1>,
        new_dk: DecryptionKey,
        join_messages: &[JoinMessage],
    ) -> FsDkrResult<()> {
        RefreshMessage::validate_collect(
            refresh_messages,
            local_key.t,
            local_key.n,
        )?;

        for refresh_message in refresh_messages.iter() {
            for i in 0..(local_key.n as usize) {
                let statement = PDLwSlackStatement {
                    ciphertext: refresh_message.points_encrypted_vec[i].clone(),
                    ek: local_key.paillier_key_vec[i].clone(),
                    Q: refresh_message.points_committed_vec[i].clone(),
                    G: Point::generator().into(),
                    h1: local_key.h1_h2_n_tilde_vec[i].g.clone(),
                    h2: local_key.h1_h2_n_tilde_vec[i].ni.clone(),
                    N_tilde: local_key.h1_h2_n_tilde_vec[i].N.clone(),
                };

                // TODO: make the err more explicit
                refresh_message.pdl_proof_vec[i].verify(&statement).map_err(|_| FsDkrError::PDLwSlackProof{
                    is_u1_eq: false,
                    is_u2_eq: false,
                    is_u3_eq: false,
                })?;

                if !refresh_message.range_proofs[i].verify(
                    &statement.ciphertext,
                    &statement.ek,
                    &local_key.h1_h2_n_tilde_vec[i],
                ) {
                    return Err(FsDkrError::RangeProof { party_index: i as u16 });
                }
            }
        }

        let old_ek = local_key.paillier_key_vec[(local_key.i - 1) as usize].clone();
        let (cipher_text_sum, li_vec) = RefreshMessage::get_ciphertext_sum(
            refresh_messages,
            local_key.i,
            &local_key.vss_scheme.parameters,
            &old_ek,
        );

        for refresh_message in refresh_messages.iter() {
            if refresh_message
                .dk_correctness_proof
                .verify(&refresh_message.ek, SALT_STRING)
                .is_err()
            {
                return Err(FsDkrError::PaillierVerificationError {
                    party_index: refresh_message.party_index,
                });
            }
            let n_length = refresh_message.ek.n.bit_length();
            if n_length > PAILLIER_KEY_SIZE || n_length < PAILLIER_KEY_SIZE - 1 {
                return Err(FsDkrError::MouliTooSmall {
                    party_index: refresh_message.party_index,
                    moduli_size: n_length,
                });
            }

            // if the proof checks, we add the new paillier public key to the key
            local_key.paillier_key_vec[(refresh_message.party_index - 1) as usize] =
                refresh_message.ek.clone();
        }

        for join_message in join_messages {
            let party_index = join_message.get_party_index();

            if join_message
                .dk_correctness_proof
                .verify(&join_message.ek, SALT_STRING)
                .is_err()
            {
                return Err(FsDkrError::PaillierVerificationError { party_index });
            }

            if join_message
                .composite_dlog_proof_base_h1
                .verify(&join_message.dlog_statement_base_h1)
                .is_err()
                || join_message
                    .composite_dlog_proof_base_h2
                    .verify(&join_message.dlog_statement_base_h2)
                    .is_err()
            {
                return Err(FsDkrError::DLogProofValidation { party_index });
            }

            let n_length = join_message.ek.n.bit_length();
            if n_length > PAILLIER_KEY_SIZE || n_length < PAILLIER_KEY_SIZE - 1 {
                return Err(FsDkrError::MouliTooSmall {
                    party_index: join_message.get_party_index() as u16,
                    moduli_size: n_length,
                });
            }

            // if the proof checks, we add the new paillier public key to the key
            local_key.paillier_key_vec[(party_index - 1) as usize] = join_message.ek.clone();
        }

        let new_share = Paillier::decrypt(&local_key.paillier_dk, cipher_text_sum)
            .0
            .into_owned();

        let new_share_fe = Scalar::from(&new_share);

        // zeroize the old dk key
        local_key.paillier_dk.q.zeroize();
        local_key.paillier_dk.p.zeroize();
        local_key.paillier_dk = new_dk;

        // TODO: zeroize
        // update old key and output new key
        // local_key.keys_linear.x_i.zeroize();

        local_key.keys_linear.x_i = new_share_fe.clone();
        local_key.keys_linear.y = Point::generator() * new_share_fe;

        // update local key list of local public keys (X_i = g^x_i is updated by adding all committed points to that party)
        for i in 0..local_key.n as usize {
            local_key.pk_vec[i] =
                refresh_messages[0].points_committed_vec[i].clone() * li_vec[0].clone();
            for j in 1..local_key.t as usize + 1 {
                local_key.pk_vec[i] = local_key.pk_vec[i].clone()
                    + refresh_messages[j].points_committed_vec[i].clone() * li_vec[j].clone();
            }
        }

        Ok(())
    }
}
