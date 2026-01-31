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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deserialize_valid_notification() {
        let json = r#"{"events":[{"action":"push","target":{"repository":"myrepo","tag":"latest"}}]}"#;
        let n: RegistryNotification = serde_json::from_str(json).unwrap();
        assert_eq!(n.events.len(), 1);
        assert_eq!(n.events[0].action, "push");
        assert_eq!(n.events[0].target.repository, "myrepo");
        assert_eq!(n.events[0].target.tag.as_deref(), Some("latest"));
    }

    #[test]
    fn push_repositories_filters_push_events() {
        let n = RegistryNotification {
            events: vec![
                Event { action: "push".into(), target: Target { repository: "a".into(), tag: None } },
                Event { action: "pull".into(), target: Target { repository: "b".into(), tag: None } },
                Event { action: "push".into(), target: Target { repository: "c".into(), tag: None } },
            ],
        };
        assert_eq!(n.push_repositories(), vec!["a", "c"]);
    }

    #[test]
    fn push_repositories_empty_for_non_push() {
        let n = RegistryNotification {
            events: vec![
                Event { action: "pull".into(), target: Target { repository: "x".into(), tag: None } },
            ],
        };
        assert!(n.push_repositories().is_empty());
    }

    #[test]
    fn push_repositories_empty_events() {
        let n = RegistryNotification { events: vec![] };
        assert!(n.push_repositories().is_empty());
    }
}
