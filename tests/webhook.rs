use async_trait::async_trait;
use axum::body::Body;
use axum::http::{Request, StatusCode};
use axum::routing::{get, post};
use axum::Router;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use stook::discovery::WebhookLookup;
use stook::forwarder::WebhookForwarder;
use stook::routes::{self, AppState};
use tower::ServiceExt;

struct MockLookup {
    map: HashMap<String, String>,
}

#[async_trait]
impl WebhookLookup for MockLookup {
    async fn lookup(&self, repository: &str) -> Option<String> {
        self.map.get(repository).cloned()
    }
}

struct MockForwarder {
    calls: Mutex<Vec<String>>,
}

#[async_trait]
impl WebhookForwarder for MockForwarder {
    async fn forward(&self, webhook_url: &str) {
        self.calls.lock().unwrap().push(webhook_url.to_string());
    }
}

fn app(lookup: HashMap<String, String>, forwarder: Arc<MockForwarder>) -> Router {
    let state = Arc::new(AppState {
        discovery: Arc::new(MockLookup { map: lookup }),
        forwarder,
    });
    Router::new()
        .route("/webhook", post(routes::webhook))
        .route("/health", get(routes::health))
        .with_state(state)
}

#[tokio::test]
async fn matching_repo_forwards() {
    let mut map = HashMap::new();
    map.insert("myrepo".into(), "http://hook.test".into());
    let fwd = Arc::new(MockForwarder { calls: Mutex::new(vec![]) });
    let app = app(map, fwd.clone());

    let body = r#"{"events":[{"action":"push","target":{"repository":"myrepo","tag":"latest"}}]}"#;
    let resp = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/webhook")
                .header("content-type", "application/json")
                .body(Body::from(body))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
    assert_eq!(fwd.calls.lock().unwrap().as_slice(), &["http://hook.test"]);
}

#[tokio::test]
async fn unmatched_repo_no_forward() {
    let fwd = Arc::new(MockForwarder { calls: Mutex::new(vec![]) });
    let app = app(HashMap::new(), fwd.clone());

    let body = r#"{"events":[{"action":"push","target":{"repository":"unknown","tag":"latest"}}]}"#;
    let resp = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/webhook")
                .header("content-type", "application/json")
                .body(Body::from(body))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
    assert!(fwd.calls.lock().unwrap().is_empty());
}

#[tokio::test]
async fn mixed_push_pull_only_push_forwarded() {
    let mut map = HashMap::new();
    map.insert("a".into(), "http://a.test".into());
    map.insert("b".into(), "http://b.test".into());
    let fwd = Arc::new(MockForwarder { calls: Mutex::new(vec![]) });
    let app = app(map, fwd.clone());

    let body = r#"{"events":[
        {"action":"push","target":{"repository":"a","tag":null}},
        {"action":"pull","target":{"repository":"b","tag":null}}
    ]}"#;
    let resp = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/webhook")
                .header("content-type", "application/json")
                .body(Body::from(body))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
    assert_eq!(fwd.calls.lock().unwrap().as_slice(), &["http://a.test"]);
}

#[tokio::test]
async fn invalid_json_returns_400() {
    let fwd = Arc::new(MockForwarder { calls: Mutex::new(vec![]) });
    let app = app(HashMap::new(), fwd);

    let resp = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/webhook")
                .header("content-type", "application/json")
                .body(Body::from("not json"))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn health_returns_200() {
    let fwd = Arc::new(MockForwarder { calls: Mutex::new(vec![]) });
    let app = app(HashMap::new(), fwd);

    let resp = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/health")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
}
