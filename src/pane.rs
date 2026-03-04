#![allow(dead_code)]

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AgentType {
    Claude,
    Codex,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AgentState {
    Working,
    Idle,
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PaneState {
    pub agent_type: Option<AgentType>,
    pub state: AgentState,
    pub model: Option<String>,
    pub context_remaining: Option<String>,
    pub activity: Option<String>,
}

impl Default for PaneState {
    fn default() -> Self {
        Self {
            agent_type: None,
            state: AgentState::Unknown,
            model: None,
            context_remaining: None,
            activity: None,
        }
    }
}

pub fn parse_pane_content(text: &str) -> PaneState {
    let lines: Vec<&str> = text.lines().collect();
    if lines.is_empty() {
        return PaneState::default();
    }

    let start = lines.len().saturating_sub(30);
    let recent = &lines[start..];

    if let Some(state) = parse_codex(recent) {
        return state;
    }
    if let Some(state) = parse_claude(recent) {
        return state;
    }

    PaneState::default()
}

fn parse_codex(lines: &[&str]) -> Option<PaneState> {
    let status = lines
        .iter()
        .rev()
        .find_map(|line| parse_codex_status_line(line))?;
    let has_prompt = lines.iter().any(|line| line.trim_start().starts_with('›'));
    let has_working = lines
        .iter()
        .any(|line| line.contains("Working (") && line.contains("esc to interrupt"));

    if has_working {
        let activity = lines
            .iter()
            .rev()
            .find_map(|line| line.trim_start().strip_prefix("• ").map(str::trim))
            .filter(|value| !value.is_empty())
            .map(ToString::to_string);
        return Some(PaneState {
            agent_type: Some(AgentType::Codex),
            state: AgentState::Working,
            model: Some(status.model),
            context_remaining: Some(status.context_remaining),
            activity,
        });
    }

    if has_prompt {
        return Some(PaneState {
            agent_type: Some(AgentType::Codex),
            state: AgentState::Idle,
            model: Some(status.model),
            context_remaining: Some(status.context_remaining),
            activity: None,
        });
    }

    None
}

fn parse_claude(lines: &[&str]) -> Option<PaneState> {
    let has_prompt = lines.iter().any(|line| {
        let trimmed = line.trim();
        trimmed == "❯" || trimmed.starts_with("❯ ")
    });
    let has_footer = lines
        .iter()
        .any(|line| line.contains("bypass permissions") || line.contains("shift+tab to cycle"));

    let spinner_line = lines.iter().rev().find_map(|line| {
        let trimmed = line.trim_start();
        trimmed
            .strip_prefix("✻ ")
            .or_else(|| trimmed.strip_prefix("⏺ "))
    });
    let has_running = lines.iter().any(|line| line.contains("Running…"));
    let spinner_has_tokens = spinner_line
        .map(|line| line.contains("↓") && line.contains("tokens"))
        .unwrap_or(false);

    if let Some(activity_line) = spinner_line
        && (has_running || spinner_has_tokens)
    {
        return Some(PaneState {
            agent_type: Some(AgentType::Claude),
            state: AgentState::Working,
            model: None,
            context_remaining: extract_tokens_phrase(lines),
            activity: Some(activity_line.trim().to_string()),
        });
    }

    if has_prompt && has_footer {
        return Some(PaneState {
            agent_type: Some(AgentType::Claude),
            state: AgentState::Idle,
            model: None,
            context_remaining: extract_tokens_phrase(lines),
            activity: None,
        });
    }

    None
}

struct CodexStatus {
    model: String,
    context_remaining: String,
}

fn parse_codex_status_line(line: &str) -> Option<CodexStatus> {
    let trimmed = line.trim();
    if !trimmed.contains("gpt-") || !trimmed.contains('·') {
        return None;
    }

    let parts: Vec<&str> = trimmed.split('·').map(str::trim).collect();
    if parts.len() < 2 {
        return None;
    }
    if !parts[1].contains("% left") && !parts[1].contains("tokens") {
        return None;
    }

    let model = parts[0];
    if model.is_empty() {
        return None;
    }

    Some(CodexStatus {
        model: model.to_string(),
        context_remaining: parts[1].to_string(),
    })
}

fn extract_tokens_phrase(lines: &[&str]) -> Option<String> {
    lines.iter().rev().find_map(|line| extract_tokens_from_line(line))
}

fn extract_tokens_from_line(line: &str) -> Option<String> {
    let bytes = line.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if !bytes[i].is_ascii_digit() {
            i += 1;
            continue;
        }

        let start = i;
        while i < bytes.len() && bytes[i].is_ascii_digit() {
            i += 1;
        }
        let digits = &line[start..i];

        while i < bytes.len() && bytes[i].is_ascii_whitespace() {
            i += 1;
        }

        if line[i..].starts_with("tokens") {
            return Some(format!("{digits} tokens"));
        }
    }

    None
}
