use async_trait::async_trait;
use reqwest::Client;
use serde::Deserialize;
use std::fmt;
use tracing::info;

#[derive(Debug)]
pub enum RedeployError {
    StackNotFound(String),
    Api(String),
    Network(reqwest::Error),
}

impl fmt::Display for RedeployError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::StackNotFound(name) => write!(f, "stack not found: {name}"),
            Self::Api(msg) => write!(f, "Portainer API error: {msg}"),
            Self::Network(e) => write!(f, "network error: {e}"),
        }
    }
}

impl From<reqwest::Error> for RedeployError {
    fn from(e: reqwest::Error) -> Self {
        Self::Network(e)
    }
}

#[async_trait]
pub trait StackRedeployer: Send + Sync {
    async fn redeploy(&self, stack_name: &str) -> Result<(), RedeployError>;
}

#[derive(Deserialize)]
#[serde(rename_all = "PascalCase")]
struct PortainerStack {
    id: i64,
    name: String,
    endpoint_id: i64,
    env: Vec<serde_json::Value>,
}

#[derive(Deserialize)]
#[serde(rename_all = "PascalCase")]
struct StackFileResponse {
    stack_file_content: String,
}

pub struct Redeployer {
    client: Client,
    portainer_url: String,
    api_key: String,
}

impl Redeployer {
    pub fn new(portainer_url: String, api_key: String) -> Self {
        Self {
            client: Client::builder()
                .danger_accept_invalid_certs(true)
                .build()
                .expect("failed to build HTTP client"),
            portainer_url: portainer_url.trim_end_matches('/').to_string(),
            api_key,
        }
    }
}

#[async_trait]
impl StackRedeployer for Redeployer {
    async fn redeploy(&self, stack_name: &str) -> Result<(), RedeployError> {
        info!(stack = %stack_name, "redeploying stack via Portainer API");

        // Step 1: Find the stack
        let stacks: Vec<PortainerStack> = self
            .client
            .get(format!("{}/api/stacks", self.portainer_url))
            .header("X-API-Key", &self.api_key)
            .send()
            .await?
            .error_for_status()
            .map_err(|e| RedeployError::Api(e.to_string()))?
            .json()
            .await?;

        let stack = stacks
            .iter()
            .find(|s| s.name == stack_name)
            .ok_or_else(|| RedeployError::StackNotFound(stack_name.to_string()))?;

        let stack_id = stack.id;
        let endpoint_id = stack.endpoint_id;
        let env = stack.env.clone();

        // Step 2: Get current stack file
        let file_resp: StackFileResponse = self
            .client
            .get(format!("{}/api/stacks/{stack_id}/file", self.portainer_url))
            .header("X-API-Key", &self.api_key)
            .send()
            .await?
            .error_for_status()
            .map_err(|e| RedeployError::Api(e.to_string()))?
            .json()
            .await?;

        // Step 3: Redeploy
        let body = serde_json::json!({
            "env": env,
            "pullImage": true,
            "prune": true,
            "stackFileContent": file_resp.stack_file_content,
        });

        let resp = self
            .client
            .put(format!(
                "{}/api/stacks/{stack_id}?endpointId={endpoint_id}",
                self.portainer_url
            ))
            .header("X-API-Key", &self.api_key)
            .json(&body)
            .send()
            .await?
            .error_for_status()
            .map_err(|e| RedeployError::Api(e.to_string()))?;

        info!(stack = %stack_name, status = %resp.status(), "stack redeployed");
        Ok(())
    }
}
