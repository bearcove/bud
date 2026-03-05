use eyre::Result;

use crate::{client, paths, tmux, util};

pub(crate) fn compact_context() -> Result<()> {
    let summary = paths::read_stdin()?;
    let pane = std::env::var("TMUX_PANE")
        .map_err(|_| eyre::eyre!("TMUX_PANE not set — are you inside tmux?"))?;

    let list_output = std::process::Command::new("mate").arg("list").output()?;
    let task_list = if list_output.status.success() {
        let stdout = String::from_utf8_lossy(&list_output.stdout)
            .trim()
            .to_string();
        if stdout.is_empty() {
            "none".to_string()
        } else {
            stdout
        }
    } else {
        "none".to_string()
    };

    let prompt = format!(
        "/captain\nYou've just been compacted. Here is your context summary from before compaction:\n\n{summary}\n\nIn-flight tasks at time of compaction:\n{task_list}"
    );

    tmux::send_to_pane(&pane, "/clear")?;
    std::thread::sleep(std::time::Duration::from_millis(500));
    tmux::send_to_pane(&pane, &prompt)?;
    Ok(())
}

pub(crate) fn show_request(request_id: &str) -> Result<()> {
    client::validate_request_id(request_id)?;
    let session_name = paths::tmux_session_name()?;
    let path = paths::request_dir(&session_name).join(request_id);
    let meta = util::read_request_meta(&path)
        .ok_or_else(|| eyre::eyre!("No task with ID {request_id} found."))?;
    let content = util::read_request_content(&path)
        .ok_or_else(|| eyre::eyre!("Task {request_id} is missing request content."))?;
    eprintln!("Task {request_id}");
    eprintln!("Source: {}  Target: {}", meta.source_pane, meta.target_pane);
    eprintln!("Title: {}", meta.title.as_deref().unwrap_or("(none)"));
    eprintln!();
    eprintln!("{content}");
    Ok(())
}

pub(crate) fn spy_request(request_id: &str) -> Result<()> {
    client::validate_request_id(request_id)?;
    let session_name = paths::tmux_session_name()?;
    let path = paths::request_dir(&session_name).join(request_id);
    let meta = util::read_request_meta(&path)
        .ok_or_else(|| eyre::eyre!("No task with ID {request_id} found."))?;
    let pane_content = tmux::capture_pane(&meta.target_pane)?;
    eprintln!("Pane {}:\n{}", meta.target_pane, pane_content);
    Ok(())
}

#[cfg(test)]
pub(crate) fn format_captain_update_for_buddy(request_id: &str, message: &str) -> String {
    format!(
        "📌 Update from the captain on task {request_id}:\n\n\
         {message}\n\n\
         If you hit a decision point, want to share progress, or need clarification, send an update:\n\n\
         cat <<'MATEEOF' | mate update {request_id}\n\
         <your progress update here>\n\
         MATEEOF"
    )
}
