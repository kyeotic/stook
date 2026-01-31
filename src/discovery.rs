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

const IMAGE_LABEL: &str = "stook.image";
const STOOK_LABEL: &str = "stook";
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
        filters.insert(
            "label".to_string(),
            vec![IMAGE_LABEL.to_string(), STOOK_LABEL.to_string()],
        );

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
                        let stack = match labels.get(COMPOSE_PROJECT_LABEL) {
                            Some(s) => s,
                            None => continue,
                        };

                        let repo = if let Some(image) = labels.get(IMAGE_LABEL) {
                            image.clone()
                        } else if labels.contains_key(STOOK_LABEL) {
                            let image_ref = container.image.as_deref().unwrap_or("");
                            repo_from_image(image_ref).to_string()
                        } else {
                            continue;
                        };

                        if !repo.is_empty() {
                            debug!(repo = %repo, stack = %stack, "discovered stack route");
                            map.insert(repo, stack.clone());
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

fn repo_from_image(image: &str) -> &str {
    // Strip :tag from the end
    let without_tag = match image.rfind(':') {
        Some(i) => &image[..i],
        None => image,
    };

    // If the first segment contains '.' or ':', it's a hostname — strip it
    match without_tag.find('/') {
        Some(i) => {
            let first_segment = &without_tag[..i];
            if first_segment.contains('.') || first_segment.contains(':') {
                &without_tag[i + 1..]
            } else {
                without_tag
            }
        }
        None => without_tag,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_repo_from_image() {
        assert_eq!(repo_from_image("registry.local/myrepo:latest"), "myrepo");
        assert_eq!(
            repo_from_image("registry.local/org/myrepo:latest"),
            "org/myrepo"
        );
        assert_eq!(repo_from_image("myrepo:latest"), "myrepo");
        assert_eq!(repo_from_image("myrepo"), "myrepo");
        assert_eq!(
            repo_from_image("localhost:5000/myrepo:v1"),
            "myrepo"
        );
        assert_eq!(repo_from_image(""), "");
    }
}
