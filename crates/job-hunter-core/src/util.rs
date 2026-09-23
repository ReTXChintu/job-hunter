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
