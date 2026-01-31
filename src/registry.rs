use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct RegistryNotification {
    pub events: Vec<Event>,
}

#[derive(Debug, Deserialize)]
pub struct Event {
    pub action: String,
    pub target: Target,
}

#[derive(Debug, Deserialize)]
pub struct Target {
    pub repository: String,
    #[allow(dead_code)]
    pub tag: Option<String>,
}

impl RegistryNotification {
    pub fn push_repositories(&self) -> Vec<&str> {
        self.events
            .iter()
            .filter(|e| e.action == "push")
            .map(|e| e.target.repository.as_str())
            .collect()
    }
}
