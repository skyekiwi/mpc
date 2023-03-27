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
    let mut server_state = req.state().clone(); // Cost of clone is pretty low here ... but there might be a better way

    let env = EnvironmentVar::load();
    let credential = OAuthCredential::new(provider, email);
    let config = OAuthTokenProofOfOwnershipConfig::new(env.client_oauth_secret, env.ownership_prover_key);
    let credential_hash = OAuthTokenProofOfOwnership::get_credential_hash(&config, &credential)
        .map_err(|e| tide::Error::from_str(500, format!("OAuthProofOfOwnership Error {:?}", e)) )?;

    // store the preimage
    // if the preimage exists - overwriting is ok - values are the same
    server_state
        .write_to_db(credential_hash.clone(), serde_json::to_vec(&credential)
            .map_err(|e| tide::Error::from_str(500, format!("OAuthProofOfOwnership Error {:?}", e)) )?
        )
        .await
        .map_err(|e| tide::Error::from_str(500, format!("OAuthProofOfOwnership Error {:?}", e)) )?;
    
    log::info!("Write to DB {:?}", credential_hash.clone());

    let verifier = OAuthTokenProofOfOwnership::generate_challenge(&config, &credential)
        .map_err(|e| tide::Error::from_str(500, format!("OAuthProofOfOwnership Error {:?}", e)) )?;

    let certificate = OAuthTokenProofOfOwnership::issue_proof(
        &config, 
        &credential, 
        &token, 
        &verifier
    )
        .map_err(|e| tide::Error::from_str(500, format!("OAuthProofOfOwnership Error {:?}", e)) )?;

    Ok(serde_json::to_string(&certificate).expect("a valid proof of ownership"))
}


// Route /oauth/preimage
#[derive(Debug, Clone, Serialize, Deserialize)]
struct OAuthAuthPreimageRequest {
    provider: String,
    email: String,
}
type OAuthAuthPreimageResponse = String; // serialized OwnershipProof

pub async fn oauth_auth_preimage(mut req: Request<ServerState>) -> tide::Result<OAuthAuthPreimageResponse> {
    let OAuthAuthPreimageRequest { provider, email } = req.body_json().await?;
    let mut server_state = req.state().clone(); // Cost of clone is pretty low here ... but there might be a better way

    let env = EnvironmentVar::load();

    let credential = OAuthCredential::new(provider, email);
    let config = OAuthTokenProofOfOwnershipConfig::new(env.client_oauth_secret, env.ownership_prover_key);
    let credential_hash = OAuthTokenProofOfOwnership::get_credential_hash(&config, &credential)
        .map_err(|e| tide::Error::from_str(500, format!("OAuthProofOfOwnership Error {:?}", e)) )?;

    // store the preimage
    // if the preimage exists - overwriting is ok - values are the same
    let preimage = server_state
        .read_from_db(credential_hash.clone())
        .await;

    match preimage {
        Ok(_) => Ok("preimage_in_db".to_string()),
        Err(_) => Ok("not_in_db".to_string())
    }
}
