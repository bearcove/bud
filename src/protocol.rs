use facet::Facet;

#[derive(Debug, Clone, Facet)]
pub struct AssignRequest {
    /// The $TMUX_PANE of the requesting agent
    pub source_pane: String,
    /// Path to the file containing the task description
    pub task_file: String,
}

#[roam::service]
pub trait Coop {
    /// Assign a task to the worker agent. Returns the request ID.
    async fn assign(&self, req: AssignRequest) -> String;
}
