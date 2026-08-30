use std::collections::HashMap;
use std::fs;
use std::path::Path;

use anyhow::{Context, bail};
use serde::Deserialize;

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
pub struct Config {
    pub subscribe: SubscribeConfig,
    #[serde(default)]
    pub handler_defaults: HandlerDefaults,
    pub handlers: Vec<Handler>,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
pub struct SubscribeConfig {
    pub events: Vec<String>,
}

#[derive(Debug, Clone, Default, Deserialize, PartialEq, Eq)]
pub struct HandlerDefaults {
    #[serde(default)]
    pub debounce_ms: u64,
    #[serde(default = "default_spawn")]
    pub spawn: SpawnMode,
}

#[derive(Debug, Clone, Copy, Default, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum SpawnMode {
    #[default]
    Async,
    Sync,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
pub struct Handler {
    pub event: String,
    pub command: String,
    #[serde(default)]
    pub debounce_ms: Option<u64>,
    #[serde(default)]
    pub spawn: Option<SpawnMode>,
    #[serde(default)]
    pub when: Option<HashMap<String, String>>,
}

fn default_spawn() -> SpawnMode {
    SpawnMode::Async
}

impl Handler {
    pub fn effective_debounce_ms(&self, defaults: &HandlerDefaults) -> u64 {
        self.debounce_ms.unwrap_or(defaults.debounce_ms)
    }

    pub fn effective_spawn(&self, defaults: &HandlerDefaults) -> SpawnMode {
        self.spawn.unwrap_or(defaults.spawn)
    }
}

impl Config {
    pub fn load(path: &Path) -> anyhow::Result<Self> {
        let raw = fs::read_to_string(path)
            .with_context(|| format!("failed to read config {}", path.display()))?;
        let expanded = expand_env_vars(&raw);
        let config: Config = toml::from_str(&expanded)
            .with_context(|| format!("failed to parse config {}", path.display()))?;
        config.validate()?;
        Ok(config)
    }

    fn validate(&self) -> anyhow::Result<()> {
        if self.subscribe.events.is_empty() {
            bail!("subscribe.events must not be empty");
        }
        for event in &self.subscribe.events {
            if !crate::protocol::socket_json::V1_EVENTS.contains(&event.as_str()) {
                log::warn!("subscribe.events contains unknown v1 event: {event}");
            }
        }
        if self.handlers.is_empty() {
            bail!("at least one [[handlers]] entry is required");
        }
        for handler in &self.handlers {
            if handler.command.trim().is_empty() {
                bail!("handler for event '{}' has empty command", handler.event);
            }
        }
        Ok(())
    }

    pub fn subscribe_event_list(&self) -> String {
        self.subscribe.events.join(",")
    }
}

/// Expand `$VAR` and `${VAR}` from the process environment.
pub fn expand_env_vars(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    let bytes = input.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'$' {
            if i + 1 < bytes.len() && bytes[i + 1] == b'{' {
                if let Some((var, end)) = parse_braced_var(&input[i + 2..]) {
                    push_var(&mut out, var);
                    i += 2 + end + 1;
                    continue;
                }
            } else if let Some((var, len)) = parse_simple_var(&input[i + 1..]) {
                push_var(&mut out, var);
                i += 1 + len;
                continue;
            }
        }
        out.push(bytes[i] as char);
        i += 1;
    }
    out
}

fn parse_braced_var(rest: &str) -> Option<(&str, usize)> {
    let end = rest.find('}')?;
    let var = &rest[..end];
    if var.is_empty() || !var.chars().all(is_var_char) {
        return None;
    }
    Some((var, end))
}

fn parse_simple_var(rest: &str) -> Option<(&str, usize)> {
    let len = rest.chars().take_while(|c| is_var_char(*c)).count();
    if len == 0 {
        return None;
    }
    Some((&rest[..len], len))
}

fn is_var_char(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == '_'
}

fn push_var(out: &mut String, var: &str) {
    match std::env::var(var) {
        Ok(value) => out.push_str(&value),
        Err(_) => {
            log::warn!("config references unset environment variable: {var}");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn expands_dollar_and_braced_vars() {
        // SAFETY: test-only env mutation; single-threaded `cargo test`.
        unsafe {
            std::env::set_var("BB_TEST_VAR", "hello");
        }
        assert_eq!(
            expand_env_vars("prefix $BB_TEST_VAR ${BB_TEST_VAR}-suffix"),
            "prefix hello hello-suffix"
        );
        unsafe {
            std::env::remove_var("BB_TEST_VAR");
        }
    }

    #[test]
    fn parses_minimal_config() {
        let raw = r#"
[subscribe]
events = ["document-focus"]

[[handlers]]
event = "document-focus"
command = "echo {path}"
"#;
        let config: Config = toml::from_str(raw).unwrap();
        assert_eq!(config.subscribe.events, vec!["document-focus"]);
        assert_eq!(config.handlers.len(), 1);
        assert_eq!(
            config.handlers[0].effective_debounce_ms(&config.handler_defaults),
            0
        );
    }
}
