mod discovery;
mod redeployer;
mod registry;
mod routes;

use axum::routing::{get, post};
use axum::Router;
use std::sync::Arc;
use tokio::net::TcpListener;
use tracing::info;

use discovery::Discovery;
use redeployer::Redeployer;
use routes::AppState;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_env("LOG_LEVEL")
                .unwrap_or_else(|_| "info".into()),
        )
        .init();

    let port: u16 = std::env::var("LISTEN_PORT")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(3000);

    let cache_ttl: u64 = std::env::var("CACHE_TTL_SECS")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(60);

    let portainer_url = std::env::var("PORTAINER_URL")
        .unwrap_or_else(|_| "https://localhost:9443".to_string());

    let api_key = std::env::var("PORTAINER_API_KEY")
        .expect("PORTAINER_API_KEY environment variable is required");

    let discovery = Discovery::new(cache_ttl).expect("failed to connect to Docker");
    let redeployer = Arc::new(Redeployer::new(portainer_url, api_key));

    let state = Arc::new(AppState {
        discovery,
        redeployer,
    });

    let app = Router::new()
        .route("/webhook", post(routes::webhook))
        .route("/health", get(routes::health))
        .with_state(state);

    let addr = format!("0.0.0.0:{port}");
    info!(addr = %addr, "starting server");
    let listener = TcpListener::bind(&addr).await.expect("failed to bind");
    axum::serve(listener, app).await.expect("server error");
}
