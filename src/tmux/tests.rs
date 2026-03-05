use super::*;

#[test]
fn prepare_outgoing_text_appends_marker_for_regular_text() {
    let marker = "🦊🪐🧿";
    assert_eq!(
        prepare_outgoing_text("hello world", marker),
        "hello world 🦊🪐🧿"
    );
}

#[test]
fn prepare_outgoing_text_appends_marker_for_multiline_text() {
    let marker = "🦊🪐🧿";
    assert_eq!(
        prepare_outgoing_text("line1\nline2", marker),
        "line1\nline2 🦊🪐🧿"
    );
}

#[test]
fn prepare_outgoing_text_skips_markers_for_slash_commands() {
    let marker = "🦊🪐🧿";
    assert!(is_slash_command("   /clear"));
    assert!(is_slash_command("/status now"));
    assert_eq!(prepare_outgoing_text("   /clear", marker), "   /clear");
    assert_eq!(prepare_outgoing_text("/status now", marker), "/status now");
}
