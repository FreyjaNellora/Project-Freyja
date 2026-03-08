//! Move ordering heuristics: killer moves, history heuristic, MVV-LVA scoring.
//!
//! Combines TT best move, captures (MVV-LVA), promotions, killer moves,
//! and history heuristic into a unified move scoring function.

use arrayvec::ArrayVec;

use crate::board::types::*;
use crate::eval::piece_value;
use crate::move_gen::{MAX_MOVES, Move};
use crate::search::MAX_DEPTH;

// ─── Constants ─────────────────────────────────────────────────────────────

/// Total squares on the 14x14 board.
const TOTAL_SQUARES: usize = 196;

/// Number of players.
const PLAYERS: usize = 4;

/// Move ordering priority scores.
const TT_MOVE_SCORE: i32 = 1_000_000;
const PROMOTION_SCORE: i32 = 500_000;
const CAPTURE_BASE_SCORE: i32 = 100_000;
const KILLER_SCORE_0: i32 = 90_000;
const KILLER_SCORE_1: i32 = 80_000;

// ─── MVV-LVA ───────────────────────────────────────────────────────────────

/// Score a capture using Most Valuable Victim - Least Valuable Attacker.
///
/// Higher score = better capture. Queen captured by pawn scores highest.
#[inline]
pub fn mvv_lva_score(mv: Move) -> i32 {
    if let Some(captured) = mv.captured() {
        let victim = piece_value(captured) as i32;
        let attacker = piece_value(mv.piece_type()) as i32;
        CAPTURE_BASE_SCORE + victim * 10 - attacker
    } else {
        0
    }
}

// ─── Killer Table ──────────────────────────────────────────────────────────

/// Killer move table: 2 slots per ply per player.
///
/// Killer moves are quiet moves that caused score improvements or cutoffs
/// at sibling nodes. They are good candidates to try early.
pub struct KillerTable {
    table: [[[Option<Move>; 2]; PLAYERS]; MAX_DEPTH],
    /// Number of times a killer move was the best/cutoff move.
    hits: u64,
    /// Number of times a killer move was tested.
    probes: u64,
}

impl Default for KillerTable {
    fn default() -> Self {
        Self::new()
    }
}

impl KillerTable {
    /// Create an empty killer table.
    pub fn new() -> Self {
        Self {
            table: [[[None; 2]; PLAYERS]; MAX_DEPTH],
            hits: 0,
            probes: 0,
        }
    }

    /// Clear all killer moves and reset statistics.
    pub fn clear(&mut self) {
        self.table = [[[None; 2]; PLAYERS]; MAX_DEPTH];
        self.hits = 0;
        self.probes = 0;
    }

    /// Check if a move is a killer at this ply for this player.
    /// Updates probe/hit statistics.
    #[inline]
    pub fn is_killer(&mut self, ply: usize, player: Player, mv: Move) -> bool {
        if ply >= MAX_DEPTH {
            return false;
        }
        let p = player.index();
        let is_k = self.table[ply][p][0] == Some(mv) || self.table[ply][p][1] == Some(mv);
        if is_k {
            self.hits += 1;
        }
        is_k
    }

    /// Check if a move is a killer (without updating stats).
    #[inline]
    pub fn is_killer_no_stats(&self, ply: usize, player: Player, mv: Move) -> bool {
        if ply >= MAX_DEPTH {
            return false;
        }
        let p = player.index();
        self.table[ply][p][0] == Some(mv) || self.table[ply][p][1] == Some(mv)
    }

    /// Which killer slot does this move match? 0, 1, or None.
    #[inline]
    pub fn killer_slot(&self, ply: usize, player: Player, mv: Move) -> Option<usize> {
        if ply >= MAX_DEPTH {
            return None;
        }
        let p = player.index();
        if self.table[ply][p][0] == Some(mv) {
            Some(0)
        } else if self.table[ply][p][1] == Some(mv) {
            Some(1)
        } else {
            None
        }
    }

    /// Store a killer move. Shifts slot 0 to slot 1, inserts in slot 0.
    /// Only call for quiet moves (not captures, not promotions).
    pub fn store(&mut self, ply: usize, player: Player, mv: Move) {
        if ply >= MAX_DEPTH {
            return;
        }
        let p = player.index();
        // Don't store if already in slot 0
        if self.table[ply][p][0] == Some(mv) {
            return;
        }
        // Shift slot 0 to slot 1, insert new in slot 0
        self.table[ply][p][1] = self.table[ply][p][0];
        self.table[ply][p][0] = Some(mv);
    }

    /// Increment probe count (called when testing a quiet move).
    #[inline]
    pub fn probe_increment(&mut self) {
        self.probes += 1;
    }

    /// Get killer statistics: (hits, probes).
    #[inline]
    pub fn stats(&self) -> (u64, u64) {
        (self.hits, self.probes)
    }

    /// Get killer hit rate as a percentage.
    #[inline]
    pub fn hit_rate_pct(&self) -> f64 {
        if self.probes == 0 {
            0.0
        } else {
            (self.hits as f64 / self.probes as f64) * 100.0
        }
    }
}

