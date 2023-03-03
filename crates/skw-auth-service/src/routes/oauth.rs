use skw_mpc_auth::ownership::oauth::OAuthCredential;
use skw_mpc_auth::{
    ProofOfOwnership, OAuthTokenProofOfOwnershipConfig, OAuthTokenProofOfOwnership,
};

use tide::Request;
use tide::prelude::*;
use serde::Deserialize;

use crate::ServerState;
use crate::env::EnvironmentVar;

// Route /oauth/validate
#[derive(Debug, Clone, Serialize, Deserialize)]
struct OAuthAuthValidateRequest {
    provider: String,
    email: String,

    token: String,
}
type OAuthAuthValidateResponse = String; // serialized OwnershipProof

pub async fn oauth_auth_validate(mut req: Request<ServerState>) -> tide::Result<OAuthAuthValidateResponse> {
    let OAuthAuthValidateRequest { provider, email, token } = req.body_json().await?;
    let env = EnvironmentVar::load();

    let credential = OAuthCredential::new(provider, email);
    // TODO: replace with real secret and signing key
    let config = OAuthTokenProofOfOwnershipConfig::new(env.client_oauth_secret, env.ownership_prover_key);
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
