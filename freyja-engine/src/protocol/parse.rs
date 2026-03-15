//! Command tokenizer and parser.

use super::commands::{Command, GoParams};

/// Parse a single protocol input line into a Command.
///
/// Returns `None` for empty/whitespace-only lines (should be silently ignored).
pub fn parse_command(line: &str) -> Option<Command> {
    let line = line.trim();
    if line.is_empty() {
        return None;
    }

    let mut tokens = line.split_whitespace();
    let first = tokens.next()?;

    Some(match first {
        "freyja" => Command::Freyja,
        "isready" => Command::IsReady,
        "stop" => Command::Stop,
        "quit" => Command::Quit,
        "d" | "debug" => Command::Debug,
        "position" => parse_position(&mut tokens),
        "go" => Command::Go(parse_go(&mut tokens)),
        "setoption" => parse_setoption(&mut tokens),
        _ => Command::Unknown(first.to_string()),
    })
}

/// Parse `position [startpos | fen4 <fen>] [moves <m1> <m2> ...]`
fn parse_position<'a>(tokens: &mut impl Iterator<Item = &'a str>) -> Command {
    let mut fen: Option<String> = None;
    let mut moves: Vec<String> = Vec::new();
    let mut reading_moves = false;

    match tokens.next() {
        Some("startpos") => {
            // fen stays None → use starting position
        }
        Some("fen4") => {
            // Collect all tokens until "moves" keyword as the FEN4 string.
            let mut fen_parts: Vec<&str> = Vec::new();
            for tok in tokens.by_ref() {
                if tok == "moves" {
                    reading_moves = true;
                    break;
                }
                fen_parts.push(tok);
            }
            fen = Some(fen_parts.join(" "));
        }
        _ => {
            // Malformed position command — treat as startpos
        }
    }

    // If we haven't hit "moves" yet, look for it
    if !reading_moves {
        for tok in tokens.by_ref() {
            if tok == "moves" {
                reading_moves = true;
                break;
            }
        }
    }

    // Collect move strings
    if reading_moves {
        for tok in tokens {
            moves.push(tok.to_string());
        }
    }

    Command::Position { fen, moves }
}

/// Parse `go [depth D] [nodes N] [movetime MS] [infinite]`
fn parse_go<'a>(tokens: &mut impl Iterator<Item = &'a str>) -> GoParams {
    let mut params = GoParams::default();
    let mut tokens = tokens.peekable();

    while let Some(tok) = tokens.next() {
        match tok {
            "depth" => {
                if let Some(v) = tokens.next().and_then(|s| s.parse().ok()) {
                    params.depth = Some(v);
                }
            }
            "nodes" => {
                if let Some(v) = tokens.next().and_then(|s| s.parse().ok()) {
                    params.nodes = Some(v);
                }
            }
            "movetime" => {
                if let Some(v) = tokens.next().and_then(|s| s.parse().ok()) {
                    params.movetime = Some(v);
                }
            }
            "infinite" => {
                params.infinite = true;
            }
            _ => {
                // Ignore unknown go parameters gracefully
            }
        }
    }

    params
}

