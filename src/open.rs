use std::path::{Path, PathBuf};

use anyhow::{Context, Result};

use crate::cli::Cli;
use crate::helix;
use crate::protocol::typable::{escape_path_for_typable, open_command};

/// Parse `path`, optional `:line`, optional `:column` suffix from the right.
pub fn parse_open_arg(arg: &str) -> (PathBuf, Option<u64>, Option<u64>) {
    let parts: Vec<&str> = arg.rsplitn(3, ':').collect();
    match parts.as_slice() {
        [column, line, path] if is_digits(column) && is_digits(line) => {
            let full = format!("{path}:{line}:{column}");
            (
                PathBuf::from(full),
                line.parse().ok(),
                column.parse().ok(),
            )
        }
        [line, path] if is_digits(line) => {
            let full = format!("{path}:{line}");
            (PathBuf::from(full), line.parse().ok(), None)
        }
        _ => (PathBuf::from(arg), None, None),
    }
}

fn is_digits(s: &str) -> bool {
    !s.is_empty() && s.chars().all(|c| c.is_ascii_digit())
}

pub fn canonicalize_open_path(path: &Path) -> Result<PathBuf> {
    let path_str = path.to_string_lossy();
    let base = path_str.split(':').next().unwrap_or(&path_str);
    let base_path = Path::new(base);

    if base_path.exists() {
        let canonical = base_path.canonicalize().with_context(|| {
            format!("failed to canonicalize {}", base_path.display())
        })?;
        if let Some(suffix) = path_str.strip_prefix(base) {
            return Ok(PathBuf::from(format!("{}{suffix}", canonical.display())));
        }
        return Ok(canonical);
    }

    if let Some(parent) = base_path.parent() {
        if !parent.as_os_str().is_empty() {
            std::fs::create_dir_all(parent).with_context(|| {
                format!("failed to create parent directory {}", parent.display())
            })?;
        }
    }
    Ok(path.to_path_buf())
}

pub fn run(cli: &Cli, arg: &str) -> Result<()> {
    let socket = cli.helix_socket()?;
    let (path, _line, _column) = parse_open_arg(arg);
    let path = canonicalize_open_path(&path)?;
    let path_str = path.to_string_lossy();
    let escaped = escape_path_for_typable(&path_str);
    let cmd = open_command(&escaped);
    log::debug!("open command: {cmd}");
    helix::remote(cli, &socket, &cmd)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_line_suffix() {
        let (path, line, column) = parse_open_arg("/tmp/foo.rs:12");
        assert_eq!(path, PathBuf::from("/tmp/foo.rs:12"));
        assert_eq!(line, Some(12));
        assert_eq!(column, None);
    }

    #[test]
    fn parses_line_and_column_suffix() {
        let (path, line, column) = parse_open_arg("/tmp/foo.rs:12:3");
        assert_eq!(path, PathBuf::from("/tmp/foo.rs:12:3"));
        assert_eq!(line, Some(12));
        assert_eq!(column, Some(3));
    }

    #[test]
    fn leaves_plain_path_untouched() {
        let (path, line, column) = parse_open_arg("/tmp/foo.rs");
        assert_eq!(path, PathBuf::from("/tmp/foo.rs"));
        assert_eq!(line, None);
        assert_eq!(column, None);
    }
}
