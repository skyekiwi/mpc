use tide::Request;
use crate::ServerState;

// Route: /info/status 
type StatusCheckResponse = String;

pub async fn status_check(_req: Request<ServerState>) -> tide::Result<StatusCheckResponse> {
    Ok("server up".to_string())
}

// pub async fn status_check(_req: Request<ServerState>) -> tide::Result<StatusCheckResponse> {
//     let auth_header = AuthHeader::test_auth_header();
//     let job_header = PayloadHeader::default();
//     let maybe_local_key: Option<Vec<u8>> = Some(vec![1,2,3]);

//     let x = MpcRequestPayload {
//         auth_header, job_header, maybe_local_key
//     };

//     Ok(serde_json::to_string(&x).unwrap())
// }
