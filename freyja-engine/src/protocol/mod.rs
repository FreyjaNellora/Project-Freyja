//! Freyja Protocol: engine-UI communication.
//!
//! Text-based protocol over stdin/stdout for four-player chess.
//! Similar to UCI but with 4-vector scores and 4-player game state.

pub mod commands;
pub mod logfile;
pub mod notation;
pub mod options;
pub mod output;
pub mod parse;

use std::io::{BufRead, Write};

use crate::eval::BootstrapEvaluator;
use crate::game_state::GameState;
use crate::hybrid::{HybridConfig, HybridSearcher};
use crate::move_gen::Move;
use crate::search::{SearchLimits, Searcher};

use self::commands::Command;
use self::logfile::LogFile;
use self::notation::parse_move_str;
use self::options::{EngineOptions, SetOptionResult, apply_option};
use self::output::{
    format_bestmove, format_error, format_info, format_info_string, format_nextturn,
};
use self::parse::parse_command;

/// Protocol version header.
const PROTOCOL_HEADER: &str = "freyja v1.0 maxn-beam-mcts";

/// The protocol handler. Generic over output writer for testability.
pub struct Protocol<W: Write> {
    output: W,
    game_state: GameState,
    options: EngineOptions,
    logfile: LogFile,
    position_set: bool,
    ply_count: u32,
}

impl<W: Write> Protocol<W> {
    /// Create a new protocol handler writing to the given output.
    pub fn new(output: W) -> Self {
        Self {
            output,
            game_state: GameState::new_standard_ffa(),
            options: EngineOptions::default(),
            logfile: LogFile::new(),
            position_set: false,
            ply_count: 0,
        }
    }

    /// Run the protocol loop, reading commands from `input` until EOF or `quit`.
    pub fn run(&mut self, input: impl BufRead) {
        self.send(PROTOCOL_HEADER);

        for line in input.lines() {
            let line = match line {
                Ok(l) => l,
                Err(_) => break,
            };

            self.logfile.log_incoming(&line);
            tracing::debug!("> {}", line);

            let cmd = match parse_command(&line) {
                Some(cmd) => cmd,
                None => continue, // empty line
            };

            match cmd {
                Command::Freyja => self.send(PROTOCOL_HEADER),
                Command::IsReady => self.send("readyok"),
                Command::Position { fen, moves } => self.handle_position(fen, moves),
                Command::Go(params) => self.handle_go(params),
                Command::Stop => { /* no-op: search is synchronous in Stage 4 */ }
                Command::Debug => self.handle_debug(),
                Command::Quit => break,
                Command::SetOption { name, value } => self.handle_setoption(&name, &value),
                Command::Unknown(cmd) => {
                    let msg = format_error(&format!("unknown command '{cmd}'"));
                    self.send(&msg);
                }
            }
        }
    }

    /// Send a line to the output, log it, and trace it.
    fn send(&mut self, line: &str) {
        let _ = writeln!(self.output, "{line}");
        let _ = self.output.flush();
        self.logfile.log_outgoing(line);
        tracing::debug!("< {}", line);
    }

    /// Handle `d` / `debug` command — dump current position as FEN4.
    fn handle_debug(&mut self) {
        let fen = self.game_state.board().to_fen4();
        self.send(&format!("fen4 {fen}"));
    }

    /// Handle `position` command.
    fn handle_position(&mut self, fen: Option<String>, moves: Vec<String>) {
        // Set up initial position
        match fen {
            None => {
                self.game_state = GameState::new_standard_ffa();
            }
            Some(fen_str) => match crate::board::Board::from_fen4(&fen_str) {
                Ok(board) => {
                    self.game_state = GameState::new(board);
                }
                Err(e) => {
                    let msg = format_error(&format!("invalid FEN4: {e:?}"));
                    self.send(&msg);
                    return;
                }
            },
        }

        self.ply_count = 0;
        self.position_set = true;

        // Apply moves silently (no per-move events during replay).
        for move_str in &moves {
            let legal = self.game_state.legal_moves();
            match parse_move_str(move_str, &legal) {
                Ok(mv) => {
                    self.game_state.apply_move(mv);
                    self.ply_count += 1;
                }
                Err(e) => {
                    let msg = format_error(&format!("in position moves: {e}"));
                    self.send(&msg);
                    return;
                }
            }
        }

        // Emit current turn so UI knows whose move it is after replay
        if !moves.is_empty() && !self.game_state.is_game_over() {
            let msg = format_nextturn(self.game_state.current_player());
            self.send(&msg);
        }
    }

