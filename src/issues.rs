use eyre::Result;
use tracing::trace;

use crate::github;

pub(crate) fn sync_issues_to_pane() -> Result<()> {
    trace!("sync_issues_to_pane: enter");
    let repo = github::infer_repo()?;
    eprintln!("Syncing issues for {repo}...");

    trace!("sync_issues_to_pane: before process_pending_issue_drafts");
    let (created, failed) = process_pending_issue_drafts(&repo)?;

    trace!("sync_issues_to_pane: before github::sync_issues");
    let issues = github::sync_issues(&repo)?;
    trace!("sync_issues_to_pane: before github::write_issue_files");
    let result = github::write_issue_files(&repo, &issues)?;
    trace!("sync_issues_to_pane: after github::write_issue_files");

    let mut summary = String::new();
    if !result.issue_edits_applied.is_empty() {
        summary.push_str("Applied issue edits:\n");
        for update in &result.issue_edits_applied {
            summary.push_str(&format!(
                "  Updated issue #{}: {}\n",
                update.number,
                update.changes.join(", ")
            ));
        }
        summary.push('\n');
    }
    if !created.is_empty() {
        summary.push_str(&format!("Created {} new issues:\n", created.len()));
        for pending in &created {
            summary.push_str(&format!(
                "  #{number}: {title} — {url}\n",
                number = pending.number,
                title = pending.title,
                url = pending.url
            ));
        }
        summary.push('\n');
    }

    for failure in &failed {
        summary.push_str(&format!(
            "Failed to create {}: {}\n",
            failure.filename, failure.error
        ));
    }
    if !failed.is_empty() {
        summary.push('\n');
    }

    if !result.issue_edit_errors.is_empty() {
        summary.push_str("Issue edit failures:\n");
        for failure in &result.issue_edit_errors {
            summary.push_str(&format!("- {failure}\n"));
        }
        summary.push('\n');
    }

    summary.push_str(&format!(
        "Synced {repo} — {} open, {} closed. Index: {}\n",
        result.open_count,
        result.closed_count,
        result.index_path.display()
    ));
    println!("{summary}");
    Ok(())
}

#[derive(Debug, Clone)]
pub(crate) struct PendingIssueCreated {
    pub(crate) number: u64,
    pub(crate) url: String,
    pub(crate) title: String,
}

#[derive(Debug, Clone)]
pub(crate) struct PendingIssueFailed {
    pub(crate) filename: String,
    pub(crate) error: String,
}

