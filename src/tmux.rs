use eyre::Result;
use std::process::Command;

pub struct Pane {
    pub id: String,
    pub title: String,
    pub command: String,
}

/// List all tmux panes in the current session.
pub fn list_panes() -> Result<Vec<Pane>> {
    let output = Command::new("tmux")
        .args(["list-panes", "-a", "-F", "#{pane_id}\t#{pane_title}\t#{pane_current_command}"])
        .output()?;

    if !output.status.success() {
        return Err(eyre::eyre!("tmux list-panes failed"));
    }

    let stdout = String::from_utf8(output.stdout)?;
    let panes = stdout
        .lines()
        .filter_map(|line| {
            let mut parts = line.splitn(3, '\t');
            let id = parts.next()?.to_string();
            let title = parts.next()?.to_string();
            let command = parts.next()?.to_string();
            Some(Pane { id, title, command })
        })
        .collect();

    Ok(panes)
}

/// Send text to a tmux pane. Uses -l for literal text, then C-m to submit.
pub fn send_to_pane(pane_id: &str, text: &str) -> Result<()> {
    // Send the text literally (no key interpretation)
    let status = Command::new("tmux")
        .args(["send-keys", "-t", pane_id, "-l", text])
        .status()?;
    if !status.success() {
        return Err(eyre::eyre!("tmux send-keys (text) failed for pane {pane_id}"));
    }

    std::thread::sleep(std::time::Duration::from_millis(50));

    // Submit with C-m (carriage return) — "Enter" alone doesn't work in some apps
    let status = Command::new("tmux")
        .args(["send-keys", "-t", pane_id, "C-m"])
        .status()?;
    if !status.success() {
        return Err(eyre::eyre!("tmux send-keys (C-m) failed for pane {pane_id}"));
    }

    Ok(())
}

/// Find a pane that is NOT the given pane_id (i.e., find the "other" agent).
pub fn find_other_pane(my_pane_id: &str) -> Result<Pane> {
    let panes = list_panes()?;
    panes
        .into_iter()
        .find(|p| p.id != my_pane_id)
        .ok_or_else(|| eyre::eyre!("no other tmux pane found"))
}
