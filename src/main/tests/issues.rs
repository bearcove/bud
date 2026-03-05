use crate::issues::{DraftMissingStage, format_missing_draft_message};
use std::path::Path;

#[test]
fn missing_draft_message_mentions_concurrency_only_with_evidence() {
    let path = Path::new("/tmp/mate-issues/example/new/draft.md");

    let neutral = format_missing_draft_message(path, DraftMissingStage::AfterCreate, false);
    assert!(neutral.contains("already removed after create"));
    assert!(!neutral.to_ascii_lowercase().contains("concurrent"));

    let concurrent = format_missing_draft_message(path, DraftMissingStage::AfterCreate, true);
    assert!(concurrent.contains("Concurrent `mate issues` run detected."));
}