// ─── History Table ─────────────────────────────────────────────────────────

/// History heuristic table: from-to scoring for quiet move ordering.
///
/// Updated when a quiet move produces a score improvement (Max^n)
/// or beta cutoff (negamax). Uses depth^2 bonus so deeper cutoffs
/// contribute more weight.
///
/// Must be extractable for Stage 11 MCTS Progressive History (ADR-007).
pub struct HistoryTable {
    table: Box<[[u32; TOTAL_SQUARES]; TOTAL_SQUARES]>,
}

impl Default for HistoryTable {
    fn default() -> Self {
        Self::new()
    }
}

impl HistoryTable {
    /// Create a zeroed history table.
    pub fn new() -> Self {
        Self {
            table: Box::new([[0u32; TOTAL_SQUARES]; TOTAL_SQUARES]),
        }
    }

    /// Clear all history scores.
    pub fn clear(&mut self) {
        for row in self.table.iter_mut() {
            for val in row.iter_mut() {
                *val = 0;
            }
        }
    }

    /// Get the history score for a from-to pair.
    #[inline]
    pub fn get(&self, from: u8, to: u8) -> u32 {
        self.table[from as usize][to as usize]
    }

    /// Increment history score with depth^2 bonus.
    #[inline]
    pub fn update(&mut self, from: u8, to: u8, depth: u32) {
        let bonus = depth * depth;
        let entry = &mut self.table[from as usize][to as usize];
        *entry = entry.saturating_add(bonus);

        // Cap at a reasonable max to prevent overflow domination
        if *entry > 1_000_000 {
            self.age();
        }
    }

    /// Age the table: halve all entries.
    /// Called to prevent history scores from growing without bound.
    pub fn age(&mut self) {
        for row in self.table.iter_mut() {
            for val in row.iter_mut() {
                *val >>= 1;
            }
        }
    }

    /// Raw table access for Stage 11 extraction (ADR-007).
    pub fn raw(&self) -> &[[u32; TOTAL_SQUARES]; TOTAL_SQUARES] {
        &self.table
    }
}

// ─── Move Scoring ──────────────────────────────────────────────────────────

/// Score a single move for ordering purposes.
///
/// Priority: TT move > MVV-LVA captures > promotions > killer0 > killer1 > history > quiet.
/// Returns i32 score; higher = search first.
#[inline]
pub fn score_move(
    mv: Move,
    tt_move: Option<Move>,
    killers: &KillerTable,
    history: &HistoryTable,
    ply: usize,
    player: Player,
) -> i32 {
    // TT best move gets highest priority
    if tt_move == Some(mv) {
        return TT_MOVE_SCORE;
    }

    // Captures: MVV-LVA
    if mv.is_capture() {
        return mvv_lva_score(mv);
    }

    // Promotions (non-capture)
    if mv.promotion().is_some() {
        return PROMOTION_SCORE;
    }

    // Killer moves
    if let Some(slot) = killers.killer_slot(ply, player, mv) {
        return if slot == 0 {
            KILLER_SCORE_0
        } else {
            KILLER_SCORE_1
        };
    }

    // History heuristic
    let h = history.get(mv.from_sq().0, mv.to_sq().0);
    h as i32
}

/// Score and sort a move list in-place (descending by score).
pub fn order_moves(
    moves: &mut ArrayVec<Move, MAX_MOVES>,
    tt_move: Option<Move>,
    killers: &KillerTable,
    history: &HistoryTable,
    ply: usize,
    player: Player,
) {
    // Score all moves
    let mut scored: ArrayVec<(Move, i32), MAX_MOVES> = moves
        .iter()
        .map(|&mv| (mv, score_move(mv, tt_move, killers, history, ply, player)))
        .collect();

    // Sort descending by score
    scored.sort_unstable_by(|a, b| b.1.cmp(&a.1));

    // Write back sorted moves
    moves.clear();
    for (mv, _) in &scored {
        moves.push(*mv);
    }
}

/// Sort captures by MVV-LVA (for quiescence search).
pub fn order_captures_mvv_lva(captures: &mut ArrayVec<Move, MAX_MOVES>) {
    captures.sort_unstable_by_key(|m| std::cmp::Reverse(mvv_lva_score(*m)));
}

