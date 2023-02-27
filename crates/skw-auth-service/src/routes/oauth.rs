use skw_mpc_auth::ownership::oauth::OAuthCredential;
use skw_mpc_auth::types::Timestamp;
use skw_mpc_auth::{
    GATokenProofOfOwnership, GATokenProofOfOwnershipConfig,
    ProofOfOwnership, GAProof, OAuthTokenProofOfOwnershipConfig, OAuthTokenProofOfOwnership,
};

use tide::Request;
use tide::prelude::*;
use serde::Deserialize;

use crate::ServerState;

// Route /ga/validate
#[derive(Debug, Clone, Serialize, Deserialize)]
struct OAuthAuthValidateRequest {
    provider: String,
    email: String,

    token: String,
}
type OAuthAuthValidateResponse = String; // serialized OwnershipProof

async fn oauth_auth_validate(mut req: Request<ServerState>) -> tide::Result<OAuthAuthValidateResponse> {
    let OAuthAuthValidateRequest { provider, email, token } = req.body_json().await?;

    let credential = OAuthCredential::new(provider, email);
    // TODO: replace with real secret and signing key
    let config = OAuthTokenProofOfOwnershipConfig::new("chokowallet".to_string(), [0u8; 32]);
    let (verifier, credential_hash) = OAuthTokenProofOfOwnership::generate_challenge(&config, &credential)
        .map_err(|e| tide::Error::from_str(500, format!("OAuthProofOfOwnership Error {:?}", e)) )?;

    let certificate = OAuthTokenProofOfOwnership::issue_proof(
        &config, 
        credential_hash.clone(), 
        &token, 
        &verifier
    )
        .map_err(|e| tide::Error::from_str(500, format!("OAuthProofOfOwnership Error {:?}", e)) )?;

    Ok(serde_json::to_string(&certificate).expect("a valid proof of ownership"))
}
