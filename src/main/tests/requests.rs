use crate::issues::DraftCleanupOutcome;

fn format_captain_update_for_buddy(request_id: &str, message: &str) -> String {
    format!(
        "📌 Update from the captain on task {request_id}:\n\n\
         {message}\n\n\
         If you hit a decision point, want to share progress, or need clarification, send an update:\n\n\
         cat <<'MATEEOF' | mate update {request_id}\n\
         <your progress update here>\n\
         MATEEOF"
    )
}

#[test]
fn captain_update_includes_buddy_response_instructions() {
    let request_id = "deadbeef";
    let update = format_captain_update_for_buddy(request_id, "Please focus on parser tests.");

    assert!(update.contains("📌 Update from the captain on task deadbeef:"));
    assert!(update.contains("cat <<'MATEEOF' | mate update deadbeef"));
    assert!(!update.contains("mate accept deadbeef"));
    assert!(update.contains("<your progress update here>"));
    assert!(!update.contains("<your reply here>"));
    assert!(!update.contains("mate respond deadbeef"));
}

#[test]
fn cleanup_created_draft_handles_removed_and_missing_states() {
    let root = std::env::temp_dir().join(format!("mate-test-{}", uuid::Uuid::new_v4()));
    std::fs::create_dir_all(&root).expect("create temp directory");
    let existing = root.join("existing.md");
    std::fs::write(&existing, "draft").expect("write draft file");

    let removed = crate::issues::cleanup_created_draft(&existing).expect("remove existing draft");
    assert_eq!(removed, DraftCleanupOutcome::Removed);
    assert!(!existing.exists(), "existing draft should be removed");

    let missing = root.join("missing.md");
    let missing_outcome =
        crate::issues::cleanup_created_draft(&missing).expect("remove missing draft");
    assert_eq!(missing_outcome, DraftCleanupOutcome::Missing);

    std::fs::remove_dir_all(&root).expect("remove temp directory");
}
