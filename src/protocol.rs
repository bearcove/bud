use facet::Facet;

#[derive(Debug, Clone, Facet)]
pub struct AssignRequest {
    /// The $TMUX_PANE of the requesting agent
    pub source_pane: String,
    /// The task content (read from stdin by the CLI)
    pub content: String,
    /// Whether to send /clear to the worker before the task
    pub clear: bool,
}

#[roam::service]
pub trait Coop {
    /// Assign a task to the worker agent. Returns the request ID.
    async fn assign(&self, req: AssignRequest) -> Result<String, String>;
}
