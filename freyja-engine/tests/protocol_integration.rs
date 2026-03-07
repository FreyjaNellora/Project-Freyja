//! Protocol integration tests — full session simulations.
//!
//! Tests exercise the protocol end-to-end using injected I/O buffers.
//! No subprocess spawning; Protocol<Vec<u8>> runs in-process.

use std::io::Cursor;

use freyja_engine::game_state::GameState;
use freyja_engine::protocol::Protocol;

/// Run a protocol session with the given input, return the output as a string.
fn run_session(input: &str) -> String {
    let reader = Cursor::new(input.as_bytes().to_vec());
    let mut output = Vec::new();
    {
        let mut proto = Protocol::new(&mut output);
        proto.run(reader);
    }
    String::from_utf8(output).unwrap()
}

// ─── Startup & Identification ───────────────────────────────────────────────

#[test]
fn test_protocol_header_on_startup() {
    let output = run_session("quit\n");
    let first_line = output.lines().next().unwrap();
    assert_eq!(first_line, "freyja v1.0 maxn-beam-mcts");
}

#[test]
fn test_freyja_command_returns_header() {
    let output = run_session("freyja\nquit\n");
    let headers: Vec<_> = output
        .lines()
        .filter(|l| *l == "freyja v1.0 maxn-beam-mcts")
        .collect();
    assert_eq!(headers.len(), 2, "startup header + freyja response");
}

// ─── isready / readyok ─────────────────────────────────────────────────────

#[test]
fn test_isready_readyok_roundtrip() {
    let output = run_session("isready\nquit\n");
    assert!(output.contains("readyok\n"));
}

#[test]
fn test_multiple_isready() {
    let output = run_session("isready\nisready\nisready\nquit\n");
    let count = output.matches("readyok").count();
    assert_eq!(count, 3);
}

// ─── Position + Go → Bestmove ───────────────────────────────────────────────

#[test]
fn test_position_startpos_go_returns_bestmove() {
    let output = run_session("position startpos\ngo depth 1\nquit\n");
    let bestmove_line = output.lines().find(|l| l.starts_with("bestmove "));
    assert!(bestmove_line.is_some(), "expected bestmove in output");
    let bm = bestmove_line.unwrap();
    assert_ne!(bm, "bestmove (none)", "startpos should have legal moves");
}

#[test]
fn test_position_with_moves_then_go() {
    // Get first move for Red
    let mut gs = GameState::new_standard_ffa();
    let first_move = gs.legal_moves()[0];
    let move_str = format!("{first_move}");

    let input = format!("position startpos moves {move_str}\ngo depth 1\nquit\n");
    let output = run_session(&input);

    // After Red moves, Blue should get a bestmove
    assert!(output.contains("bestmove "));
    assert!(!output.contains("bestmove (none)"));
}

#[test]
fn test_go_without_position_errors() {
    let output = run_session("go depth 1\nquit\n");
    assert!(output.contains("error: no position set"));
}

// ─── Info Output Format ─────────────────────────────────────────────────────

#[test]
fn test_info_contains_4vector_scores() {
    let output = run_session("position startpos\ngo depth 1\nquit\n");
    let info_line = output
        .lines()
        .find(|l| l.starts_with("info depth"))
        .unwrap();
    assert!(info_line.contains("score red"));
    assert!(info_line.contains("blue"));
    assert!(info_line.contains("yellow"));
    assert!(info_line.contains("green"));
}

#[test]
fn test_info_contains_pv() {
    let output = run_session("position startpos\ngo depth 1\nquit\n");
    let info_line = output
        .lines()
        .find(|l| l.starts_with("info depth"))
        .unwrap();
    assert!(
        info_line.contains("pv "),
        "info should contain principal variation"
    );
}

// ─── Unknown Commands ───────────────────────────────────────────────────────

#[test]
fn test_unknown_command_produces_error() {
    let output = run_session("bogus\nquit\n");
    assert!(output.contains("info string error: unknown command 'bogus'"));
}

#[test]
fn test_unknown_command_does_not_crash() {
    // Multiple unknown commands in sequence
    let output = run_session("foo\nbar\nbaz\nisready\nquit\n");
    assert!(
        output.contains("readyok"),
        "engine still functional after unknown commands"
    );
}

// ─── Whitespace Robustness ──────────────────────────────────────────────────

#[test]
fn test_empty_lines_ignored() {
    let output = run_session("\n\n  \n\tisready\n\nquit\n");
    assert!(output.contains("readyok"));
}

#[test]
fn test_trailing_whitespace() {
    let output = run_session("isready   \nquit\n");
    assert!(output.contains("readyok"));
}

#[test]
fn test_carriage_return_line_endings() {
    let output = run_session("isready\r\nquit\r\n");
    assert!(output.contains("readyok"));
}

// ─── setoption ──────────────────────────────────────────────────────────────

#[test]
fn test_setoption_unknown_option_error() {
    let output = run_session("setoption name Bogus value 42\nquit\n");
    assert!(output.contains("unknown option 'Bogus'"));
}

#[test]
fn test_setoption_beam_width() {
    // Should not error
    let output = run_session("setoption name BeamWidth value 32\nisready\nquit\n");
    assert!(output.contains("readyok"));
    assert!(!output.contains("error"));
}

