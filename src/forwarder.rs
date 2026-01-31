use reqwest::Client;
use tracing::{error, info};

pub struct Forwarder {
    client: Client,
}

impl Forwarder {
    pub fn new() -> Self {
        Self {
            client: Client::new(),
        }
    }

    pub async fn forward(&self, webhook_url: &str) {
        info!(url = %webhook_url, "forwarding webhook to Portainer");
        match self.client.post(webhook_url).send().await {
            Ok(resp) => {
                info!(status = %resp.status(), url = %webhook_url, "forwarded webhook");
            }
            Err(e) => {
                error!(error = %e, url = %webhook_url, "failed to forward webhook");
            }
        }
    }
}
