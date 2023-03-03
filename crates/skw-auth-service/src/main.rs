use skw_auth_service::{
	ServerState,
	routes::email::{email_auth_init, email_auth_validate},
	routes::ga::{ga_auth_init, ga_auth_validate},
	routes::oauth::oauth_auth_validate,
	routes::usage::{usage_link, usage_validate}, shutdown_db
};
use skw_mpc_storage::{run_db_server, default_mpc_storage_opt};
use tide::{utils::{After}, Response, StatusCode};

#[async_std::main]
async fn main() {

	// --- Initialize environmental variables and settings ---
	dotenv::dotenv().ok();
	env_logger::init();

	// --- Run level DB server ---
	let (storage_config, storage_in_sender) = default_mpc_storage_opt(
        format!("email-auth-code-storage"), false
    );
	run_db_server(storage_config);
	log::info!("Level DB server started.");


	// --- Start web server ---
	let state = ServerState::new(&storage_in_sender);
	let mut app = tide::with_state(state);

	app.with(After(|mut res: Response | async {
		if let Some(err) = res.error() {
			let msg = format!("Error: {:?}", err);
			res.set_status(StatusCode::InternalServerError);
			res.set_body(msg);
		}

		Ok(res)
	}));

    app.at("/auth/email/init").post(email_auth_init);
	app.at("/auth/email/validate").post(email_auth_validate);

	app.at("/auth/ga/init").post(ga_auth_init);
	app.at("/auth/ga/validate").post(ga_auth_validate);

	app.at("/auth/oauth/validate").post(oauth_auth_validate);

	app.at("/usage/link").post(usage_link);
	app.at("/usage/validate").post(usage_validate);

	log::info!("Start listening web server...");
    let _ = app.listen("0.0.0.0:8080").await;


	// --- Gracefully close the web server ---
	log::info!("Web server closed.");
	// Shutdown level db
	shutdown_db(storage_in_sender).await.expect("db should be able to close successfully");
	log::info!("Level DB server closed.");

}
