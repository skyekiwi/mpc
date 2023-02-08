use tide::Request;
use tide::prelude::*;
use futures::channel::mpsc;
use futures::StreamExt;
use serde::Deserialize;

use skw_mpc_storage::db::MpcStorage;
use skw_mpc_auth::email::EmailAuth;
use rand::Rng;
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Deserialize)]
struct EmailAuthRequest {
    email: String,
}

#[derive(Clone)]
struct ServerState {
    db_sender: mpsc::Sender<DBMutation>
}
#[derive(Clone)]
struct DBMutation {
	key: Vec<u8>,
	value: Vec<u8>,
}

async fn produce_db_request<'a >(mut db_req_sender: mpsc::Sender<DBMutation>, mutation: &'a DBMutation) {
	println!("{:?}", mutation.clone().value);
	let _ = db_req_sender.try_send(mutation.clone());
}

async fn get_email_auth_code(mut req: Request<ServerState>) -> tide::Result {
    let EmailAuthRequest { email } = req.body_json().await?;
	let db_sender = req.state().db_sender.clone();
	let random_seed = rand::thread_rng().gen::<[u8; 32]>();
	// let now = SystemTime::now();
	// let since_the_epoch = now.duration_since(UNIX_EPOCH).expect("Time went backwards");
	let auth = EmailAuth::new(
		&email,
		random_seed,
		0
	);

	let auth_code = auth.get_code(None).expect("MPC Auth Error");
	let code = String::from_utf8(auth_code.secret_key.to_vec());
	let mutation = DBMutation {
		key: (&auth_code.code).iter().cloned().collect(),
		value: serde_json::to_string(&auth_code).unwrap().as_bytes().to_vec()
	};
	produce_db_request(db_sender, &mutation).await;
    Ok(format!("My email address is {}", email).into())
}

#[async_std::main]
async fn main() {
	let (db_sender, mut db_receiver) = mpsc::channel(0);
	let state = ServerState {
		db_sender: db_sender
	};

	let mut storage = MpcStorage::new("email-auth-code-storage", false).expect("Storage not created!");

	async_std::task::spawn(async move {
		loop {
			futures::select! {
				req = db_receiver.select_next_some() => {
					storage.put(&req.key, &req.value);
					println!("{:?}", String::from_utf8(req.value).unwrap());
					println!("{:?}", String::from_utf8(req.key).unwrap());
				}
			}
		}
	});

	let mut app = tide::with_state(state);
    app.at("/email/auth/code").post(get_email_auth_code);
    app.listen("127.0.0.1:8080").await;
}
