use axum::extract::State;
use axum::http::StatusCode;
use axum::Json;
use std::sync::Arc;
use tracing::{debug, error, warn};

use crate::discovery::WebhookLookup;
use crate::redeployer::StackRedeployer;
use crate::registry::RegistryNotification;

pub struct AppState {
    pub discovery: Arc<dyn WebhookLookup>,
    pub redeployer: Arc<dyn StackRedeployer>,
}

pub async fn health() -> StatusCode {
    StatusCode::OK
}

pub async fn webhook(
    State(state): State<Arc<AppState>>,
    Json(notification): Json<RegistryNotification>,
) -> StatusCode {
    let repos = notification.push_repositories();
    debug!(count = repos.len(), "received push events");

    for repo in repos {
        match state.discovery.lookup(repo).await {
            Some(stack_name) => {
                if let Err(e) = state.redeployer.redeploy(&stack_name).await {
                    error!(stack = %stack_name, error = %e, "failed to redeploy stack");
                }
            }
            None => {
                warn!(repository = %repo, "no stack route found, ignoring");
            }
        }
    }

    StatusCode::OK
}
