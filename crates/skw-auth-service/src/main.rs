use tide::Request;
use tide::prelude::*;
use futures::{channel::{oneshot, mpsc::{Sender}}};
use futures::StreamExt;
use serde::Deserialize;

use skw_mpc_storage::db::{default_mpc_storage_opt, run_db_server, DBOpIn};
use skw_mpc_auth::email::EmailAuth;
use rand::Rng;
use std::time::{SystemTime, UNIX_EPOCH};

use blake2::{Blake2bVar, Digest};
use blake2::digest::{Update, VariableOutput};
use skw_auth_service::email::send_auth_code_to_email;

#[derive(Debug, Deserialize)]
struct EmailAuthRequest {
    email: String,
}

#[derive(Clone)]
struct ServerState {
    storage_in_sender: Sender<DBOpIn>
}

async fn produce_db_request(mut storage_in_sender: Sender<DBOpIn>, op: DBOpIn) {
	let _ = storage_in_sender.try_send(op);
}

async fn get_email_auth_code(mut req: Request<ServerState>) -> tide::Result {
    let EmailAuthRequest { email } = req.body_json().await?;
	let storage_in_sender = req.state().storage_in_sender.clone();
	let random_seed = rand::thread_rng().gen::<[u8; 32]>();
	let auth = EmailAuth::new(
		&email,
		random_seed,
		0
	);

	let auth_code = auth.get_code(None).expect("MPC Auth Error");
	let code = String::from_utf8(auth_code.secret_key.to_vec());
	send_auth_code_to_email(email.as_str(), &auth_code.code).await;

	let s = auth_code.secret_key.to_vec();
	let mut hasher = Blake2bVar::new(32).unwrap();
	hasher.update(email.as_bytes());

	let mut key = [0u8; 32];
	hasher.finalize_variable(&mut key).unwrap();

	let (i, o) = oneshot::channel();
	let op = DBOpIn::WriteToDB {
		key: key,
		value: serde_json::to_string(&auth_code).unwrap().as_bytes().to_vec(),
		result_sender: i
	};
	produce_db_request(storage_in_sender, op).await;
	let res = o.await;
    Ok(format!("My email address is {}", email).into())
}

async fn shutdown_db(mut storage_in_sender: Sender<DBOpIn>) {
	let (i, o) = oneshot::channel();
	storage_in_sender.try_send(DBOpIn::Shutdown {
		result_sender: i
	});
	let res = o.await;
}

#[async_std::main]
async fn main() {
	let (storage_config, storage_in_sender) = default_mpc_storage_opt(
        format!("email-auth-code-storage"), false
    );
	run_db_server(storage_config);

	let state = ServerState {
		storage_in_sender: storage_in_sender.clone()
	};

	let mut app = tide::with_state(state);
    app.at("/email/auth/code").post(get_email_auth_code);
    app.listen("127.0.0.1:8080").await;

	shutdown_db(storage_in_sender.clone()).await;
}
