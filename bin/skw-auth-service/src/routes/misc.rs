use tide::Request;
use crate::env::PeerIds;
use crate::ServerState;

// Route: /usage/link 
type PeerIdsResponse = String;

pub async fn peer_ids(_req: Request<ServerState>) -> tide::Result<PeerIdsResponse> {
    let env = PeerIds::load();

    Ok(serde_json::to_string(&env).expect("peerIds should be valid"))
}
