use skw_mpc_auth::{
    Ed25519ProverConfig, Ed25519SelfProveableSystem, SelfProveableSystem, MpcUsageCertification, UsageCertification,
};

use tide::Request;
use tide::prelude::*;
use serde::Deserialize;

use crate::ServerState;

// Route: /usage/link 
#[derive(Debug, Clone, Serialize, Deserialize)]
struct UsageLinkRequest {
    keygen_id: String,
    ownership_proof: String,
}
type UsageLinkResponse = String;

pub async fn usage_link(mut req: Request<ServerState>) -> tide::Result<UsageLinkResponse> {
    let UsageLinkRequest { keygen_id, ownership_proof } = req.body_json().await?;
    let keygen_id: [u8; 32] = hex::decode(&keygen_id)
        .map_err(|e| tide::Error::from_str(500, format!("LinkUsage Error {:?}", e)) )?
        .try_into()
        .map_err(|_| tide::Error::from_str(500, format!("LinkUsage Error keygen_id length error")) )?;
    let ownership_proof = serde_json::from_str(&ownership_proof)
        .map_err(|_| tide::Error::from_str(500, format!("LinkUsage Error unable to parse ownership_proof")) )?;

    // 1. make all keys align
    let ownership_prover_default_config: Ed25519ProverConfig = [0u8; 32].into();
    let ownership_verifier_default_config  = Ed25519SelfProveableSystem::derive_verifier_config(&ownership_prover_default_config)
        .map_err(|e| tide::Error::from_str(500, format!("LinkUsage Error {:?}", e)) )?;

    let usage_prover_default_config: Ed25519ProverConfig = [1u8; 32].into();

    // 2. generate a certificate
    let certification = MpcUsageCertification::issue_usage_certification(
        &keygen_id, 
        &ownership_verifier_default_config, 
        &usage_prover_default_config, 
        &ownership_proof
    )
        .map_err(|e| tide::Error::from_str(500, format!("LinkUsage Error {:?}", e)) )?;

    Ok(serde_json::to_string(&certification).expect("certification should be valid for serialization"))
}


// NOTE: this will not be called unless for testing purpose
// The proof can be self validated
// Route /usage/validate
#[derive(Debug, Clone, Serialize, Deserialize)]
struct UsageValidateRequest {
    keygen_id: String,
    credential_hash: String,
    usage_certification: String,
}
type UsageValidateResponse = String; // dummy "ok"

pub async fn usage_validate(mut req: Request<ServerState>) -> tide::Result<UsageValidateResponse> {
    let UsageValidateRequest { keygen_id, credential_hash, usage_certification} = req.body_json().await?;
    let keygen_id: [u8; 32] = hex::decode(&keygen_id)
        .map_err(|e| tide::Error::from_str(500, format!("LinkValidate Error {:?}", e)) )?
        .try_into()
        .map_err(|_| tide::Error::from_str(500, format!("LinkValidate Error keygen_id length error")) )?;
    let credential_hash: [u8; 32] = hex::decode(&credential_hash)
        .map_err(|e| tide::Error::from_str(500, format!("LinkValidate Error {:?}", e)) )?
        .try_into()
        .map_err(|_| tide::Error::from_str(500, format!("LinkValidate Error credential_hash length error")) )?;
    let usage_certification = serde_json::from_str(&usage_certification)
        .map_err(|_| tide::Error::from_str(500, format!("LinkValidate Error unable to parse usage_certification")) )?;

    // 1. make all keys align    
    let usage_prover_default_config: Ed25519ProverConfig = [1u8; 32].into();
    let usage_verifier_default_config  = Ed25519SelfProveableSystem::derive_verifier_config(&usage_prover_default_config)
        .map_err(|e| tide::Error::from_str(500, format!("LinkValidate Error {:?}", e)) )?;

    // 2. generate a certificate
    MpcUsageCertification::verify_usage_certification(
        &keygen_id, 
        &credential_hash, 
        &usage_verifier_default_config, 
        &usage_certification
    )
        .map_err(|e| tide::Error::from_str(500, format!("LinkValidate Error {:?}", e)) )?;

    Ok("ok".to_string())
}

