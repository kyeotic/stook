use async_trait::async_trait;
use bollard::container::ListContainersOptions;
use bollard::Docker;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;
use tracing::{debug, error, info};

#[async_trait]
pub trait WebhookLookup: Send + Sync {
    async fn lookup(&self, repository: &str) -> Option<String>;
}

const IMAGE_LABEL: &str = "webhook-router.image";
const COMPOSE_PROJECT_LABEL: &str = "com.docker.compose.project";

pub struct Discovery {
    docker: Docker,
    cache: RwLock<Cache>,
    ttl: Duration,
}

struct Cache {
    map: HashMap<String, String>,
    updated_at: Option<Instant>,
}

impl Discovery {
    pub fn new(ttl_secs: u64) -> Result<Arc<Self>, bollard::errors::Error> {
        let docker = Docker::connect_with_local_defaults()?;
        Ok(Arc::new(Self {
            docker,
            cache: RwLock::new(Cache {
                map: HashMap::new(),
                updated_at: None,
            }),
            ttl: Duration::from_secs(ttl_secs),
        }))
    }
}

#[async_trait]
impl WebhookLookup for Discovery {
    async fn lookup(&self, repository: &str) -> Option<String> {
        {
            let cache = self.cache.read().await;
            if let Some(updated_at) = cache.updated_at {
                if updated_at.elapsed() < self.ttl {
                    return cache.map.get(repository).cloned();
                }
            }
        }
        self.refresh().await;
        self.cache.read().await.map.get(repository).cloned()
    }
}

impl Discovery {
    async fn refresh(&self) {
        debug!("refreshing container label cache");
        let mut filters = HashMap::new();
        filters.insert("label".to_string(), vec![IMAGE_LABEL.to_string()]);

        let opts = ListContainersOptions {
            all: true,
            filters,
            ..Default::default()
        };

        match self.docker.list_containers(Some(opts)).await {
            Ok(containers) => {
                let mut map = HashMap::new();
                for container in &containers {
                    if let Some(labels) = &container.labels {
                        if let (Some(image), Some(stack)) = (
                            labels.get(IMAGE_LABEL),
                            labels.get(COMPOSE_PROJECT_LABEL),
                        ) {
                            debug!(image = %image, stack = %stack, "discovered stack route");
                            map.insert(image.clone(), stack.clone());
                        }
                    }
                }
                info!(count = map.len(), "refreshed stack route cache");
                let mut cache = self.cache.write().await;
                cache.map = map;
                cache.updated_at = Some(Instant::now());
            }
            Err(e) => {
                error!(error = %e, "failed to query Docker for container labels");
            }
        }
    }
}
