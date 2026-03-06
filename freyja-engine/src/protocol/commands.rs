//! Protocol command types.

/// Parameters for the `go` command.
#[derive(Debug, Clone, Default)]
pub struct GoParams {
    /// Maximum search depth in plies.
    pub depth: Option<u32>,
    /// Maximum nodes to search.
    pub nodes: Option<u64>,
    /// Maximum time in milliseconds.
    pub movetime: Option<u64>,
    /// Search until `stop` is received.
    pub infinite: bool,
}

/// A parsed protocol command.
#[derive(Debug)]
pub enum Command {
    /// Engine identification.
    Freyja,
    /// Readiness ping.
    IsReady,
    /// Set board position, optionally with moves applied.
    Position {
        /// FEN4 string, or None for startpos.
        fen: Option<String>,
        /// Move strings to apply after setting position.
        moves: Vec<String>,
    },
    /// Begin search with given parameters.
    Go(GoParams),
    /// Halt search (no-op while search is synchronous).
    Stop,
    /// Exit the engine.
    Quit,
    /// Set an engine option.
    SetOption {
        /// Option name.
        name: String,
        /// Option value.
        value: String,
    },
    /// Unrecognized command (kept for error reporting).
    Unknown(String),
}
