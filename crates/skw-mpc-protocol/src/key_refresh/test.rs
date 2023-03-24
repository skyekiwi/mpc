#[cfg(test)]
mod tests {
    use crate::key_refresh::{RefreshMessage, FsDkrResult, JoinMessage};

    use crate::gg20::party_i::verify;
    use crate::gg20::party_i::Keys;
    use crate::gg20::state_machine::keygen::{
        Keygen, LocalKey,
    };
    use crate::gg20::state_machine::sign::{
        CompletedOfflineStage, OfflineStage, SignManual,
    };

    use skw_crypto_curv::arithmetic::Converter;
    use skw_crypto_curv::cryptographic_primitives::secret_sharing::feldman_vss::{
        ShamirSecretSharing, VerifiableSS,
    };
    use skw_crypto_curv::BigInt;
    

    use skw_crypto_curv::elliptic::curves::Secp256k1;
    use skw_crypto_paillier::DecryptionKey;
    use skw_round_based::dev::Simulation;
    use std::collections::HashMap;

    #[test]
    fn test1() {
        // std::env::set_var("RUST_LOG", "debug");
        // pretty_env_logger::init();
    
        //simulate keygen
        let t = 3;
        let n = 5;

        let mut keys = simulate_keygen(t, n);

        let old_keys = keys.clone();
        simulate_dkr(&mut keys);

        // check that sum of old keys is equal to sum of new keys
        let old_linear_secret_key: Vec<_> = (0..old_keys.len())
            .map(|i| old_keys[i].keys_linear.x_i.clone())
            .collect();

        let new_linear_secret_key: Vec<_> =
            (0..keys.len()).map(|i| keys[i].keys_linear.x_i.clone()).collect();
        let indices: Vec<_> = (0..t + 1).collect();
        let vss = VerifiableSS::<Secp256k1> {
            parameters: ShamirSecretSharing {
                threshold: t,
                share_count: n,
            },
            commitments: Vec::new(),
        };
        assert_eq!(
            vss.reconstruct(&indices[..], &old_linear_secret_key[0..(t + 1) as usize]),
            vss.reconstruct(&indices[..], &new_linear_secret_key[0..(t + 1) as usize])
        );
        assert_ne!(old_linear_secret_key, new_linear_secret_key);
    }

    #[test]
    fn test_sign_rotate_sign() {
        let mut keys = simulate_keygen(2, 5);
        let offline_sign = simulate_offline_stage(keys.clone(), &[1, 2, 3]);
        simulate_signing(offline_sign, b"ZenGo");
        simulate_dkr(&mut keys);
        let offline_sign = simulate_offline_stage(keys.clone(), &[2, 3, 4]);
        simulate_signing(offline_sign, b"ZenGo");
        simulate_dkr(&mut keys);
        let offline_sign = simulate_offline_stage(keys, &[1, 3, 5]);
        simulate_signing(offline_sign, b"ZenGo");
    }

    #[test]
    fn test_remove_sign_rotate_sign() {
        let mut keys = simulate_keygen(2, 5);
        let offline_sign = simulate_offline_stage(keys.clone(), &[1, 2, 3]);
        simulate_signing(offline_sign, b"ZenGo");
        simulate_dkr_removal(&mut keys, [1].to_vec());
        let offline_sign = simulate_offline_stage(keys.clone(), &[2, 3, 4]);
        simulate_signing(offline_sign, b"ZenGo");
        simulate_dkr_removal(&mut keys, [1, 2].to_vec());
        let offline_sign = simulate_offline_stage(keys, &[3, 4, 5]);
        simulate_signing(offline_sign, b"ZenGo");
    }

    #[test]
    fn test_add_party() {
        fn simulate_replace(
            keys: &mut Vec<LocalKey<Secp256k1>>,
            party_indices: &[u16],
            t: u16, n: u16,
        ) -> FsDkrResult<()> {
            fn generate_join_messages_and_keys(
                number_of_new_parties: usize,
            ) -> (Vec<JoinMessage>, Vec<Keys>) {
                // the new party generates it's join message to start joining the computation
                (0..number_of_new_parties)
                    .map(|i| JoinMessage::distribute((i + 1) as u16))
                    .unzip()
            }

            fn generate_refresh_parties_replace(
                keys: &mut [LocalKey<Secp256k1>],
                join_messages: &[JoinMessage],
            ) -> (Vec<RefreshMessage>, Vec<DecryptionKey>) {
                keys.iter_mut()
                    .map(|key| RefreshMessage::replace(join_messages, key).unwrap())
                    .unzip()
            }

            // each party that wants to join generates a join message and a pair of pailier keys.
            let (join_messages, new_keys) =
                generate_join_messages_and_keys(party_indices.len());

            // // each new party has to be informed through offchannel communication what party index
            // // it has been assigned (the information is public).
            // for (join_message, party_index) in join_messages.iter_mut().zip(party_indices) {
            //     join_message.party_index = Some(*party_index);
            // }

            // each existing party has to generate it's refresh message aware of the new parties
            let (refresh_messages, dk_keys) =
                generate_refresh_parties_replace(keys, join_messages.as_slice());

            // all existing parties rotate aware of the join_messages
            for i in 0..keys.len() as usize {
                RefreshMessage::collect(
                    refresh_messages.as_slice(),
                    &mut keys[i],
                    dk_keys[i].clone(),
                    join_messages.as_slice(),
                )
                .expect("");
            }

            // all new parties generate a local key
            for (join_message, dk) in join_messages.iter().zip(new_keys) {
                let party_index = join_message.party_index;
                let local_key = join_message.collect(
                    refresh_messages.as_slice(),
                    dk,
                    join_messages.as_slice(),
                    t,
                    n,
                )?;
                keys.insert((party_index - 1) as usize, local_key);
            }

            Ok(())
        }

        let t = 1;
        let n = 3;

        let all_keys = simulate_keygen(t, n);

        // remove the first two key - the client side key
        let mut keys = all_keys[1..].to_vec();

        // replace client key 
        simulate_replace(&mut keys, &[1], t, n).unwrap();

        // use client key and FNode1 key
        let offline_sign = simulate_offline_stage(keys, &[1, 2]);
        simulate_signing(offline_sign, b"ZenGo");
    }