    /// Handle `go` command — run Max^n search.
    fn handle_go(&mut self, params: commands::GoParams) {
        if !self.position_set {
            let msg = format_error("no position set");
            self.send(&msg);
            return;
        }

        if self.game_state.is_game_over() {
            let msg = format_info_string("game is over");
            self.send(&msg);
            let bm = format_bestmove(None);
            self.send(&bm);
            return;
        }

        // Check MaxRounds
        if let Some(max) = self.options.max_rounds
            && self.ply_count / 4 >= max
        {
            let msg = format_info_string("game stopped: max rounds reached");
            self.send(&msg);
            let bm = format_bestmove(None);
            self.send(&bm);
            return;
        }

        let legal = self.game_state.legal_moves();

        if legal.is_empty() {
            let msg = format_info_string("no legal moves");
            self.send(&msg);
            let bm = format_bestmove(None);
            self.send(&bm);
            return;
        }

        // Run real search — default to 5s time budget if no constraints given
        // (quiescence adds overhead at leaf nodes, need more time for depth 4+)
        let no_constraints = params.depth.is_none()
            && params.nodes.is_none()
            && params.movetime.is_none()
            && !params.infinite;
        // Wire MaxNodes from options if not specified in go command
        let effective_max_nodes = params.nodes.or(if self.options.max_nodes > 0 {
            Some(self.options.max_nodes)
        } else {
            None
        });
        let limits = SearchLimits {
            max_depth: params.depth,
            max_nodes: effective_max_nodes,
            max_time_ms: if no_constraints {
                Some(5000)
            } else {
                params.movetime
            },
            infinite: params.infinite,
            game_ply: self.ply_count,
        };

        let hybrid_config = HybridConfig {
            maxn_config: self.options.search_config(),
            mcts_config: self.options.mcts_config(),
            time_split_ratio: self.options.time_split_ratio,
            phase_cutover_ply: self.options.phase_cutover_ply,
            ..HybridConfig::default()
        };
        let eval = BootstrapEvaluator::with_zone_weights(crate::eval::ZoneWeights {
            territory: self.options.territory_weight,
            influence: self.options.influence_weight,
            tension: self.options.tension_weight,
            swarm: self.options.swarm_weight,
        });
        let mut searcher = HybridSearcher::new(eval, hybrid_config);
        let start = std::time::Instant::now();
        let result = searcher.search(&mut self.game_state, &limits);
        let elapsed_ms = start.elapsed().as_millis() as u64;

        // Send info line
        let nps = if elapsed_ms > 0 {
            Some(result.nodes * 1000 / elapsed_ms)
        } else {
            None
        };
        let pv_slice: &[Move] = &result.pv;
        let qnodes_opt = if result.qnodes > 0 {
            Some(result.qnodes)
        } else {
            None
        };
        let tt_rate = if result.tt_hit_rate > 0.0 {
            Some(result.tt_hit_rate)
        } else {
            None
        };
        let killer_rate = if result.killer_hit_rate > 0.0 {
            Some(result.killer_hit_rate)
        } else {
            None
        };
        let info = format_info(
            Some(result.depth),
            Some(result.scores),
            Some(result.nodes),
            qnodes_opt,
            nps,
            Some(pv_slice),
            tt_rate,
            killer_rate,
        );
        self.send(&info);

        let bm = format_bestmove(result.best_move);
        self.send(&bm);
    }

