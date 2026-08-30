use crate::protocol::socket_json::SocketLine;

/// Substitute `{path}`, `{line}`, `{column}`, `{mode}` from a Helix event line.
pub fn substitute(command: &str, event: &SocketLine) -> String {
    let path = event.path.as_deref().unwrap_or("");
    let line = event
        .line
        .map(|n| n.to_string())
        .unwrap_or_default();
    let column = event
        .column
        .map(|n| n.to_string())
        .unwrap_or_default();
    let mode = event.mode.as_deref().unwrap_or("");

    command
        .replace("{path}", path)
        .replace("{line}", &line)
        .replace("{column}", &column)
        .replace("{mode}", mode)
}

/// Returns true when optional `when` filters match the event fields.
pub fn matches_when(when: &std::collections::HashMap<String, String>, event: &SocketLine) -> bool {
    for (key, expected) in when {
        let actual = match key.as_str() {
            "path" => event.path.as_deref().unwrap_or(""),
            "line" => return event.line.map(|n| n.to_string()) == Some(expected.clone()),
            "column" => return event.column.map(|n| n.to_string()) == Some(expected.clone()),
            "mode" => event.mode.as_deref().unwrap_or(""),
            "event" => event.event.as_deref().unwrap_or(""),
            _ => return false,
        };
        if actual != expected.as_str() {
            return false;
        }
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::protocol::socket_json::SocketLine;

    fn sample_event() -> SocketLine {
        serde_json::from_str(
            r#"{"v":1,"op":"event","event":"document-focus","path":"/tmp/a.rs","line":3,"column":1,"mode":"normal"}"#,
        )
        .unwrap()
    }

    #[test]
    fn substitutes_placeholders() {
        let event = sample_event();
        let out = substitute("show {path}:{line}:{column} mode={mode}", &event);
        assert_eq!(out, "show /tmp/a.rs:3:1 mode=normal");
    }

    #[test]
    fn missing_fields_become_empty() {
        let event: SocketLine =
            serde_json::from_str(r#"{"v":1,"op":"event","event":"mode-changed","mode":"insert"}"#)
                .unwrap();
        let out = substitute("{path}|{line}|{column}|{mode}", &event);
        assert_eq!(out, "|||insert");
    }

    #[test]
    fn when_filter_matches_mode() {
        let event = sample_event();
        let mut when = std::collections::HashMap::new();
        when.insert("mode".into(), "normal".into());
        assert!(matches_when(&when, &event));
        when.insert("mode".into(), "insert".into());
        assert!(!matches_when(&when, &event));
    }
}