/// Parse `setoption name <name> value <value>`
fn parse_setoption<'a>(tokens: &mut impl Iterator<Item = &'a str>) -> Command {
    let mut name = String::new();
    let mut value = String::new();
    let mut reading_value = false;

    for tok in tokens {
        match tok {
            "name" if !reading_value => {
                // Start collecting name tokens (skip the "name" keyword itself)
            }
            "value" if !reading_value => {
                reading_value = true;
            }
            _ => {
                if reading_value {
                    if !value.is_empty() {
                        value.push(' ');
                    }
                    value.push_str(tok);
                } else {
                    if !name.is_empty() {
                        name.push(' ');
                    }
                    name.push_str(tok);
                }
            }
        }
    }

    Command::SetOption { name, value }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_empty_line() {
        assert!(parse_command("").is_none());
        assert!(parse_command("   ").is_none());
        assert!(parse_command("\n").is_none());
        assert!(parse_command("\r\n").is_none());
    }

    #[test]
    fn test_simple_commands() {
        assert!(matches!(parse_command("freyja"), Some(Command::Freyja)));
        assert!(matches!(parse_command("isready"), Some(Command::IsReady)));
        assert!(matches!(parse_command("stop"), Some(Command::Stop)));
        assert!(matches!(parse_command("quit"), Some(Command::Quit)));
    }

    #[test]
    fn test_whitespace_handling() {
        assert!(matches!(parse_command("  freyja  "), Some(Command::Freyja)));
        assert!(matches!(
            parse_command("\tisready\t"),
            Some(Command::IsReady)
        ));
        assert!(matches!(parse_command("  quit  \r\n"), Some(Command::Quit)));
    }

    #[test]
    fn test_unknown_command() {
        if let Some(Command::Unknown(cmd)) = parse_command("bogus") {
            assert_eq!(cmd, "bogus");
        } else {
            panic!("expected Unknown");
        }
    }

    #[test]
    fn test_position_startpos() {
        if let Some(Command::Position { fen, moves }) = parse_command("position startpos") {
            assert!(fen.is_none());
            assert!(moves.is_empty());
        } else {
            panic!("expected Position");
        }
    }

    #[test]
    fn test_position_startpos_with_moves() {
        if let Some(Command::Position { fen, moves }) =
            parse_command("position startpos moves d2d4 d13d11")
        {
            assert!(fen.is_none());
            assert_eq!(moves, vec!["d2d4", "d13d11"]);
        } else {
            panic!("expected Position");
        }
    }

    #[test]
    fn test_position_fen4() {
        let cmd = "position fen4 some/fen/string r KkBb - - moves e2e4";
        if let Some(Command::Position { fen, moves }) = parse_command(cmd) {
            assert_eq!(fen.as_deref(), Some("some/fen/string r KkBb - -"));
            assert_eq!(moves, vec!["e2e4"]);
        } else {
            panic!("expected Position");
        }
    }

    #[test]
    fn test_position_fen4_no_moves() {
        let cmd = "position fen4 some/fen/string r KkBb - -";
        if let Some(Command::Position { fen, moves }) = parse_command(cmd) {
            assert_eq!(fen.as_deref(), Some("some/fen/string r KkBb - -"));
            assert!(moves.is_empty());
        } else {
            panic!("expected Position");
        }
    }

    #[test]
    fn test_go_depth() {
        if let Some(Command::Go(params)) = parse_command("go depth 5") {
            assert_eq!(params.depth, Some(5));
            assert!(params.nodes.is_none());
            assert!(!params.infinite);
        } else {
            panic!("expected Go");
        }
    }

    #[test]
    fn test_go_multiple_params() {
        if let Some(Command::Go(params)) = parse_command("go depth 7 nodes 50000 movetime 1000") {
            assert_eq!(params.depth, Some(7));
            assert_eq!(params.nodes, Some(50000));
            assert_eq!(params.movetime, Some(1000));
            assert!(!params.infinite);
        } else {
            panic!("expected Go");
        }
    }

    #[test]
    fn test_go_infinite() {
        if let Some(Command::Go(params)) = parse_command("go infinite") {
            assert!(params.infinite);
        } else {
            panic!("expected Go");
        }
    }

    #[test]
    fn test_go_no_params() {
        // bare "go" should work — stub will just return first legal move
        if let Some(Command::Go(params)) = parse_command("go") {
            assert!(params.depth.is_none());
            assert!(params.nodes.is_none());
            assert!(!params.infinite);
        } else {
            panic!("expected Go");
        }
    }

    #[test]
    fn test_setoption() {
        if let Some(Command::SetOption { name, value }) =
            parse_command("setoption name GameMode value FreeForAll")
        {
            assert_eq!(name, "GameMode");
            assert_eq!(value, "FreeForAll");
        } else {
            panic!("expected SetOption");
        }
    }

    #[test]
    fn test_setoption_path_value() {
        if let Some(Command::SetOption { name, value }) =
            parse_command("setoption name LogFile value /tmp/engine.log")
        {
            assert_eq!(name, "LogFile");
            assert_eq!(value, "/tmp/engine.log");
        } else {
            panic!("expected SetOption");
        }
    }

    #[test]
    fn test_setoption_none_value() {
        if let Some(Command::SetOption { name, value }) =
            parse_command("setoption name LogFile value none")
        {
            assert_eq!(name, "LogFile");
            assert_eq!(value, "none");
        } else {
            panic!("expected SetOption");
        }
    }
}
