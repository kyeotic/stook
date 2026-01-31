use async_trait::async_trait;
use reqwest::Client;
use tracing::{error, info};

#[async_trait]
pub trait WebhookForwarder: Send + Sync {
    async fn forward(&self, webhook_url: &str);
}

pub struct Forwarder {
    client: Client,
}

impl Default for Forwarder {
    fn default() -> Self {
        Self::new()
    }
}

impl Forwarder {
    pub fn new() -> Self {
        Self {
            client: Client::new(),
        }
    }
}

#[async_trait]
impl WebhookForwarder for Forwarder {
    async fn forward(&self, webhook_url: &str) {
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