// ─── Tests ─────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::board::types::Square;

    fn make_quiet_move(from: u8, to: u8) -> Move {
        Move::new(Square(from), Square(to), PieceType::Knight)
    }

    fn make_capture_move(from: u8, to: u8, captured: PieceType) -> Move {
        Move::capture(Square(from), Square(to), PieceType::Pawn, captured)
    }

    #[test]
    fn test_mvv_lva_queen_capture_by_pawn_highest() {
        let qxp = make_capture_move(10, 20, PieceType::Queen); // pawn captures queen
        let pxp = Move::capture(Square(10), Square(20), PieceType::Queen, PieceType::Pawn); // queen captures pawn

        assert!(
            mvv_lva_score(qxp) > mvv_lva_score(pxp),
            "PxQ should score higher than QxP"
        );
    }

    #[test]
    fn test_killer_store_and_check() {
        let mut killers = KillerTable::new();
        let mv1 = make_quiet_move(10, 20);
        let mv2 = make_quiet_move(30, 40);
        let mv3 = make_quiet_move(50, 60);

        killers.store(5, Player::Red, mv1);
        assert!(killers.is_killer_no_stats(5, Player::Red, mv1));
        assert!(!killers.is_killer_no_stats(5, Player::Blue, mv1));
        assert!(!killers.is_killer_no_stats(6, Player::Red, mv1));

        // Store mv2 at same ply — mv1 shifts to slot 1
        killers.store(5, Player::Red, mv2);
        assert!(killers.is_killer_no_stats(5, Player::Red, mv1)); // slot 1
        assert!(killers.is_killer_no_stats(5, Player::Red, mv2)); // slot 0

        // Store mv3 — mv1 is evicted, mv2 shifts to slot 1
        killers.store(5, Player::Red, mv3);
        assert!(!killers.is_killer_no_stats(5, Player::Red, mv1)); // evicted
        assert!(killers.is_killer_no_stats(5, Player::Red, mv2)); // slot 1
        assert!(killers.is_killer_no_stats(5, Player::Red, mv3)); // slot 0
    }

    #[test]
    fn test_killer_no_duplicate() {
        let mut killers = KillerTable::new();
        let mv1 = make_quiet_move(10, 20);
        let mv2 = make_quiet_move(30, 40);

        killers.store(5, Player::Red, mv1);
        killers.store(5, Player::Red, mv2);
        // Store mv1 again — should not shift since it's already slot 1
        // Actually, it's not slot 0, so it will be inserted in slot 0.
        // But the no-duplicate check only prevents re-storing slot 0.
        killers.store(5, Player::Red, mv1);
        // Both should still be present
        assert!(killers.is_killer_no_stats(5, Player::Red, mv1));
        assert!(killers.is_killer_no_stats(5, Player::Red, mv2));
    }

    #[test]
    fn test_history_update_and_get() {
        let mut history = HistoryTable::new();
        assert_eq!(history.get(10, 20), 0);

        history.update(10, 20, 3); // bonus = 9
        assert_eq!(history.get(10, 20), 9);

        history.update(10, 20, 4); // bonus = 16
        assert_eq!(history.get(10, 20), 25);
    }

    #[test]
    fn test_history_age() {
        let mut history = HistoryTable::new();
        history.update(10, 20, 10); // bonus = 100
        assert_eq!(history.get(10, 20), 100);

        history.age();
        assert_eq!(history.get(10, 20), 50);
    }

    #[test]
    fn test_history_clear() {
        let mut history = HistoryTable::new();
        history.update(10, 20, 5);
        assert!(history.get(10, 20) > 0);

        history.clear();
        assert_eq!(history.get(10, 20), 0);
    }

    #[test]
    fn test_score_move_tt_highest() {
        let killers = KillerTable::new();
        let history = HistoryTable::new();
        let mv = make_quiet_move(10, 20);

        let score_with_tt = score_move(mv, Some(mv), &killers, &history, 0, Player::Red);
        let score_without_tt = score_move(mv, None, &killers, &history, 0, Player::Red);

        assert!(score_with_tt > score_without_tt);
        assert_eq!(score_with_tt, TT_MOVE_SCORE);
    }

    #[test]
    fn test_score_move_capture_over_quiet() {
        let killers = KillerTable::new();
        let history = HistoryTable::new();
        let capture = make_capture_move(10, 20, PieceType::Pawn);
        let quiet = make_quiet_move(10, 20);

        let cap_score = score_move(capture, None, &killers, &history, 0, Player::Red);
        let quiet_score = score_move(quiet, None, &killers, &history, 0, Player::Red);

        assert!(
            cap_score > quiet_score,
            "Captures should score higher than quiet moves"
        );
    }

    #[test]
    fn test_order_moves_tt_first() {
        let killers = KillerTable::new();
        let history = HistoryTable::new();
        let mv1 = make_quiet_move(10, 20);
        let mv2 = make_quiet_move(30, 40);
        let mv3 = make_quiet_move(50, 60);

        let mut moves: ArrayVec<Move, MAX_MOVES> = ArrayVec::new();
        moves.push(mv1);
        moves.push(mv2);
        moves.push(mv3);

        // mv2 is the TT move — should be first after ordering
        order_moves(&mut moves, Some(mv2), &killers, &history, 0, Player::Red);
        assert_eq!(moves[0], mv2, "TT move should be first");
    }

    #[test]
    fn test_order_captures_mvv_lva() {
        let pxq = make_capture_move(10, 20, PieceType::Queen); // pawn captures queen
        let pxp = make_capture_move(30, 40, PieceType::Pawn); // pawn captures pawn

        let mut captures: ArrayVec<Move, MAX_MOVES> = ArrayVec::new();
        captures.push(pxp);
        captures.push(pxq);

        order_captures_mvv_lva(&mut captures);
        assert_eq!(captures[0], pxq, "PxQ should come first");
    }
}
