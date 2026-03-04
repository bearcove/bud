use std::path::Path;
use std::time::Duration;

pub fn format_age(age: Duration) -> String {
    let secs = age.as_secs();
    if secs < 60 {
        format!("{secs}s")
    } else if secs < 3_600 {
        format!("{}m", secs / 60)
    } else if secs < 86_400 {
        format!("{}h", secs / 3_600)
    } else {
        format!("{}d", secs / 86_400)
    }
}

pub fn serialize_request_file(source_pane: &str, title: Option<&str>) -> String {
    match title {
        Some(title) if !title.trim().is_empty() => format!("{source_pane}\n{}", title.trim()),
        _ => source_pane.to_string(),
    }
}

pub fn parse_request_file(path: &Path) -> Option<(String, Option<String>)> {
    let content = std::fs::read_to_string(path).ok()?;
    let mut lines = content.lines();
    let source_pane = lines.next()?.trim().to_string();
    if source_pane.is_empty() {
        return None;
    }
    let title = lines
        .next()
        .map(str::trim)
        .filter(|title| !title.is_empty())
        .map(ToString::to_string);
    Some((source_pane, title))
}