#[test]
fn test_setoption_game_mode() {
    let output = run_session("setoption name GameMode value FreeForAll\nisready\nquit\n");
    assert!(output.contains("readyok"));
    assert!(!output.contains("error"));
}

// ─── LogFile Toggle ─────────────────────────────────────────────────────────

#[test]
fn test_logfile_enable_disable() {
    let dir = std::env::temp_dir();
    let path = dir.join("freyja_integration_logfile.log");
    let path_str = path.to_str().unwrap();

    let input = format!(
        "setoption name LogFile value {path_str}\n\
         isready\n\
         setoption name LogFile value none\n\
         quit\n"
    );
    let output = run_session(&input);
    assert!(output.contains("readyok"));

    // Verify log file was written
    let contents = std::fs::read_to_string(&path).unwrap();
    assert!(
        contents.contains("] > isready"),
        "log should contain incoming command"
    );
    assert!(
        contents.contains("] < readyok"),
        "log should contain outgoing response"
    );

    let _ = std::fs::remove_file(&path);
}

// ─── MaxRounds Auto-Stop ────────────────────────────────────────────────────

#[test]
fn test_max_rounds_auto_stop() {
    // Get 4 legal moves (one per player) from startpos
    let mut gs = GameState::new_standard_ffa();
    let mut move_strs = Vec::new();
    for _ in 0..4 {
        let legal = gs.legal_moves();
        move_strs.push(format!("{}", legal[0]));
        gs.apply_move(legal[0]);
    }

    let input = format!(
        "setoption name MaxRounds value 1\n\
         position startpos moves {}\n\
         go depth 1\n\
         quit\n",
        move_strs.join(" ")
    );
    let output = run_session(&input);
    assert!(output.contains("max rounds reached"));
    assert!(output.contains("bestmove (none)"));
}

#[test]
fn test_max_rounds_does_not_trigger_under_limit() {
    // Only 2 plies, MaxRounds=1 needs 4 plies to trigger
    let mut gs = GameState::new_standard_ffa();
    let mut move_strs = Vec::new();
    for _ in 0..2 {
        let legal = gs.legal_moves();
        move_strs.push(format!("{}", legal[0]));
        gs.apply_move(legal[0]);
    }

    let input = format!(
        "setoption name MaxRounds value 1\n\
         position startpos moves {}\n\
         go depth 1\n\
         quit\n",
        move_strs.join(" ")
    );
    let output = run_session(&input);
    assert!(!output.contains("max rounds reached"));
    assert!(output.contains("bestmove "));
    assert!(!output.contains("bestmove (none)"));
}

// ─── 4PC Orientation: Moves From All 4 Players ─────────────────────────────

#[test]
fn test_all_four_players_can_move_via_protocol() {
    // Apply one move per player, verify no errors and nextturn events
    let mut gs = GameState::new_standard_ffa();
    let mut move_strs = Vec::new();
    for _ in 0..4 {
        let legal = gs.legal_moves();
        move_strs.push(format!("{}", legal[0]));
        gs.apply_move(legal[0]);
    }

    let input = format!(
        "position startpos moves {}\ngo depth 1\nquit\n",
        move_strs.join(" ")
    );
    let output = run_session(&input);
    // Should not contain any errors
    assert!(
        !output.contains("error:"),
        "no errors applying 4-player moves"
    );
    // Should have nextturn events for Blue, Yellow, Green after Red/Blue/Yellow moves
    assert!(output.contains("nextturn Blue"));
    assert!(output.contains("nextturn Yellow"));
    assert!(output.contains("nextturn Green"));
}

// ─── Game Over Handling ─────────────────────────────────────────────────────

#[test]
fn test_position_fen4_roundtrip() {
    // Get the FEN4 of starting position, set it via protocol, verify go works
    let board = freyja_engine::board::Board::starting_position();
    let fen = board.to_fen4();

    let input = format!("position fen4 {fen}\ngo depth 1\nquit\n");
    let output = run_session(&input);
    assert!(output.contains("bestmove "));
    assert!(!output.contains("error:"));
}

// ─── Protocol Conformance: Every Message Format ─────────────────────────────

#[test]
fn test_all_output_lines_are_valid_protocol() {
    let output = run_session("position startpos\ngo depth 1\nsetoption name Bogus value x\nquit\n");

    for line in output.lines() {
        let valid = line.starts_with("freyja ")
            || line == "readyok"
            || line.starts_with("bestmove ")
            || line.starts_with("info ")
            || line.is_empty();
        assert!(valid, "unexpected protocol output line: '{line}'");
    }
}

// ─── Invalid FEN4 ───────────────────────────────────────────────────────────

#[test]
fn test_invalid_fen4_error() {
    let output = run_session("position fen4 garbage_fen\nquit\n");
    assert!(output.contains("info string error: invalid FEN4"));
}

// ─── Stop Command ───────────────────────────────────────────────────────────

#[test]
fn test_stop_command_no_crash() {
    // stop is a no-op in Stage 4 (synchronous search)
    let output = run_session("stop\nisready\nquit\n");
    assert!(output.contains("readyok"));
}
