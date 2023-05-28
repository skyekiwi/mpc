//! Message definitions for new parties that can join the protocol
//! Key points about a new party joining the refresh protocol:
//! * A new party wants to join, broadcasting a pailier ek, correctness of the ek generation,
//! dlog statements and dlog proofs.
//! * All the existing parties receives the join message. We assume for now that everyone accepts
//! the new party. All parties pick an index and add the new ek to their LocalKey at the given index.
//! * The party index of the new party is transmitted back to the joining party offchannel (it's
//! public information).
//! * All the existing parties enter the distribute phase, in which they start refreshing their
//! existing keys taking into the account the join messages that they received.
//! ** All parties (including new ones) collect the refresh messages and the join messages.

use super::error::{FsDkrError, FsDkrResult};
use super::refresh_message::RefreshMessage;
use skw_crypto_curv::arithmetic::{Zero};
use skw_crypto_curv::cryptographic_primitives::secret_sharing::feldman_vss::{
    ShamirSecretSharing, VerifiableSS,
};
// use skw_crypto_curv::elliptic::curves::traits::{ECPoint, ECScalar};
use skw_crypto_curv::BigInt;
use skw_crypto_curv::elliptic::curves::{Scalar, Point, Secp256k1};
use crate::gg20::party_i::{Keys, SharedKeys, generate_dlog_statement_proofs};
use crate::gg20::state_machine::keygen::LocalKey;

use skw_crypto_paillier::{Decrypt, EncryptionKey, Paillier};
use skw_crypto_zk_paillier::zkproofs::{CompositeDLogProof, DLogStatement, NiCorrectKeyProof};

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt::Debug;

/// Message used by new parties to join the protocol.
#[derive(Clone, Deserialize, Serialize, Debug)]
pub struct JoinMessage {
    pub(crate) ek: EncryptionKey,
    pub(crate) dk_correctness_proof: NiCorrectKeyProof,
    pub(crate) party_index: u16,
    pub(crate) dlog_statement_base_h1: DLogStatement,
    pub(crate) dlog_statement_base_h2: DLogStatement,
    pub(crate) composite_dlog_proof_base_h1: CompositeDLogProof,
    pub(crate) composite_dlog_proof_base_h2: CompositeDLogProof,
}

impl JoinMessage {
    /// The distribute phase for a new party. This distribute phase has to happen before the existing
    /// parties distribute. Calling this function will generate a JoinMessage and a pair of Pailier
    /// [Keys] that are going to be used when generating the [LocalKey].
    pub fn distribute(party_index: u16) -> (Self, Keys) {
        let pailier_key_pair = Keys::create(0);
        let (
            dlog_statement_base_h1,
            dlog_statement_base_h2,
            composite_dlog_proof_base_h1,
            composite_dlog_proof_base_h2,
        ) = generate_dlog_statement_proofs();

        let join_message = JoinMessage {
            // in a join message, we only care about the ek and the correctness proof
            ek: pailier_key_pair.ek.clone(),
            dk_correctness_proof: NiCorrectKeyProof::proof(&pailier_key_pair.dk, None),
            dlog_statement_base_h1,
            dlog_statement_base_h2,
            composite_dlog_proof_base_h1,
            composite_dlog_proof_base_h2,
            party_index,
        };

        (join_message, pailier_key_pair)
    }
    /// Returns the party index if it has been assigned one, throws
    /// [FsDkrError::NewPartyUnassignedIndexError] otherwise
    pub fn get_party_index(&self) -> u16 {
        self.party_index
    }

