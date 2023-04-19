use skw_mpc_auth::types::Timestamp;
use skw_mpc_auth::{
    GATokenProofOfOwnership, GATokenProofOfOwnershipConfig,
    ProofOfOwnership, GAProof,
};

use tide::Request;
use tide::prelude::*;
use serde::Deserialize;

use crate::ServerState;
use crate::env::EnvironmentVar;

// Route: /ga/init
#[derive(Debug, Clone, Serialize, Deserialize)]
struct GAAuthInitRequest ();

type GAAuthInitResponse = String;

pub async fn ga_auth_init(req: Request<ServerState>) -> tide::Result<GAAuthInitResponse> {
    // let EmailAuthInitRequest { email } = req.body_json().await?;
    let mut server_state = req.state().clone(); // Cost of clone is pretty low here ... but there might be a better way
    let env = EnvironmentVar::load();

    // 1. Generate & store a verifier
    let config = GATokenProofOfOwnershipConfig::new(30, env.ownership_prover_key); // default GA timeout is 30 seconds

    // generate a random material base
    let random_material: [u8; 32] = rand::random();
    let verifier = GATokenProofOfOwnership::generate_challenge(&config, &random_material)
        .map_err(|e| tide::Error::from_str(500, format!("GAProofOfOwnership Error {:?}", e)) )?;

    let credential_hash = GATokenProofOfOwnership::get_credential_hash(&config, &random_material)
        .map_err(|e| tide::Error::from_str(500, format!("GAProofOfOwnership Error {:?}", e)) )?;

    server_state
        .write_to_db(
            credential_hash,
            serde_json::to_vec(&verifier).expect("verifier should be able to serialize to json")
        ).await
        .map_err(|e| tide::Error::from_str(500, format!("GAProofOfOwnership Error {:?}", e)) )?;

    let ga_token_str = verifier.try_to_string()
        .map_err(|e| tide::Error::from_str(500, format!("GAProofOfOwnership Error {:?}", e)) )?;

    Ok(ga_token_str)
}

// Route /ga/validate
#[derive(Debug, Clone, Serialize, Deserialize)]
struct GAAuthValidateRequest {
    ga_hash: String, // hex encoded without leading 0x
    code: String, // hex encoded without leading 0x
    time: Timestamp, // u64, time when the user init the request
}
type GAAuthValidateResponse = String; // serialized OwnershipProof

pub async fn ga_auth_validate(mut req: Request<ServerState>) -> tide::Result<GAAuthValidateResponse> {
    let GAAuthValidateRequest { ga_hash, code, time } = req.body_json().await?;
    let env = EnvironmentVar::load();

    let ga_hash: [u8; 32] = hex::decode(&ga_hash)
        .map_err(|e| tide::Error::from_str(500, format!("GAProofOfOwnership Error {:?}", e)) )?
        .try_into()
        .map_err(|_| tide::Error::from_str(500, format!("GAProofOfOwnership Error email_hash length error")) )?;
    let code: [u8; 6] = hex::decode(&code)
        .map_err(|e| tide::Error::from_str(500, format!("GAProofOfOwnership Error {:?}", e)) )?
        .try_into()
        .map_err(|_| tide::Error::from_str(500, format!("GAProofOfOwnership Error email_hash length error")) )?;

    let mut server_state = req.state().clone(); // Cost of clone is pretty low here ... but there might be a better way

    // 1. fetch over th verifier from DB
    let verifier_bytes = server_state.read_from_db(ga_hash.clone())
        .await
        .map_err(|e| tide::Error::from_str(500, format!("GAProofOfOwnership Error {:?}", e)) )?;
    let verifier = serde_json::from_slice(&verifier_bytes)
        .map_err(|e| tide::Error::from_str(500, format!("GAProofOfOwnership Error {:?}", e)) )?;

    let config = GATokenProofOfOwnershipConfig::new(30, env.ownership_prover_key); // default GA timeout is 30 seconds

    let certificate = GATokenProofOfOwnership::issue_proof(
        &config,
        ga_hash,
        &GAProof::new(code, time),
        &verifier
    )
        .map_err(|e| tide::Error::from_str(500, format!("EmailAuthValidate Error {:?}", e)) )?;


    Ok(serde_json::to_string(&certificate).expect("a valid proof of ownership"))
}
