use skw_mpc_payload::{AuthHeader, PayloadHeader};
use tide::Request;
use serde::{Deserialize, Serialize};

use crate::ServerState;

#[derive(Serialize, Deserialize)]
pub struct MpcRequestPayload {
    pub auth_header: String, 
    pub job_header: String,
    pub maybe_local_key: String,
}

pub async fn mpc_submit(mut req: Request<ServerState>) -> Result<String, tide::Error> {
    let MpcRequestPayload { auth_header, job_header, maybe_local_key } = req.body_json().await?;

    let auth_header: AuthHeader = serde_json::from_str(&auth_header)
        .map_err(|e| tide::Error::from_str(500, format!("MPC Error {:?}", e)) )?;
    let job_header: PayloadHeader = serde_json::from_str(&job_header)
        .map_err(|e| tide::Error::from_str(500, format!("MPC Error {:?}", e)) )?;
    let maybe_local_key: Option<Vec<u8>> = serde_json::from_str(&maybe_local_key)
        .map_err(|e| tide::Error::from_str(500, format!("MPC Error {:?}", e)) )?;

    log::info!("{:?}", maybe_local_key);
    let mut server_state = req.state().clone(); // Cost of clone is pretty low here ... but there might be a better way
    let result = server_state.light_node
        .send_request(job_header, auth_header, maybe_local_key)
        .await
        .map_err(|e| tide::Error::from_str(500, format!("MPC Error {:?}", e)) )?;
    
    Ok(serde_json::to_string(&result).unwrap())
}
