fn send_auth_code_to_email(email: &str, auth_code: &[u8; _]) {
	sendgrid_body = json!({
		"personalizations": [
			{
				"to": [
					{"email": email.as_str()}
				]
			}
		],
		"from": {
			"email": "test@choko.app"
		},
		"subject": "Verification Code",
		"content": [
			{
				"type": "text/plain",
				"value": "and easy to do anywhere, even with cURL"
			}
		]
	});
	let response = reqwest::Client::new()
        .post("https://api.sendgrid.com/v3/mail/send")
		.header("Authorization", "Bearer ")
		.header("Content-Type", "application/json")
        .json(&sendgrid_body)
        .send().await?;
}
