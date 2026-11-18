use std::path::{Path, PathBuf};

use crate::error::{AppError, AppResult};

/// Canonicalize and validate a repository directory chosen by the user.
pub fn canonical_dir(path: &str) -> AppResult<PathBuf> {
    if path.trim().is_empty() {
        return Err(AppError::msg("Repository path is empty"));
    }
    if path.contains('\0') {
        return Err(AppError::msg("Invalid path"));
    }
    let p = PathBuf::from(path);
    let c = std::fs::canonicalize(&p).map_err(|e| AppError::msg(format!("Path not found: {e}")))?;
    if !c.is_dir() {
        return Err(AppError::msg("Path is not a directory"));
    }
    Ok(c)
}

/// Validate ref / branch / remote names before handing them to git.
/// Rejects option-injection and shell metacharacters. Args are never shell-interpolated.
pub fn validate_ref(name: &str) -> AppResult<()> {
    let bad_chars = |c: char| {
        c.is_whitespace()
            || matches!(
                c,
                '\'' | '"' | '`' | '$' | ';' | '|' | '&' | '<' | '>' | '\\' | '!'
            )
    };
    if name.is_empty()
        || name.starts_with('-')
        || name.contains("..")
        || name.contains("@{")
        || name.contains('\0')
        || name.contains("//")
        || name.ends_with('/')
        || name.ends_with('.')
        || name.ends_with(".lock")
        || name.chars().any(bad_chars)
    {
        return Err(AppError::msg(format!("Invalid git name: {name}")));
    }
    Ok(())
}

/// Validate a remote URL. Keeps URL schemes we can hand to git safely.
pub fn validate_url(url: &str) -> AppResult<()> {
    if url.is_empty() || url.starts_with('-') || url.contains('\0') || url.chars().any(|c| c.is_whitespace()) {
        return Err(AppError::msg("Invalid remote URL"));
    }
    let ok = url.starts_with("https://")
        || url.starts_with("http://")
        || url.starts_with("ssh://")
        || url.starts_with("git://")
        || url.starts_with("git@")
        || Path::new(url).is_absolute();
    if !ok {
        return Err(AppError::msg(
            "Unsupported URL. Use https://, ssh://, git@host:path or an absolute path",
        ));
    }
    Ok(())
}

/// Never echo credentials or tokens into logs / UI errors.
pub fn redact(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for token in text.split_whitespace() {
        let lower = token.to_ascii_lowercase();
        let looks_secret = (lower.contains("://") && token.contains('@'))
            || lower.starts_with("token=")
            || lower.starts_with("password=")
            || lower.starts_with("authorization:")
            || lower.starts_with("ghp_")
            || lower.starts_with("glpat-");
        if looks_secret {
            if let Some(scheme_end) = token.find("://") {
                let scheme = &token[..scheme_end + 3];
                let rest = &token[scheme_end + 3..];
                let host = rest.split('@').next_back().unwrap_or("");
                out.push_str(&format!("{scheme}<redacted>@{host}"));
            } else {
                out.push_str("<redacted>");
            }
        } else if token.len() > 40 && token.chars().all(|c| c.is_ascii_alphanumeric()) {
            out.push_str("<redacted>");
        } else {
            out.push_str(token);
        }
        out.push(' ');
    }
    out.trim_end().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_option_injection() {
        assert!(validate_ref("--upload-pack=evil").is_err());
        assert!(validate_ref("main; rm -rf /").is_err());
        assert!(validate_ref("feature/ok-name").is_ok());
    }

    #[test]
    fn redacts_credentials() {
        let s = redact("fatal: https://user:secret@github.com/x.git failed");
        assert!(!s.contains("secret"));
    }
}
