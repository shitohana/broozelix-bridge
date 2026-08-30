/// Escape `\` and `"` for paths embedded in typable `:open "…"` commands.
pub fn escape_path_for_typable(path: &str) -> String {
    let mut out = String::with_capacity(path.len());
    for ch in path.chars() {
        match ch {
            '\\' | '"' => {
                out.push('\\');
                out.push(ch);
            }
            _ => out.push(ch),
        }
    }
    out
}

/// Build the typable open command body (hx adds the leading `:`).
pub fn open_command(escaped_path: &str) -> String {
    format!(r#"open "{}""#, escaped_path)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn escapes_backslash_and_quote() {
        assert_eq!(
            escape_path_for_typable(r#"/tmp/a"b\c"#),
            r#"/tmp/a\"b\\c"#
        );
    }

    #[test]
    fn open_command_wraps_escaped_path() {
        assert_eq!(
            open_command("/tmp/foo.rs"),
            r#"open "/tmp/foo.rs""#
        );
    }
}
