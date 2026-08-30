use serde::Deserialize;

pub const PROTOCOL_V: u32 = 1;
pub const OP_EVENT: &str = "event";
pub const OP_ERROR: &str = "error";

pub const V1_EVENTS: &[&str] = &[
    "document-focus",
    "document-open",
    "document-close",
    "document-write",
    "mode-changed",
    "cwd-changed",
];

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
pub struct SocketLine {
    pub v: u32,
    pub op: String,
    #[serde(default)]
    pub event: Option<String>,
    #[serde(default)]
    pub path: Option<String>,
    #[serde(default)]
    pub line: Option<u64>,
    #[serde(default)]
    pub column: Option<u64>,
    #[serde(default)]
    pub mode: Option<String>,
    #[serde(default)]
    pub error: Option<String>,
}

impl SocketLine {
    pub fn is_event(&self) -> bool {
        self.op == OP_EVENT
    }

    pub fn is_error(&self) -> bool {
        self.op == OP_ERROR
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn v1_events_match_helix() {
        assert_eq!(V1_EVENTS.len(), 6);
        assert!(V1_EVENTS.contains(&"document-focus"));
        assert!(V1_EVENTS.contains(&"cwd-changed"));
    }

    #[test]
    fn parses_document_focus() {
        let line: SocketLine = serde_json::from_str(
            r#"{"v":1,"op":"event","event":"document-focus","path":"/tmp/a.rs","line":12,"column":4}"#,
        )
        .unwrap();
        assert!(line.is_event());
        assert_eq!(line.event.as_deref(), Some("document-focus"));
        assert_eq!(line.path.as_deref(), Some("/tmp/a.rs"));
        assert_eq!(line.line, Some(12));
        assert_eq!(line.column, Some(4));
    }
}
