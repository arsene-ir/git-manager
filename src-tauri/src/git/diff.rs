use crate::dto::{DiffHunk, DiffLine, FileDiff};

/// Parse one file's unified diff (git -U3 output) into hunks for the renderer.
pub fn parse_unified(text: &str, path: &str) -> FileDiff {
    if text.contains("Binary files") || text.contains("GIT binary patch") {
        return FileDiff { path: path.into(), old_path: None, binary: true, hunks: Vec::new() };
    }

    let mut hunks: Vec<DiffHunk> = Vec::new();
    let mut old_lineno = 0u32;
    let mut new_lineno = 0u32;

    for line in text.lines() {
        if line.starts_with("@@") {
            let (o, n) = parse_hunk_header(line);
            old_lineno = o;
            new_lineno = n;
            hunks.push(DiffHunk { header: line.to_string(), lines: Vec::new() });
            continue;
        }
        let Some(hunk) = hunks.last_mut() else { continue };
        if line.starts_with("\\ No newline") {
            continue;
        }

        let (kind, old, new) = if line.starts_with('+') {
            let n = new_lineno;
            new_lineno += 1;
            ("add", None, Some(n))
        } else if line.starts_with('-') {
            let o = old_lineno;
            old_lineno += 1;
            ("del", Some(o), None)
        } else if line.starts_with(' ') {
            let o = old_lineno;
            let n = new_lineno;
            old_lineno += 1;
            new_lineno += 1;
            ("ctx", Some(o), Some(n))
        } else {
            continue;
        };

        let text = line.chars().skip(1).collect::<String>();
        hunk.lines.push(DiffLine { kind: kind.into(), text, old_lineno: old, new_lineno: new });
    }

    FileDiff { path: path.into(), old_path: None, binary: false, hunks }
}

fn parse_hunk_header(line: &str) -> (u32, u32) {
    // @@ -oldStart,oldLen +newStart,newLen @@
    let mut old = 1;
    let mut new = 1;
    for token in line.split_whitespace() {
        if let Some(v) = token.strip_prefix('-') {
            old = v.split(',').next().and_then(|x| x.parse().ok()).unwrap_or(1);
        } else if let Some(v) = token.strip_prefix('+') {
            new = v.split(',').next().and_then(|x| x.parse().ok()).unwrap_or(1);
        }
    }
    (old, new)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_basic_hunks() {
        let text = "@@ -1,3 +1,4 @@\n context\n-old\n+new\n+added\n";
        let d = parse_unified(text, "a.txt");
        assert_eq!(d.hunks.len(), 1);
        let lines = &d.hunks[0].lines;
        assert_eq!(lines[0].kind, "ctx");
        assert_eq!(lines[1].text, "old");
        assert_eq!(lines[1].old_lineno, Some(2));
        assert_eq!(lines[2].kind, "add");
        assert_eq!(lines[2].new_lineno, Some(2));
    }

    #[test]
    fn detects_binary() {
        let d = parse_unified("Binary files a/x.png and b/x.png differ", "x.png");
        assert!(d.binary);
    }
}