    fn simulate_keygen(t: u16, n: u16) -> Vec<LocalKey<Secp256k1>> {
        //simulate keygen
        let mut simulation = Simulation::new();
        simulation.enable_benchmarks(false);

        for i in 1..=n {
            simulation.add_party(Keygen::new(i, t, n).unwrap());
        }

        simulation.run().unwrap()
    }

    fn simulate_dkr_removal(keys: &mut Vec<LocalKey<Secp256k1>>, remove_party_indices: Vec<u16>) {
        let mut broadcast_messages: HashMap<u16, Vec<RefreshMessage>> = HashMap::new();
        let mut new_dks: HashMap<u16, DecryptionKey> = HashMap::new();
        let mut refresh_messages: Vec<RefreshMessage> = Vec::new();
        let mut party_key: HashMap<u16, LocalKey<Secp256k1>> = HashMap::new();

        for key in keys.iter_mut() {
            let (refresh_message, new_dk) = RefreshMessage::distribute(key);
            refresh_messages.push(refresh_message.clone());
            new_dks.insert(refresh_message.party_index, new_dk);
            party_key.insert(refresh_message.party_index, key.clone());
        }

        for refresh_message in refresh_messages.iter() {
            broadcast_messages.insert(refresh_message.party_index, Vec::new());
        }

        for refresh_message in refresh_messages.iter_mut() {
            if !remove_party_indices.contains(&refresh_message.party_index) {
                refresh_message.remove_party_indices = remove_party_indices.clone();
            } else {
                let mut new_remove_party_indices = remove_party_indices.clone();
                new_remove_party_indices.retain(|value| *value != refresh_message.party_index);
                refresh_message.remove_party_indices = new_remove_party_indices;
            }

            for (party_index, refresh_bucket) in broadcast_messages.iter_mut() {
                if refresh_message.remove_party_indices.contains(party_index) {
                    continue;
                }
                refresh_bucket.push(refresh_message.clone());
            }
        }

        for remove_party_index in remove_party_indices.iter() {
            assert_eq!(broadcast_messages[remove_party_index].len(), 1);
        }

        // keys will be updated to refreshed values
        for (party, key) in party_key.iter_mut() {
            if remove_party_indices.contains(party) {
                continue;
            }

            RefreshMessage::collect(
                broadcast_messages[party].clone().as_slice(),
                key,
                new_dks[party].clone(),
                &[],
            )
            .expect("");
        }

        for remove_party_index in remove_party_indices {
            let result = RefreshMessage::collect(
                &broadcast_messages[&remove_party_index],
                &mut keys[remove_party_index as usize],
                new_dks[&remove_party_index].clone(),
                &[],
            );
            assert!(result.is_err());
        }
    }

    fn simulate_dkr(keys: &mut Vec<LocalKey<Secp256k1>>) -> (Vec<RefreshMessage>, Vec<DecryptionKey>) {
        let mut broadcast_vec: Vec<RefreshMessage> = Vec::new();
        let mut new_dks: Vec<DecryptionKey> = Vec::new();

        for key in keys.iter() {
            let (refresh_message, new_dk) = RefreshMessage::distribute(key);
            broadcast_vec.push(refresh_message);
            new_dks.push(new_dk);
        }

        // keys will be updated to refreshed values
        for i in 0..keys.len() as usize {
            RefreshMessage::collect(&broadcast_vec, &mut keys[i], new_dks[i].clone(), &[])
                .expect("");
        }

        (broadcast_vec, new_dks)
    }

    fn simulate_offline_stage(
        local_keys: Vec<LocalKey<Secp256k1>>,
        s_l: &[u16],
    ) -> Vec<CompletedOfflineStage> {
        let mut simulation = Simulation::new();
        simulation.enable_benchmarks(false);

        for (i, &keygen_i) in (1..).zip(s_l) {
            simulation.add_party(
                OfflineStage::new(
                    i,
                    s_l.to_vec(),
                    local_keys[usize::from(keygen_i - 1)].clone(),
                )
                .unwrap(),
            );
        }

        simulation.run().unwrap()
    }

    fn simulate_signing(offline: Vec<CompletedOfflineStage>, message: &[u8]) {
        let pk = offline[0].public_key();

        let parties = offline
            .iter()
            .map(|o| SignManual::new(BigInt::from_bytes(message), o.clone()))
            .collect::<Result<Vec<_>, _>>()
            .unwrap();
        let (parties, local_sigs): (Vec<_>, Vec<_>) = parties.into_iter().unzip();
        // parties.remove(0).complete(&local_sigs[1..]).unwrap();
        let local_sigs_except = |i: usize| {
            let mut v = vec![];
            v.extend_from_slice(&local_sigs[..i]);
            if i + 1 < local_sigs.len() {
                v.extend_from_slice(&local_sigs[i + 1..]);
            }
            v
        };

        assert!(parties
            .into_iter()
            .enumerate()
            .map(|(i, p)| p.complete(&local_sigs_except(i)).unwrap())
            .all(|signature| verify(&signature, &pk, &BigInt::from_bytes(message)).is_ok()));
    }
}
