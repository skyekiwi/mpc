use skw_mpc_auth::GAProof;
use skw_mpc_auth::{
    GAProofSystem,
    EmailProofOfOwnership, EmailProofOfOwnershipConfig,

    ProofOfOwnership, ProofSystem,
};

use tide::Request;
use tide::prelude::*;
use serde::Deserialize;

use crate::ServerState;
use crate::util::send_auth_code_to_email;

// Route: /email/init 
#[derive(Debug, Clone, Serialize, Deserialize)]
struct EmailAuthInitRequest {
    email: String,
}

type EmailAuthInitResponse = String; // Dummy "ok"

pub async fn email_auth_init(mut req: Request<ServerState>) -> tide::Result<EmailAuthInitResponse> {
    let EmailAuthInitRequest { email } = req.body_json().await?;
    let mut server_state = req.state().clone(); // Cost of clone is pretty low here ... but there might be a better way

    
    // 1. Generate & store a verifier
    // TODO: replace with real secret key
    // Default email auth timeout is 10mins
    let config = EmailProofOfOwnershipConfig::new(600, [0u8; 32]);
    let (verifier, credential_hash) = EmailProofOfOwnership::generate_challenge(&config, &email)
        .map_err(|e| tide::Error::from_str(500, format!("EmailProofOfOwnership Error {:?}", e)) )?;

    server_state
        .write_to_db(
            credential_hash.clone(), 
            serde_json::to_vec(&verifier).expect("verifier should be able to serialize to json")
        ).await
        .map_err(|e| tide::Error::from_str(500, format!("EMailProofOfOwnership Error {:?}", e)) )?;

    // 2. generate a proof & send via email to user
    let proof = GAProofSystem::generate_proof(&verifier, &0)
        .map_err(|e| tide::Error::from_str(500, format!("EMailProofOfOwnership Error {:?}", e)) )?;

    send_auth_code_to_email(&email, &proof.code()).await;

    // Ok("ok".to_string())
    Ok(serde_json::to_string(&proof).unwrap())
}

// Route /email/validate
#[derive(Debug, Clone, Serialize, Deserialize)]
struct EmailAuthValidateRequest {
    email_hash: String, // hex encoded without leading 0x
    code: String, // hex encoded without leading 0x
}
type EmailAuthValidateResponse = String; // serialized OwnershipProof

pub async fn email_auth_validate(mut req: Request<ServerState>) -> tide::Result<EmailAuthValidateResponse> {
    let EmailAuthValidateRequest { email_hash, code } = req.body_json().await?;
    let email_hash: [u8; 32] = hex::decode(&email_hash)
        .map_err(|e| tide::Error::from_str(500, format!("EmailAuthValidate Error {:?}", e)) )?
        .try_into()
        .map_err(|_| tide::Error::from_str(500, format!("EmailAuthValidate Error email_hash length error")) )?;
    let code: [u8; 6] = hex::decode(&code)
        .map_err(|e| tide::Error::from_str(500, format!("EmailAuthValidate Error {:?}", e)) )?
        .try_into()
        .map_err(|_| tide::Error::from_str(500, format!("EmailAuthValidate Error email_hash length error")) )?;

    let mut server_state = req.state().clone(); // Cost of clone is pretty low here ... but there might be a better way

    // 1. fetch over th verifier from DB
    let verifier_bytes = server_state.read_from_db(email_hash.clone())
        .await
        .map_err(|e| tide::Error::from_str(500, format!("EmailAuthValidate Error {:?}", e)) )?;
    let verifier = serde_json::from_slice(&verifier_bytes)
        .map_err(|e| tide::Error::from_str(500, format!("EmailAuthValidate Error {:?}", e)) )?;

    // TODO: replace with real secret key
    let config = EmailProofOfOwnershipConfig::new(600, [0u8; 32]);
    
    let certificate = EmailProofOfOwnership::issue_proof(
        &config, 
        email_hash, 
        &GAProof::new(code, 0),
        &verifier
    )
        .map_err(|e| tide::Error::from_str(500, format!("EmailAuthValidate Error {:?}", e)) )?;


    Ok(serde_json::to_string(&certificate).expect("a valid proof of ownership"))
}
