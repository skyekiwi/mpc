pub mod routes;
use skw_mpc_light_node::client::NodeClient;

#[derive(Clone)]
pub struct ServerState {
    light_node: NodeClient
}

impl ServerState {
    pub fn new(light_node: NodeClient) -> Self {
        Self { light_node }
    }
}