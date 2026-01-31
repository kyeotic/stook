use axum::extract::State;
use axum::http::StatusCode;
use axum::Json;
use std::sync::Arc;
use tracing::{debug, warn};

use crate::discovery::WebhookLookup;
use crate::forwarder::WebhookForwarder;
use crate::registry::RegistryNotification;

pub struct AppState {
    pub discovery: Arc<dyn WebhookLookup>,
    pub forwarder: Arc<dyn WebhookForwarder>,
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
            Some(url) => {
                state.forwarder.forward(&url).await;
            }
            None => {
                warn!(repository = %repo, "no webhook route found, ignoring");
            }
        }
    }

    StatusCode::OK
}
