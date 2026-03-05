use super::{AgentState, AgentType, parse_pane_content};

#[test]
fn detects_claude_working_snapshot_from_recording() {
    let text = "\
✽ Combobulating… (0s)

────────────────────────────────────────────────────────────────────────────────
❯
────────────────────────────────────────────────────────────────────────────────
  esc to interrupt                                                             0 tokens
                                                       current: 2.1.68 · latest: 2.1.68
";
    let parsed = parse_pane_content(text);
    assert_eq!(parsed.agent_type, Some(AgentType::Claude));
    assert_eq!(parsed.state, AgentState::Working);
}

#[test]
fn detects_claude_idle_snapshot_from_recording() {
    let text = "\
⏺ Done.
✻ Worked for 1m 14s

Resume this session with:
claude --resume eea841a9-c5e8-4176-a995-c52ddd9a3c23
";
    let parsed = parse_pane_content(text);
    assert_eq!(parsed.agent_type, Some(AgentType::Claude));
    assert_eq!(parsed.state, AgentState::Idle);
}

#[test]
fn detects_codex_working_snapshot_from_recording() {
    let text = "\
• Working (35s • esc to interrupt) · 1 background terminal running · /ps to view · /clean to close

› Run /review on my current changes

  gpt-5.3-codex medium · 98% left · ~/bearcove/mucp
";
    let parsed = parse_pane_content(text);
    assert_eq!(parsed.agent_type, Some(AgentType::Codex));
    assert_eq!(parsed.state, AgentState::Working);
}

#[test]
fn detects_codex_idle_snapshot_from_recording() {
    let text = "\
╭────────────────────────────────────────────────────╮
│ >_ OpenAI Codex (v0.107.0)                         │
│ model:     gpt-5.3-codex medium   /model to change │
╰────────────────────────────────────────────────────╯

› Run /review on my current changes
";
    let parsed = parse_pane_content(text);
    assert_eq!(parsed.agent_type, Some(AgentType::Codex));
    assert_eq!(parsed.state, AgentState::Idle);
}

#[test]
fn does_not_misclassify_plain_shell_prompt_as_claude() {
    let text = "\
~/repo
❯ ls -la
Cargo.toml
";
    let parsed = parse_pane_content(text);
    assert_eq!(parsed.agent_type, None);
    assert_eq!(parsed.state, AgentState::Unknown);
}

#[test]
fn does_not_misclassify_generic_gpt_status_as_codex() {
    let text = "\
› run the checks
gpt-4.1 mini · 80% left · ~/repo
";
    let parsed = parse_pane_content(text);
    assert_eq!(parsed.agent_type, None);
    assert_eq!(parsed.state, AgentState::Unknown);
}

#[test]
fn does_not_misclassify_generic_working_status_as_codex() {
    let text = "\
›
• Working (12s • esc to interrupt)
";
    let parsed = parse_pane_content(text);
    assert_eq!(parsed.agent_type, None);
    assert_eq!(parsed.state, AgentState::Unknown);
}

#[test]
fn does_not_misclassify_spinner_like_line_as_claude() {
    let text = "\
❯
✻ Indexing… (0s)
";
    let parsed = parse_pane_content(text);
    assert_eq!(parsed.agent_type, None);
    assert_eq!(parsed.state, AgentState::Unknown);
}