    /// Collect phase of the protocol. Compared to the [RefreshMessage::collect], this has to be
    /// tailored for a sent JoinMessage on which we assigned party_index. In this collect, a [LocalKey]
    /// is filled with the information provided by the [RefreshMessage]s from the other parties and
    /// the other join messages (multiple parties can be added/replaced at once).
    pub fn collect(
        &self,
        refresh_messages: &[RefreshMessage],
        paillier_key: Keys,
        join_messages: &[JoinMessage],
        t: u16,
        n: u16,
    ) -> FsDkrResult<LocalKey<Secp256k1>> {
        RefreshMessage::validate_collect(refresh_messages, t, n)?;

        // check if a party_index has been assigned to the current party
        let party_index = self.get_party_index();

        // check if a party_index has been assigned to all other new parties
        // TODO: Check if no party_index collision exists
        for join_message in join_messages.iter() {
            join_message.get_party_index();
        }

        let parameters = ShamirSecretSharing {
            threshold: t,
            share_count: n,
        };

        // generate a new share, the details can be found here https://hackmd.io/@omershlo/Hy1jBo6JY.
        let (cipher_text_sum, li_vec) = RefreshMessage::get_ciphertext_sum(
            refresh_messages,
            party_index,
            &parameters,
            &paillier_key.ek,
        );
        let new_share = Paillier::decrypt(&paillier_key.dk, cipher_text_sum)
            .0
            .into_owned();

        let new_share_fe = Scalar::from(&new_share);
        let paillier_dk = paillier_key.dk.clone();
        let key_linear_x_i = new_share_fe.clone();
        let key_linear_y = Point::generator() * new_share_fe.clone();
        let keys_linear = SharedKeys {
            x_i: key_linear_x_i,
            y: key_linear_y,
        };
        let mut pk_vec: Vec<_> = (0..n as usize)
            .map(|i| refresh_messages[0].points_committed_vec[i].clone() * li_vec[0].clone())
            .collect();

        for i in 0..n as usize {
            for j in 1..(t as usize + 1) {
                pk_vec[i] = pk_vec[i].clone()
                    + refresh_messages[j].points_committed_vec[i].clone() * li_vec[j].clone();
            }
        }

        // check what parties are assigned in the current rotation and associate their paillier
        // ek to each available party index.
        let available_parties: HashMap<u16, &EncryptionKey> = refresh_messages
            .iter()
            .map(|msg| (msg.party_index, &msg.ek))
            .chain(std::iter::once((party_index, &paillier_key.ek)))
            .chain(
                join_messages
                    .iter()
                    .map(|join_message| (join_message.party_index, &join_message.ek)),
            )
            .collect();

        // TODO: submit the statement the dlog proof as well!
        // check what parties are assigned in the current rotation and associate their DLogStatements
        // and check their CompositeDlogProofs.
        let available_h1_h2_ntilde_vec: HashMap<u16, &DLogStatement> = refresh_messages
            .iter()
            .map(|msg| (msg.party_index, &msg.dlog_statement))
            .chain(std::iter::once((party_index, &self.dlog_statement_base_h1)))
            .chain(join_messages.iter().map(|join_message| {
                (
                    join_message.party_index,
                    &join_message.dlog_statement_base_h1,
                )
            }))
            .collect();

        // generate the paillier public key vec needed for the LocalKey generation.
        let paillier_key_vec: Vec<EncryptionKey> = (1..n + 1)
            .map(|party| {
                let ek = available_parties.get(&party);

                match ek {
                    None => EncryptionKey {
                        n: BigInt::zero(),
                        nn: BigInt::zero(),
                    },
                    Some(key) => (*key).clone(),
                }
            })
            .collect();
        // generate the DLogStatement vec needed for the LocalKey generation.
        let h1_h2_ntilde_vec: Vec<DLogStatement> = (1..n + 1)
            .map(|party| {
                let statement = available_h1_h2_ntilde_vec.get(&party);

                match statement {
                    None => generate_dlog_statement_proofs().0,
                    Some(dlog_statement) => (*dlog_statement).clone(),
                }
            })
            .collect();

        // check if all the existing parties submitted the same public key. If they differ, abort.
        // TODO: this should be verifiable?
        for refresh_message in refresh_messages.iter() {
            if refresh_message.public_key != refresh_messages[0].public_key {
                return Err(FsDkrError::BroadcastedPublicKeyError);
            }
        }

        // generate the vss_scheme for the LocalKey
        let (vss_scheme, _) = VerifiableSS::<Secp256k1>::share(t, n, &new_share_fe);
        // TODO: secret cleanup might be needed.

        let local_key = LocalKey {
            paillier_dk,
            pk_vec,
            keys_linear,
            paillier_key_vec,
            y_sum_s: refresh_messages[0].public_key.clone(),
            h1_h2_n_tilde_vec: h1_h2_ntilde_vec,
            vss_scheme,
            i: party_index as u16,
            t: t as u16,
            n: n as u16,
        };

        Ok(local_key)
    }
}