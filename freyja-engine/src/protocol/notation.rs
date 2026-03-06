//! Move string parsing and formatting for protocol I/O.
//!
//! Move strings use long algebraic notation on the 14x14 board:
//! - Files: a-n, Ranks: 1-14
//! - Normal: `d2d4`, Capture: `e4d5`, Castling: king from/to e.g. `h1j1`
//! - Promotion: `d7d8q` (lowercase piece char: q/r/b/n)
//! - Variable length (4-7 chars) due to 2-digit ranks

use crate::board::types::{PieceType, Square};
use crate::move_gen::Move;

/// Parse a move string and match it against the legal moves list.
///
/// The move string encodes only from/to squares and optional promotion.
/// We find the unique legal move matching those fields.
pub fn parse_move_str(s: &str, legal_moves: &[Move]) -> Result<Move, String> {
    let s = s.trim();
    if s.len() < 4 {
        return Err(format!("move string too short: '{s}'"));
    }

    let (from_sq, rest) =
        parse_square_prefix(s).ok_or_else(|| format!("invalid from-square in move: '{s}'"))?;
    let (to_sq, rest) =
        parse_square_prefix(rest).ok_or_else(|| format!("invalid to-square in move: '{s}'"))?;

    let promotion = if rest.is_empty() {
        None
    } else {
        Some(
            parse_promo_char(rest)
                .ok_or_else(|| format!("invalid promotion char in move: '{s}'"))?,
        )
    };

    // Find matching legal move
    let mut matched: Option<Move> = None;
    for &mv in legal_moves {
        if mv.from_sq() == from_sq && mv.to_sq() == to_sq {
            let mv_promo = mv.promotion();
            if promotion == mv_promo {
                if matched.is_some() {
                    return Err(format!("ambiguous move: '{s}'"));
                }
                matched = Some(mv);
            }
        }
    }

    matched.ok_or_else(|| format!("illegal move: '{s}'"))
}

/// Parse a square from the start of a string, returning (Square, remaining_str).
///
/// Handles variable-length rank notation: `a1` (2 chars), `a14` (3 chars).
fn parse_square_prefix(s: &str) -> Option<(Square, &str)> {
    if s.len() < 2 {
        return None;
    }

    // Try 3-char square first (2-digit rank like "a10"), then 2-char
    if s.len() >= 3
        && let Some(sq) = Square::from_notation(&s[..3])
    {
        return Some((sq, &s[3..]));
    }
    if let Some(sq) = Square::from_notation(&s[..2]) {
        return Some((sq, &s[2..]));
    }

    None
}

/// Parse a single promotion character into a PieceType.
fn parse_promo_char(s: &str) -> Option<PieceType> {
    if s.len() != 1 {
        return None;
    }
    match s.as_bytes()[0] {
        b'q' => Some(PieceType::PromotedQueen),
        b'r' => Some(PieceType::Rook),
        b'b' => Some(PieceType::Bishop),
        b'n' => Some(PieceType::Knight),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game_state::GameState;

    #[test]
    fn test_parse_square_prefix_single_digit_rank() {
        let (sq, rest) = parse_square_prefix("d2d4").unwrap();
        assert_eq!(sq, Square::from_notation("d2").unwrap());
        assert_eq!(rest, "d4");
    }

    #[test]
    fn test_parse_square_prefix_double_digit_rank() {
        // a14 is a corner square (invalid), use d14 which is valid
        let (sq, rest) = parse_square_prefix("d14d1").unwrap();
        assert_eq!(sq, Square::from_notation("d14").unwrap());
        assert_eq!(rest, "d1");
    }

    #[test]
    fn test_parse_move_from_starting_position() {
        let mut gs = GameState::new_standard_ffa();
        let legal = gs.legal_moves();
        // Red's pawn d2->d4 should be a legal double-push
        let mv = parse_move_str("d2d4", &legal).unwrap();
        assert_eq!(mv.from_sq(), Square::from_notation("d2").unwrap());
        assert_eq!(mv.to_sq(), Square::from_notation("d4").unwrap());
    }

    #[test]
    fn test_parse_move_single_push() {
        let mut gs = GameState::new_standard_ffa();
        let legal = gs.legal_moves();
        let mv = parse_move_str("d2d3", &legal).unwrap();
        assert_eq!(mv.from_sq(), Square::from_notation("d2").unwrap());
        assert_eq!(mv.to_sq(), Square::from_notation("d3").unwrap());
    }

    #[test]
    fn test_parse_illegal_move() {
        let mut gs = GameState::new_standard_ffa();
        let legal = gs.legal_moves();
        assert!(parse_move_str("d2d5", &legal).is_err());
    }

    #[test]
    fn test_parse_move_too_short() {
        assert!(parse_move_str("d2", &[]).is_err());
    }

    #[test]
    fn test_parse_move_invalid_square() {
        assert!(parse_move_str("z2d4", &[]).is_err());
    }

    #[test]
    fn test_parse_promo_char() {
        assert_eq!(parse_promo_char("q"), Some(PieceType::PromotedQueen));
        assert_eq!(parse_promo_char("r"), Some(PieceType::Rook));
        assert_eq!(parse_promo_char("b"), Some(PieceType::Bishop));
        assert_eq!(parse_promo_char("n"), Some(PieceType::Knight));
        assert_eq!(parse_promo_char("x"), None);
        assert_eq!(parse_promo_char("qq"), None);
    }
}
