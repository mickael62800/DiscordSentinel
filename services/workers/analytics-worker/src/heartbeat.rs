use tracing::warn;

pub fn start(api_url: String, worker_name: &'static str) {
    tokio::spawn(async move {
        let client = reqwest::Client::new();
        let url = format!("{}/api/bots/heartbeat", api_url);

        loop {
            let _ = client
                .post(&url)
                .json(&serde_json::json!({ "name": worker_name }))
                .send()
                .await
                .map_err(|e| warn!(error = %e, "Heartbeat echoue"));

            tokio::time::sleep(tokio::time::Duration::from_secs(30)).await;
        }
    });
}
