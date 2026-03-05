use eyre::Result;

pub(crate) fn watch_ci() -> Result<()> {
    let pane = std::env::var("TMUX_PANE")
        .map_err(|_| eyre::eyre!("TMUX_PANE not set — are you inside tmux?"))?;
    let exe = std::env::current_exe()?;
    let dev_null = std::fs::File::options()
        .read(true)
        .write(true)
        .open("/dev/null")?;

    std::process::Command::new(exe)
        .args(["_watch-inner", &pane])
        .stdin(std::process::Stdio::from(dev_null.try_clone()?))
        .stdout(std::process::Stdio::from(dev_null.try_clone()?))
        .stderr(std::process::Stdio::from(dev_null))
        .spawn()?;

    eprintln!("Started CI watcher in background for {pane}.");
    Ok(())
}

pub(crate) fn watch_ci_inner(pane: &str) -> Result<()> {
    match run_watch_ci_inner(pane) {
        Ok(()) => Ok(()),
        Err(err) => {
            let _ = crate::tmux::send_to_pane(pane, &format!("❌ CI watch failed: {err}"));
            Ok(())
        }
    }
}

fn run_watch_ci_inner(pane: &str) -> Result<()> {
    let branch = current_branch()?;
    let run_id = poll_latest_run_id(&branch, std::time::Duration::from_secs(30))?
        .ok_or_else(|| eyre::eyre!("no CI run found for branch `{branch}` within 30s"))?;

    let watch_status = std::process::Command::new("gh")
        .args(["run", "watch", &run_id, "--exit-status"])
        .status()?;

    if watch_status.success() {
        crate::tmux::send_to_pane(pane, "✅ CI passed.")?;
        return Ok(());
    }

    let failed_log_output = std::process::Command::new("gh")
        .args(["run", "view", &run_id, "--log-failed"])
        .output()?;

    let mut summary_lines: Vec<String> = String::from_utf8_lossy(&failed_log_output.stdout)
        .lines()
        .take(50)
        .map(ToString::to_string)
        .collect();

    if summary_lines.is_empty() {
        summary_lines = String::from_utf8_lossy(&failed_log_output.stderr)
            .lines()
            .take(50)
            .map(ToString::to_string)
            .collect();
    }

    let summary = if summary_lines.is_empty() {
        "No failed log output available.".to_string()
    } else {
        summary_lines.join("\n")
    };

    let message = format!("❌ CI failed:\n```\n{summary}\n```");
    crate::tmux::send_to_pane(pane, &message)?;
    Ok(())
}

fn current_branch() -> Result<String> {
    let output = std::process::Command::new("git")
        .args(["rev-parse", "--abbrev-ref", "HEAD"])
        .output()?;
    if !output.status.success() {
        return Err(eyre::eyre!("failed to determine current git branch"));
    }
    let branch = String::from_utf8(output.stdout)?.trim().to_string();
    if branch.is_empty() {
        return Err(eyre::eyre!("current git branch is empty"));
    }
    Ok(branch)
}

fn poll_latest_run_id(branch: &str, timeout: std::time::Duration) -> Result<Option<String>> {
    let started_at = std::time::Instant::now();
    loop {
        if let Some(run_id) = latest_run_id(branch)? {
            return Ok(Some(run_id));
        }
        if started_at.elapsed() >= timeout {
            return Ok(None);
        }
        std::thread::sleep(std::time::Duration::from_secs(5));
    }
}

fn latest_run_id(branch: &str) -> Result<Option<String>> {
    let output = std::process::Command::new("gh")
        .args([
            "run",
            "list",
            "--branch",
            branch,
            "--limit",
            "1",
            "--json",
            "databaseId,status",
            "--jq",
            ".[0].databaseId",
        ])
        .output()?;
    if !output.status.success() {
        return Err(eyre::eyre!("failed to list CI runs with gh"));
    }
    let run_id = String::from_utf8(output.stdout)?.trim().to_string();
    if run_id.is_empty() || run_id == "null" {
        return Ok(None);
    }
    Ok(Some(run_id))
}
