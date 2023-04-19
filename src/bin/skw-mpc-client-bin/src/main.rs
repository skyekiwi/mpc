use std::{fs, io::Write};

use futures::{channel::mpsc, StreamExt};
use skw_mpc_client_bin::{
	ServerState, 
	routes::misc::status_check,
	routes::mpc::mpc_submit,
};

use skw_mpc_light_node::{light_node_event_loop, client::NodeClient};
use skw_mpc_node::async_executor;
use tide::{utils::{After}, Response, StatusCode, http::headers::HeaderValue};
use tide::security::{CorsMiddleware, Origin};

#[tokio::main]
async fn main() {

	// --- Initialize environmental variables and settings ---
	dotenv::dotenv().ok();
	env_logger::init();

	// --- Start A Light Node ---
	let (client_request_sender, client_request_receiver) = mpsc::channel(0);
    async_executor(light_node_event_loop(client_request_receiver));
    let mut light_node_client = NodeClient::new(client_request_sender);

    let mut light_client_node_res = light_node_client
        .bootstrap_node(
            Some([3u8; 32]), 
            "/ip4/127.0.0.1/tcp/2622/ws".to_string(),
            "mpc-storage-db-12D3KooWK99VoVxNE7XzyBwXEzW7xhK7Gpv85r9F3V3fyKSUKPH5".to_string()
        ).await;
	let peer_id = light_node_client.peer_id();

	let env_file_node1 = format!("LIGHT_NODE_ID = {}\n", peer_id.to_string());

	let mut file = fs::OpenOptions::new()
		.append(true)
		.open("./.env.peers")
		.expect("able to open a file");

	file.write_all(env_file_node1.as_bytes()).expect("able to write");
	
	log::info!("Peer Id written to .env.peers");
	
    async_executor(async move {
        loop {
            let res = light_client_node_res.select_next_some().await;
			log::error!("Node encounter error {:?}", res);
        }
    });

	// --- Start web server ---
	let state = ServerState::new(light_node_client);
	let mut app = tide::with_state(state);

	app.with(
		CorsMiddleware::new()
		.allow_methods("GET, POST, OPTIONS".parse::<HeaderValue>().unwrap())
		.allow_origin(Origin::from("*"))
		.allow_credentials(false)
	);
	app.with(After(|mut res: Response | async {
		if let Some(err) = res.error() {
			let msg = format!("Error: {:?}", err);
			log::error!("Req Error {msg}");
			res.set_status(StatusCode::Ok);
			res.set_body(msg);
		}

		Ok(res)
	}));

	app.at("/info/status").get(status_check);
	app.at("/mpc/submit").post(mpc_submit);
	// app.at("/usage/link").post(usage_link);
	// app.at("/usage/validate").post(usage_validate);

	log::info!("Start listening web server...");
    let _ = app.listen("0.0.0.0:2619").await;


	// --- Gracefully close the web server ---
	log::info!("Web server closed.");
	// Shutdown level db
	// shutdown_db(storage_in_sender).await.expect("db should be able to close successfully");
	log::info!("Level DB server closed.");

}
