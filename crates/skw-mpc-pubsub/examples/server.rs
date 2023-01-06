// use std::collections::hash_map::{Entry, HashMap};
// use tokio::sync::{RwLock};
// use skw_mpc_pubsub::node::MpcPubsub;

// use rocket::http::Status;
// use rocket::State;
// use skw_mpc_protocol::gg20::state_machine::keygen::Keygen;
// use serde::{Deserialize, Serialize};

// use libp2p::gossipsub::{ IdentTopic as Topic, TopicHash };


// #[derive(Serialize, Deserialize, Debug)]
// struct KeygenRequest {
//     unique_idx: u16,
// }

// fn keygen_handler(topic: TopicHash, msg: &str) {
// 	()
// }

// #[rocket::get("/create_keygen_channel/<task_id>")]
// async fn create_keygen_channel(
// 	db: &State<Db>,
// 	task_id: &str
// ) -> Status {

// 	let gossipsub = &db.node.swarm.behaviour().gossipsub;
// 	// for topic in db.node.swarm.behaviour().gossipsub.topics() {
// 	// 	print!("{}", topic)
// 	// }
// 	let topic = Topic::new(task_id);
// 	gossipsub.subscribe(&topic);

// 	// let create_keygen = |msg: &str| async {
// 	// 	let mut output_file = tokio::fs::OpenOptions::new()
// 	// 		.write(true)
// 	// 		.create_new(true)
// 	// 		.open(task_id)
// 	// 		.await
// 	// 		.unwrap();
// 	// 	let keygen_data: Vec<KeygenRequest> = serde_json::from_str(msg).unwrap();
// 	// 	print!("{}", keygen_data.len());
// 	// 	Ok(())
// 	// 	// let keygen = Keygen::new(keygen_data, args.threshold, args.number_of_parties)?;
// 	// };
// 	Status::Ok
// }

// struct Db<'a> {
// 	node: &'a MpcPubsub,
// 	rooms: RwLock<HashMap<String, String>>
// }
// impl<'a> Db<'a> {
// 	pub fn empty(_node: &MpcPubsub) -> Self {
// 		Self {
// 			node: _node,
// 			rooms: RwLock::new(HashMap::new())
// 		}
// 	}
// }


// #[tokio::main]
// async fn main() -> Result<(), Box<dyn std::error::Error>> {
// 	let mut node = match MpcPubsub::new().await {
// 		Ok(n) => n,
// 		Err(e) => return Status::InternalServerError,
// 	};
// 	node.start(keygen_handler).await;

//     rocket::build()
//         .mount("/", rocket::routes![create_keygen_channel])
//         .manage(Db::empty(&node))
//         .launch()
//         .await?;
//     Ok(())
// }


fn main() {
	println!("placeholder");
}