    /// Handle `setoption` command.
    fn handle_setoption(&mut self, name: &str, value: &str) {
        match apply_option(&mut self.options, name, value) {
            SetOptionResult::Ok => {}
            SetOptionResult::EnableLogFile(path) => {
                if let Err(e) = self.logfile.enable(&path) {
                    let msg = format_error(&format!("failed to open log file '{path}': {e}"));
                    self.send(&msg);
                }
            }
            SetOptionResult::DisableLogFile => {
                self.logfile.disable();
            }
            SetOptionResult::UnknownOption(name) => {
                let msg = format_error(&format!("unknown option '{name}'"));
                self.send(&msg);
            }
            SetOptionResult::InvalidValue(detail) => {
                let msg = format_error(&detail);
                self.send(&msg);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    fn run_protocol(input: &str) -> String {
        let reader = Cursor::new(input.as_bytes().to_vec());
        let mut output = Vec::new();
        {
            let mut proto = Protocol::new(&mut output);
            proto.run(reader);
        }
        String::from_utf8(output).unwrap()
    }

    #[test]
    fn test_startup_header() {
        let output = run_protocol("quit\n");
        assert!(output.starts_with("freyja v1.0 maxn-beam-mcts\n"));
    }

    #[test]
    fn test_isready_readyok() {
        let output = run_protocol("isready\nquit\n");
        assert!(output.contains("readyok\n"));
    }

    #[test]
    fn test_freyja_identification() {
        let output = run_protocol("freyja\nquit\n");
        // Should have header on startup + header in response to freyja command
        let count = output.matches("freyja v1.0 maxn-beam-mcts").count();
        assert_eq!(count, 2);
    }

    #[test]
    fn test_unknown_command() {
        let output = run_protocol("bogus\nquit\n");
        assert!(output.contains("info string error: unknown command 'bogus'"));
    }

    #[test]
    fn test_go_without_position() {
        let output = run_protocol("go depth 1\nquit\n");
        assert!(output.contains("info string error: no position set"));
    }

    #[test]
    fn test_position_startpos_go() {
        let output = run_protocol("position startpos\ngo depth 1\nquit\n");
        assert!(output.contains("bestmove "));
        // Should NOT contain "(none)" — there are legal moves from startpos
        assert!(!output.contains("bestmove (none)"));
    }

    #[test]
    fn test_position_with_moves() {
        let output = run_protocol("position startpos moves d2d4\ngo depth 1\nquit\n");
        // After Red plays d2d4, Blue should have a bestmove
        assert!(output.contains("bestmove "));
        assert!(!output.contains("bestmove (none)"));
    }

    #[test]
    fn test_info_contains_scores() {
        let output = run_protocol("position startpos\ngo depth 1\nquit\n");
        assert!(output.contains("score red"));
        assert!(output.contains("blue"));
        assert!(output.contains("yellow"));
        assert!(output.contains("green"));
    }

    #[test]
    fn test_empty_lines_ignored() {
        let output = run_protocol("\n\n  \nisready\n\nquit\n");
        assert!(output.contains("readyok"));
    }

    #[test]
    fn test_extra_whitespace() {
        let output = run_protocol("  isready  \nquit\n");
        assert!(output.contains("readyok"));
    }

    #[test]
    fn test_position_invalid_fen() {
        let output = run_protocol("position fen4 bad_fen\nquit\n");
        assert!(output.contains("info string error: invalid FEN4"));
    }

    #[test]
    fn test_position_invalid_move() {
        let output = run_protocol("position startpos moves z9z9\nquit\n");
        assert!(output.contains("info string error:"));
    }

    #[test]
    fn test_setoption_max_rounds() {
        let output =
            run_protocol("setoption name MaxRounds value 0\nposition startpos\ngo depth 1\nquit\n");
        // MaxRounds 0 → unlimited, so bestmove should be normal
        // (ply_count starts at 0, 0/4 = 0, 0 >= 0 is never checked because max_rounds is None)
        assert!(output.contains("bestmove "));
    }

    #[test]
    fn test_max_rounds_stops_game() {
        // Find 4 legal moves programmatically: one per player from startpos.
        use crate::game_state::GameState;
        let mut gs = GameState::new_standard_ffa();
        let mut move_strs = Vec::new();
        for _ in 0..4 {
            let legal = gs.legal_moves();
            let mv = legal[0];
            move_strs.push(format!("{mv}"));
            gs.apply_move(mv);
        }

        let input = format!(
            "setoption name MaxRounds value 1\nposition startpos moves {}\ngo depth 1\nquit\n",
            move_strs.join(" ")
        );
        let output = run_protocol(&input);
        assert!(
            output.contains("max rounds reached"),
            "Expected 'max rounds reached' in output:\n{output}"
        );
    }

    #[test]
    fn test_unknown_option() {
        let output = run_protocol("setoption name Bogus value 42\nquit\n");
        assert!(output.contains("unknown option 'Bogus'"));
    }

    #[test]
    fn test_position_replay_emits_final_state() {
        // Position replay emits a single nextturn for final state (not per-move)
        let output = run_protocol("position startpos moves d2d4\nquit\n");
        // Should have exactly one nextturn (the final state after d2d4)
        let nextturn_count = output.matches("info string nextturn").count();
        assert_eq!(
            nextturn_count, 1,
            "Expected exactly 1 nextturn, got {nextturn_count}"
        );
        assert!(
            output.contains("info string nextturn Blue"),
            "After Red's d2d4, Blue should be next"
        );
    }
}
