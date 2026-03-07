//! Protocol response formatting.

use crate::board::types::Player;
use crate::move_gen::Move;

/// Format a bestmove response.
pub fn format_bestmove(mv: Option<Move>) -> String {
    match mv {
        Some(mv) => format!("bestmove {mv}"),
        None => "bestmove (none)".to_string(),
    }
}

/// Format an info string with search results.
///
/// All fields are optional; only present fields are included.
pub fn format_info(
    depth: Option<u32>,
    scores: Option<[i16; 4]>,
    nodes: Option<u64>,
    nps: Option<u64>,
    pv: Option<&[Move]>,
) -> String {
    let mut parts: Vec<String> = vec!["info".to_string()];

    if let Some(d) = depth {
        parts.push(format!("depth {d}"));
    }
    if let Some(s) = scores {
        parts.push(format!(
            "score red {} blue {} yellow {} green {}",
            s[0], s[1], s[2], s[3]
        ));
    }
    if let Some(n) = nodes {
        parts.push(format!("nodes {n}"));
    }
    if let Some(n) = nps {
        parts.push(format!("nps {n}"));
    }
    if let Some(pv_moves) = pv
        && !pv_moves.is_empty()
    {
        let pv_str: Vec<String> = pv_moves.iter().map(|m| m.to_string()).collect();
        parts.push(format!("pv {}", pv_str.join(" ")));
    }

    parts.join(" ")
}

/// Format an info string message.
pub fn format_info_string(msg: &str) -> String {
    format!("info string {msg}")
}

/// Format an elimination event.
pub fn format_eliminated(player: Player, reason: &str) -> String {
    format!("info string eliminated {} {reason}", player_name(player))
}

/// Format a turn change event.
pub fn format_nextturn(player: Player) -> String {
    format!("info string nextturn {}", player_name(player))
}

/// Format an error message.
pub fn format_error(msg: &str) -> String {
    format!("info string error: {msg}")
}

/// Convert Player to protocol name.
fn player_name(player: Player) -> &'static str {
    match player {
        Player::Red => "Red",
        Player::Blue => "Blue",
        Player::Yellow => "Yellow",
        Player::Green => "Green",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::board::types::Square;

    #[test]
    fn test_bestmove_with_move() {
        let mv = Move::new(
            Square::from_notation("d2").unwrap(),
            Square::from_notation("d4").unwrap(),
            crate::board::types::PieceType::Pawn,
        );
        assert_eq!(format_bestmove(Some(mv)), "bestmove d2d4");
    }

    #[test]
    fn test_bestmove_none() {
        assert_eq!(format_bestmove(None), "bestmove (none)");
    }

    #[test]
    fn test_info_full() {
        let result = format_info(
            Some(5),
            Some([150, 120, 100, 140]),
            Some(50000),
            Some(100000),
            None,
        );
        assert_eq!(
            result,
            "info depth 5 score red 150 blue 120 yellow 100 green 140 nodes 50000 nps 100000"
        );
    }

    #[test]
    fn test_info_depth_only() {
        assert_eq!(format_info(Some(3), None, None, None, None), "info depth 3");
    }

    #[test]
    fn test_eliminated() {
        assert_eq!(
            format_eliminated(Player::Red, "checkmate"),
            "info string eliminated Red checkmate"
        );
    }

    #[test]
    fn test_nextturn() {
        assert_eq!(format_nextturn(Player::Blue), "info string nextturn Blue");
    }

    #[test]
    fn test_error() {
        assert_eq!(
            format_error("unknown command 'bogus'"),
            "info string error: unknown command 'bogus'"
        );
    }
}