#[derive(Clone, Copy)]
pub(crate) enum DraftMissingStage {
    BeforeRead,
    AfterCreate,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum DraftCleanupOutcome {
    Removed,
    Missing,
}

pub(crate) fn format_missing_draft_message(
    path: &std::path::Path,
    stage: DraftMissingStage,
    has_concurrency_evidence: bool,
) -> String {
    let base = match stage {
        DraftMissingStage::BeforeRead => {
            format!(
                "Skipping draft {}: file disappeared before read.",
                path.display()
            )
        }
        DraftMissingStage::AfterCreate => {
            format!("Draft {} already removed after create.", path.display())
        }
    };
    if has_concurrency_evidence {
        format!("{base} Concurrent `mate issues` run detected.")
    } else {
        base
    }
}

pub(crate) fn cleanup_created_draft(path: &std::path::Path) -> std::io::Result<DraftCleanupOutcome> {
    trace!("cleanup_created_draft: attempt {}", path.display());
    match fs_err::remove_file(path) {
        Ok(()) => {
            trace!("cleanup_created_draft: removed {}", path.display());
            Ok(DraftCleanupOutcome::Removed)
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            trace!(
                "cleanup_created_draft: missing before remove {}",
                path.display()
            );
            Ok(DraftCleanupOutcome::Missing)
        }
        Err(e) => {
            trace!(
                "cleanup_created_draft: failed remove {} => {e}",
                path.display()
            );
            Err(e)
        }
    }
}

fn process_pending_issue_drafts(
    repo: &str,
) -> Result<(Vec<PendingIssueCreated>, Vec<PendingIssueFailed>)> {
    use std::io::ErrorKind;

    let base_dir = github::issue_repo_dir(repo);
    let new_dir = base_dir.join("new");
    trace!(
        "process_pending_issue_drafts: base {} new {}",
        base_dir.display(),
        new_dir.display()
    );
    if !new_dir.is_dir() {
        trace!("process_pending_issue_drafts: new dir missing");
        return Ok((Vec::new(), Vec::new()));
    }

    let failed_dir = base_dir.join("failed");
    trace!(
        "process_pending_issue_drafts: failed_dir {}",
        failed_dir.display()
    );
    fs_err::create_dir_all(&failed_dir)?;
    trace!("process_pending_issue_drafts: ensured failed_dir");

    let mut paths: Vec<std::path::PathBuf> = Vec::new();
    let entries = fs_err::read_dir(&new_dir)?;
    for entry in entries {
        let entry = match entry {
            Ok(value) => value,
            Err(e) => {
                trace!("process_pending_issue_drafts: read_dir entry error: {e}");
                continue;
            }
        };
        let raw_path = entry.path();
        trace!(
            "process_pending_issue_drafts: discovered (pre-filter) {}",
            raw_path.display()
        );
        if !entry.file_type().is_ok_and(|ft| ft.is_file()) {
            trace!(
                "process_pending_issue_drafts: filtered non-file {}",
                raw_path.display()
            );
            continue;
        }
        if raw_path.extension().is_none_or(|ext| ext != "md") {
            trace!(
                "process_pending_issue_drafts: filtered non-md {}",
                raw_path.display()
            );
            continue;
        }
        if entry.file_name().to_string_lossy() == "TEMPLATE.md" {
            trace!(
                "process_pending_issue_drafts: filtered TEMPLATE {}",
                raw_path.display()
            );
            continue;
        }
        trace!("process_pending_issue_drafts: kept {}", raw_path.display());
        paths.push(raw_path);
    }
    paths.sort();

    if paths.is_empty() {
        trace!("process_pending_issue_drafts: no drafts");
        return Ok((Vec::new(), Vec::new()));
    }
    trace!(
        "process_pending_issue_drafts: processing {} draft(s)",
        paths.len()
    );

    let mut existing_labels = github::sync_labels_set(repo)?;
    let mut existing_milestones = github::sync_milestones_set(repo)?;
    let mut created = Vec::new();
    let mut failed = Vec::new();

    for path in paths {
        eprintln!("[#38] processing draft path: {}", path.display());
        let original_name = path
            .file_name()
            .and_then(|name| name.to_str())
            .map(std::string::ToString::to_string)
            .unwrap_or_else(String::new);
        trace!("process_pending_issue_drafts: reading {}", path.display());
        let content = match fs_err::read_to_string(&path) {
            Ok(content) => {
                trace!("process_pending_issue_drafts: read ok {}", path.display());
                content
            }
            Err(e) => {
                eprintln!("[#38] read failed before read for {}: {e}", path.display());
                if e.kind() == ErrorKind::NotFound {
                    eprintln!(
                        "{}",
                        format_missing_draft_message(&path, DraftMissingStage::BeforeRead, false)
                    );
                    continue;
                }
                if let Err(move_err) = move_file(&path, &failed_dir.join(&original_name)) {
                    eprintln!("Failed {original_name}: move_to_failed_failed: {move_err}");
                }
                failed.push(PendingIssueFailed {
                    filename: original_name,
                    error: format!("read failed at {}: {e}", path.display()),
                });
                continue;
            }
        };

        eprintln!("[#38] read succeeded for {}", path.display());
        trace!("process_pending_issue_drafts: parse {}", path.display());
        let draft = match github::parse_new_issue(&content) {
            Ok(issue) => issue,
            Err(e) => {
                trace!(
                    "process_pending_issue_drafts: parse failed {} => {e}",
                    path.display()
                );
                if let Err(move_err) = move_file(&path, &failed_dir.join(&original_name)) {
                    trace!(
                        "process_pending_issue_drafts: move_file result {} -> {} failed: {move_err}",
                        path.display(),
                        failed_dir.join(&original_name).display()
                    );
                    eprintln!("Failed {original_name}: move_to_failed_failed: {move_err}");
                }
                failed.push(PendingIssueFailed {
                    filename: original_name,
                    error: format!("parse failed for {}: {e}", path.display()),
                });
                continue;
            }
        };

        let mut prep_error: Option<String> = None;
        for label in &draft.labels {
            if existing_labels.contains(label) {
                continue;
            }
            if let Err(e) = github::ensure_label_exists(repo, label) {
                prep_error = Some(format!("label '{label}' creation failed: {e}"));
                break;
            }
            existing_labels.insert(label.clone());
        }
        if prep_error.is_none()
            && let Some(milestone) = draft.milestone.as_deref()
            && !existing_milestones.contains(milestone)
        {
            if let Err(e) = github::ensure_milestone_exists(repo, milestone) {
                prep_error = Some(format!("milestone '{milestone}' creation failed: {e}"));
            } else {
                existing_milestones.insert(milestone.to_string());
            }
        }

        if let Some(error_message) = prep_error {
            trace!(
                "process_pending_issue_drafts: prep error for {} => {error_message}",
                path.display()
            );
            if let Err(move_err) = move_file(&path, &failed_dir.join(&original_name)) {
                eprintln!("Failed {original_name}: move_to_failed_failed: {move_err}");
            }
            failed.push(PendingIssueFailed {
                filename: original_name,
                error: error_message,
            });
            continue;
        }

        match github::create_issue(repo, &draft) {
            Ok((number, url)) => {
                eprintln!("[#38] created issue {} for {}", number, path.display());
                trace!(
                    "process_pending_issue_drafts: cleanup_created_draft before {}",
                    path.display()
                );
                match cleanup_created_draft(&path) {
                    Ok(DraftCleanupOutcome::Removed) => {}
                    Ok(DraftCleanupOutcome::Missing) => {
                        eprintln!(
                            "{}",
                            format_missing_draft_message(
                                &path,
                                DraftMissingStage::AfterCreate,
                                false
                            )
                        );
                    }
                    Err(e) => {
                        trace!(
                            "process_pending_issue_drafts: cleanup failed {} => {e}",
                            path.display()
                        );
                        if let Err(move_err) = move_file(&path, &failed_dir.join(&original_name)) {
                            eprintln!("Failed {original_name}: move_to_failed_failed: {move_err}");
                        }
                        failed.push(PendingIssueFailed {
                            filename: original_name,
                            error: format!("cleanup failed at {}: {e}", path.display()),
                        });
                        continue;
                    }
                }
                eprintln!("[#38] cleanup succeeded for {}", path.display());
                trace!(
                    "process_pending_issue_drafts: created and cleaned up {} as {}",
                    path.display(),
                    number
                );
                created.push(PendingIssueCreated {
                    number,
                    url,
                    title: draft.title,
                });
            }
            Err(e) => {
                trace!(
                    "process_pending_issue_drafts: create failed {} => {e}",
                    path.display()
                );
                if let Err(move_err) = move_file(&path, &failed_dir.join(&original_name)) {
                    eprintln!("Failed {original_name}: move_to_failed_failed: {move_err}");
                }
                failed.push(PendingIssueFailed {
                    filename: original_name,
                    error: format!("create failed for {}: {e}", path.display()),
                });
            }
        }
    }

    Ok((created, failed))
}

fn move_file(from: &std::path::Path, to: &std::path::Path) -> Result<()> {
    use std::io::ErrorKind;

    trace!("move_file: {} -> {}", from.display(), to.display());
    if to.exists() {
        trace!("move_file: dest exists before remove {}", to.display());
        fs_err::remove_file(to)?;
    }
    if let Err(e) = fs_err::rename(from, to) {
        if e.kind() == ErrorKind::NotFound {
            trace!("move_file: source not found {}", from.display());
            return Ok(());
        }
        trace!(
            "move_file: rename failed {} -> {}: {e}",
            from.display(),
            to.display()
        );
        return Err(e.into());
    }
    trace!(
        "move_file: rename ok {} -> {}",
        from.display(),
        to.display()
    );
    Ok(())
}
