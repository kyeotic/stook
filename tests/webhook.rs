use async_trait::async_trait;
use axum::body::Body;
use axum::http::{Request, StatusCode};
use axum::routing::{get, post};
use axum::Router;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use stook::discovery::WebhookLookup;
use stook::redeployer::{RedeployError, StackRedeployer};
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

struct MockRedeployer {
    calls: Mutex<Vec<String>>,
}

#[async_trait]
impl StackRedeployer for MockRedeployer {
    async fn redeploy(&self, stack_name: &str) -> Result<(), RedeployError> {
        self.calls.lock().unwrap().push(stack_name.to_string());
        Ok(())
    }
}

fn app(lookup: HashMap<String, String>, redeployer: Arc<MockRedeployer>) -> Router {
    let state = Arc::new(AppState {
        discovery: Arc::new(MockLookup { map: lookup }),
        redeployer,
    });
    Router::new()
        .route("/webhook", post(routes::webhook))
        .route("/health", get(routes::health))
        .with_state(state)
}

#[tokio::test]
async fn matching_repo_redeploys() {
    let mut map = HashMap::new();
    map.insert("myrepo".into(), "mystack".into());
    let redeployer = Arc::new(MockRedeployer { calls: Mutex::new(vec![]) });
    let app = app(map, redeployer.clone());

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
    assert_eq!(redeployer.calls.lock().unwrap().as_slice(), &["mystack"]);
}

#[tokio::test]
async fn unmatched_repo_no_redeploy() {
    let redeployer = Arc::new(MockRedeployer { calls: Mutex::new(vec![]) });
    let app = app(HashMap::new(), redeployer.clone());

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
    assert!(redeployer.calls.lock().unwrap().is_empty());
}

#[tokio::test]
async fn mixed_push_pull_only_push_redeployed() {
    let mut map = HashMap::new();
    map.insert("a".into(), "stack-a".into());
    map.insert("b".into(), "stack-b".into());
    let redeployer = Arc::new(MockRedeployer { calls: Mutex::new(vec![]) });
    let app = app(map, redeployer.clone());

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
    assert_eq!(redeployer.calls.lock().unwrap().as_slice(), &["stack-a"]);
}

#[tokio::test]
async fn invalid_json_returns_400() {
    let redeployer = Arc::new(MockRedeployer { calls: Mutex::new(vec![]) });
    let app = app(HashMap::new(), redeployer);

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
    let redeployer = Arc::new(MockRedeployer { calls: Mutex::new(vec![]) });
    let app = app(HashMap::new(), redeployer);

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
