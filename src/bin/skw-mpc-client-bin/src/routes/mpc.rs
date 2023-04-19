use skw_mpc_payload::{AuthHeader, PayloadHeader};
use tide::Request;
use serde::{Deserialize, Serialize};

use crate::ServerState;

#[derive(Serialize, Deserialize)]
pub struct MpcRequestPayload {
    pub auth_header: AuthHeader, 
    pub job_header: PayloadHeader,
    pub maybe_local_key: Option<Vec<u8>>,
}

pub async fn mpc_submit(mut req: Request<ServerState>) -> Result<String, tide::Error> {
    let MpcRequestPayload { auth_header, job_header, maybe_local_key } = req.body_json().await?;

    log::info!("{:?}", maybe_local_key);
    let mut server_state = req.state().clone(); // Cost of clone is pretty low here ... but there might be a better way
    let result = server_state.light_node
        .send_request(job_header, auth_header, maybe_local_key)
        .await
        .map_err(|e| tide::Error::from_str(500, format!("MPC Error {:?}", e)) )?;
    
    Ok(serde_json::to_string(&result).unwrap())
}
