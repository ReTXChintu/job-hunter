use chrono::{DateTime, Utc};

pub fn new_id() -> String {
    uuid::Uuid::new_v4().to_string()
}

pub fn now() -> DateTime<Utc> {
    Utc::now()
}

/// Turn any label into a filesystem and URL safe slug.
pub fn slugify(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    let mut last_dash = true;
    for ch in input.chars() {
        let c = ch.to_ascii_lowercase();
        if c.is_ascii_alphanumeric() {
            out.push(c);
            last_dash = false;
        } else if !last_dash {
            out.push('-');
            last_dash = true;
        }
    }
    let trimmed = out.trim_matches('-').to_string();
    if trimmed.is_empty() {
        "untitled".into()
    } else {
        trimmed
    }
}

/// Normalise free text for comparison: lowercase, collapse whitespace, strip
/// punctuation.
pub fn normalize_text(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    let mut last_space = true;
    for ch in input.chars() {
        if ch.is_alphanumeric() {
            out.extend(ch.to_lowercase());
            last_space = false;
        } else if !last_space {
            out.push(' ');
            last_space = true;
        }
    }
    out.trim().to_string()
}

pub fn truncate(s: &str, max_chars: usize) -> String {
    if s.chars().count() <= max_chars {
        s.to_string()
    } else {
        let mut t: String = s.chars().take(max_chars).collect();
        t.push_str("...");
        t
    }
}

/// Extract the first balanced JSON object or array from a blob of text.
/// Claude occasionally wraps JSON in prose or code fences; this recovers it.
pub fn extract_json(text: &str) -> Option<String> {
    let start = text.find(['{', '['])?;
    let bytes = text.as_bytes();
    let open = bytes[start] as char;
    let close = if open == '{' { '}' } else { ']' };
    let mut depth = 0i32;
    let mut in_str = false;
    let mut escape = false;
    for (i, ch) in text[start..].char_indices() {
        if in_str {
            if escape {
                escape = false;
            } else if ch == '\\' {
                escape = true;
            } else if ch == '"' {
                in_str = false;
            }
            continue;
        }
        match ch {
            '"' => in_str = true,
            c if c == open => depth += 1,
            c if c == close => {
                depth -= 1;
                if depth == 0 {
                    return Some(text[start..start + i + ch.len_utf8()].to_string());
                }
            }
            _ => {}
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slugify_basic() {
        assert_eq!(slugify("ABC Technologies, Inc."), "abc-technologies-inc");
        assert_eq!(slugify("   "), "untitled");
        assert_eq!(
            slugify("Senior Full-Stack Developer"),
            "senior-full-stack-developer"
        );
    }

    #[test]
    fn normalize_text_basic() {
        assert_eq!(normalize_text("  Hello,   World! "), "hello world");
        assert_eq!(normalize_text("React.js/Node.JS"), "react js node js");
    }

    #[test]
    fn extract_json_from_fence() {
        let text = "Here you go:\n```json\n{\"a\": [1, 2, {\"b\": \"}\"}]}\n```";
        assert_eq!(
            extract_json(text).unwrap(),
            "{\"a\": [1, 2, {\"b\": \"}\"}]}"
        );
        assert!(extract_json("no json here").is_none());
    }
}

/// True when `file` resolves to a location inside `root` (both canonicalised,
/// so short names, `..` and symlinks cannot escape the data directory).
pub fn path_is_inside(root: &std::path::Path, file: &std::path::Path) -> bool {
    let (Ok(root), Ok(file)) = (std::fs::canonicalize(root), std::fs::canonicalize(file)) else {
        return false;
    };
    let norm = |p: &std::path::Path| -> String {
        // Compare on components so `\\?\` prefixes, separators and (on
        // Windows) letter case do not matter.
        p.components()
            .filter_map(|c| match c {
                std::path::Component::Normal(s) => Some(s.to_string_lossy().to_string()),
                std::path::Component::Prefix(pr) => {
                    Some(pr.as_os_str().to_string_lossy().to_string())
                }
                _ => None,
            })
            .map(|s| if cfg!(windows) { s.to_lowercase() } else { s })
            .collect::<Vec<_>>()
            .join("/")
    };
    let r = norm(&root);
    let f = norm(&file);
    f == r || f.starts_with(&format!("{r}/"))
}

#[cfg(test)]
mod path_tests {
    use super::*;

    #[test]
    fn detects_files_inside_and_outside_root() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().join("data");
        std::fs::create_dir_all(root.join("resumes")).unwrap();
        let inside = root.join("resumes").join("r.html");
        std::fs::write(&inside, "x").unwrap();
        let outside = dir.path().join("other.html");
        std::fs::write(&outside, "x").unwrap();
        assert!(path_is_inside(&root, &inside));
        assert!(!path_is_inside(&root, &outside));
        assert!(!path_is_inside(
            &root,
            &root
                .join("resumes")
                .join("..")
                .join("..")
                .join("other.html")
        ));
        // Short (8.3) and forward-slash spellings must be accepted on Windows.
        let spelled = std::path::PathBuf::from(
            inside
                .to_string_lossy()
                .replace(std::path::MAIN_SEPARATOR, "/"),
        );
        assert!(path_is_inside(&root, &spelled));
    }
}
