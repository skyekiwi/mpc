use std::env;
use serde_json::json;

use skw_mpc_auth::types::CODE_LEN;

pub async fn send_auth_code_to_email(receiver_email: &str, auth_code: &[u8; CODE_LEN]) {
	let sendgrid_api_key = env::var("SENDGRID_API_KEY").unwrap();
	let sender_email = env::var("SENDER_EMAIL").unwrap();
	let sendgrid_body = json!({
		"personalizations": [
			{
				"to": [
					{"email": receiver_email}
				]
			}
		],
		"from": {
			"email": sender_email.as_str()
		},
		"subject": "Verification Code",
		"content": [
			{
				"type": "text/plain",
				"value": "and easy to do anywhere, even with cURL"
			}
		]
	});
	let authorization_header = format!("Bearer {}", sendgrid_api_key);
	let response = reqwest::Client::new()
        .post("https://api.sendgrid.com/v3/mail/send")
		.header("Authorization", authorization_header)
		.header("Content-Type", "application/json")
        .json(&sendgrid_body)
        .send().await;
	println!("{:?}" , response);
	// Ok(());
}
