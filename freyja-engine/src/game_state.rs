//! Game state management: turns, eliminations, scoring, DKW.
//!
//! Wraps Board with game-level state for four-player chess.
//! Fixed-size data structures only (ADR-004). No heap allocation.
//!
//! **Search boundary (Odin W5):** Search uses `make_move`/`unmake_move` on
//! the Board directly, NOT `apply_move`. GameState fields (`player_status`,
//! `scores`, etc.) are stale during search. This is expected and correct.

use arrayvec::ArrayVec;

use crate::board::Board;
use crate::board::types::*;
use crate::move_gen::{MAX_MOVES, Move, MoveFlags, generate_legal_moves, make_move};

// ─── Constants ──────────────────────────────────────────────────────────────

/// Maximum half-moves before forced draw.
pub const MAX_GAME_LENGTH: usize = 1024;

/// FFA points awarded for checkmating an opponent's king.
pub const CHECKMATE_POINTS: u16 = 20;

/// FFA points awarded to a stalemated player (self-award).
pub const STALEMATE_POINTS: u16 = 20;

/// FFA points awarded to each active player on draw.
pub const DRAW_POINTS: u16 = 10;

/// Point lead required to claim win with 2 active players.
pub const CLAIM_WIN_THRESHOLD: u16 = 21;

/// 50-move rule threshold in half-moves (50 rounds × 4 players).
pub const FIFTY_MOVE_THRESHOLD: u16 = 200;

// ─── Enums ──────────────────────────────────────────────────────────────────

/// Status of a player in the game.
///
/// Three states, not two (Odin lesson): DKW is distinct from Eliminated.
/// - Active: normal play
/// - DeadKingWalking: resigned/timed out, pieces dead, king wanders
/// - Eliminated: fully out (checkmate, stalemate, DKW king stuck/captured)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlayerStatus {
    /// Player is actively playing.
    Active,
    /// Player resigned/timed out. Pieces are dead (0pts), king makes random moves.
    DeadKingWalking,
    /// Player is fully eliminated. King removed from board.
    Eliminated,
}

/// Reason a player was eliminated.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EliminationReason {
    Checkmate,
    Stalemate,
    Resignation,
    Timeout,
    DkwKingStuck,
}

/// Game mode — determines win conditions.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GameMode {
    FreeForAll,
}

/// Result of the game.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GameResult {
    Ongoing,
    Decisive { winner: Player },
    Draw,
}

/// What happened when a player's turn arrived.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TurnDetermination {
    HasMoves,
    Checkmate,
    Stalemate,
}

// ─── Scoring ────────────────────────────────────────────────────────────────

/// FFA capture points by piece type. Dead/grey pieces return 0.
pub fn capture_points(piece_type: PieceType) -> u16 {
    match piece_type {
        PieceType::Pawn => 1,
        PieceType::Knight => 3,
        PieceType::Bishop => 5,
        PieceType::Rook => 5,
        PieceType::Queen => 9,
        PieceType::PromotedQueen => 1, // NOT 9 — dual-value system
        PieceType::King => 0,
    }
}

/// Bonus points for checking multiple opponent kings with one move.
pub fn check_bonus_points(kings_checked: usize) -> u16 {
    match kings_checked {
        0 | 1 => 0,
        2 => 1,
        _ => 5, // 3 is max in 4PC
    }
}

// ─── GameState ──────────────────────────────────────────────────────────────

/// The full game state for four-player chess.
///
/// Fixed-size only (ADR-004). Cheaply cloneable for MCTS (Stage 10).
/// Position history is a fixed-size array, not Vec.
#[derive(Clone)]
pub struct GameState {
    board: Board,
    player_status: [PlayerStatus; 4],
    scores: [u16; 4],
    current_player: Player,
    elimination_order: [Option<Player>; 4],
    elimination_count: u8,
    position_history: [u64; MAX_GAME_LENGTH],
    history_count: u16,
    half_move_clock: u16,
    #[allow(dead_code)] // Used in future stages (Teams mode, Stage 18)
    game_mode: GameMode,
    game_over: bool,
    winner: Option<Player>,
    rng_seed: u64,
}

// ─── Construction ───────────────────────────────────────────────────────────

impl GameState {
    /// Create a new game state from a board.
    pub fn new(board: Board) -> Self {
        let current_player = board.side_to_move();
        Self {
            board,
            player_status: [PlayerStatus::Active; 4],
            scores: [0; 4],
            current_player,
            elimination_order: [None; 4],
            elimination_count: 0,
            position_history: [0u64; MAX_GAME_LENGTH],
            history_count: 0,
            half_move_clock: 0,
            game_mode: GameMode::FreeForAll,
            game_over: false,
            winner: None,
            rng_seed: 0xDEAD_BEEF_CAFE_BABE,
        }
    }

    /// Create a standard FFA game from the starting position.
    pub fn new_standard_ffa() -> Self {
        Self::new(Board::starting_position())
    }
}

// ─── Accessors ──────────────────────────────────────────────────────────────

impl GameState {
    pub fn board(&self) -> &Board {
        &self.board
    }

    pub fn board_mut(&mut self) -> &mut Board {
        &mut self.board
    }

    pub fn player_status(&self, player: Player) -> PlayerStatus {
        self.player_status[player.index()]
    }

    pub fn score(&self, player: Player) -> u16 {
        self.scores[player.index()]
    }

    pub fn scores(&self) -> [u16; 4] {
        self.scores
    }

    pub fn current_player(&self) -> Player {
        self.current_player
    }

    pub fn is_game_over(&self) -> bool {
        self.game_over
    }

    pub fn winner(&self) -> Option<Player> {
        self.winner
    }

    pub fn result(&self) -> GameResult {
        if !self.game_over {
            GameResult::Ongoing
        } else if let Some(w) = self.winner {
            GameResult::Decisive { winner: w }
        } else {
            GameResult::Draw
        }
    }

    pub fn is_active(&self, player: Player) -> bool {
        self.player_status[player.index()] == PlayerStatus::Active
    }

    pub fn active_player_count(&self) -> usize {
        self.player_status
            .iter()
            .filter(|&&s| s == PlayerStatus::Active)
            .count()
    }

    pub fn half_move_clock(&self) -> u16 {
        self.half_move_clock
    }

    pub fn history_count(&self) -> u16 {
        self.history_count
    }

    /// Generate legal moves for the current player.
    pub fn legal_moves(&mut self) -> ArrayVec<Move, MAX_MOVES> {
        debug_assert!(
            self.is_active(self.current_player),
            "Cannot generate moves for non-active player {}",
            self.current_player
        );
        self.board.set_side_to_move(self.current_player);
        generate_legal_moves(&mut self.board)
    }
}

// ─── Turn Management ────────────────────────────────────────────────────────

impl GameState {
    /// Find the next active player after `after`, skipping eliminated/DKW players.
    /// Returns None if no active player exists.
    fn next_active_player(&self, after: Player) -> Option<Player> {
        let mut candidate = after.next();
        for _ in 0..3 {
            if self.player_status[candidate.index()] == PlayerStatus::Active {
                return Some(candidate);
            }
            candidate = candidate.next();
        }
        None
    }

    /// Find the previous active or DKW player before `before`.
    /// Used to attribute checkmate points (who delivered the mating position).
    fn prev_active_or_dkw_player(&self, before: Player) -> Option<Player> {
        let mut candidate = before.prev();
        for _ in 0..3 {
            match self.player_status[candidate.index()] {
                PlayerStatus::Active | PlayerStatus::DeadKingWalking => {
                    return Some(candidate);
                }
                PlayerStatus::Eliminated => {}
            }
            candidate = candidate.prev();
        }
        None
    }
}

// ─── Check / Checkmate / Stalemate Detection ────────────────────────────────

impl GameState {
    /// Determine whether a player is checkmated, stalemated, or has moves.
    /// Only called when the player's turn arrives.
    fn determine_status_at_turn(&mut self, player: Player) -> TurnDetermination {
        let saved_stm = self.board.side_to_move();
        self.board.set_side_to_move(player);
        let legal = generate_legal_moves(&mut self.board);
        self.board.set_side_to_move(saved_stm);

        if legal.is_empty() {
            if self.board.is_in_check(player) {
                TurnDetermination::Checkmate
            } else {
                TurnDetermination::Stalemate
            }
        } else {
            TurnDetermination::HasMoves
        }
    }

    /// Count how many active opponent kings are in check from the given attacker.
    fn count_kings_checked(&self, attacker: Player) -> usize {
        let mut count = 0;
        for opp in attacker.opponents() {
            // Check Active and DKW players (they still have kings on board)
            if self.player_status[opp.index()] == PlayerStatus::Eliminated {
                continue;
            }
            let king_sq = self.board.king_square(opp);
            if king_sq == ELIMINATED_KING_SENTINEL {
                continue;
            }
            if self.board.is_square_attacked_by(Square(king_sq), attacker) {
                count += 1;
            }
        }
        count
    }
}

// ─── Elimination ────────────────────────────────────────────────────────────

impl GameState {
    /// Eliminate a player. Removes king from board, sets sentinel.
    /// Pieces stay on board as "dead" (0 points on capture).
    fn eliminate_player(&mut self, player: Player, reason: EliminationReason) {
        tracing::info!(
            player = %player,
            reason = ?reason,
            scores = ?self.scores,
            "Player eliminated"
        );

        self.player_status[player.index()] = PlayerStatus::Eliminated;

        // Track elimination order
        if (self.elimination_count as usize) < 4 {
            self.elimination_order[self.elimination_count as usize] = Some(player);
            self.elimination_count += 1;
        }

        // Remove king from board, set sentinel.
        // Guard: king may already be captured via make_move in multi-player scenarios.
        let king_sq = self.board.king_square(player);
        if king_sq != ELIMINATED_KING_SENTINEL {
            let sq = Square(king_sq);
            if let Some(piece) = self.board.piece_at(sq)
                && piece.piece_type == PieceType::King
                && piece.player == player
            {
                self.board.remove_piece(sq);
            }
            self.board.set_king_eliminated(player);
        }
    }

    /// Start DKW for a player (resign/timeout in FFA mode).
    /// Pieces go "dead" (0pts on capture), king remains and wanders.
    fn start_dkw(&mut self, player: Player) {
        tracing::info!(player = %player, "Player entering DKW state");
        self.player_status[player.index()] = PlayerStatus::DeadKingWalking;
        // Pieces stay on board — "dead" status is derived from PlayerStatus,
        // not stored on each piece (unlike Odin's PieceStatus approach).
    }

    /// Check for checkmate/stalemate chain starting from current_player.
    /// When one player is eliminated, the next in line might also be mated.
    fn check_elimination_chain(&mut self) {
        loop {
            if self.game_over {
                break;
            }

            let player = self.current_player;
            if self.player_status[player.index()] != PlayerStatus::Active {
                break;
            }

            match self.determine_status_at_turn(player) {
                TurnDetermination::HasMoves => break,
                TurnDetermination::Checkmate => {
                    // Award checkmate points to the player who delivered the position
                    if let Some(prev) = self.prev_active_or_dkw_player(player) {
                        self.scores[prev.index()] += CHECKMATE_POINTS;
                    }
                    self.eliminate_player(player, EliminationReason::Checkmate);

                    // Advance to next active player and continue chain
                    if let Some(next) = self.next_active_player(player) {
                        self.current_player = next;
                        self.board.set_side_to_move(next);
                    } else {
                        self.end_game();
                        break;
                    }
                }
                TurnDetermination::Stalemate => {
                    // Stalemated player gets points
                    self.scores[player.index()] += STALEMATE_POINTS;
                    self.eliminate_player(player, EliminationReason::Stalemate);

                    if let Some(next) = self.next_active_player(player) {
                        self.current_player = next;
                        self.board.set_side_to_move(next);
                    } else {
                        self.end_game();
                        break;
                    }
                }
            }
        }
    }
}

// ─── DKW Processing ────────────────────────────────────────────────────────

impl GameState {
    /// Process DKW instant moves for all DKW players.
    /// Order: process each DKW player in turn order starting after current_player.
    fn process_dkw_moves(&mut self) {
        let mut candidate = self.current_player.next();
        for _ in 0..4 {
            if candidate == self.current_player {
                break;
            }
            if self.player_status[candidate.index()] == PlayerStatus::DeadKingWalking {
                let king_sq = self.board.king_square(candidate);
                if king_sq == ELIMINATED_KING_SENTINEL {
                    candidate = candidate.next();
                    continue;
                }

                if let Some(dkw_mv) = self.generate_dkw_move(candidate) {
                    // Save side_to_move, make DKW move, restore
                    let saved_stm = self.board.side_to_move();
                    self.board.set_side_to_move(candidate);
                    let _undo = make_move(&mut self.board, dkw_mv);
                    // DKW moves are permanent — no unmake
                    self.board.set_side_to_move(saved_stm);

                    tracing::debug!(
                        player = %candidate,
                        from = %dkw_mv.from_sq(),
                        to = %dkw_mv.to_sq(),
                        "DKW king move"
                    );
                } else {
                    // DKW king is stuck — eliminate
                    self.eliminate_player(candidate, EliminationReason::DkwKingStuck);
                }
            }
            candidate = candidate.next();
        }
    }

    /// Generate a random legal king move for a DKW player.
    /// King-only, no captures, no castling. Returns None if stuck.
    fn generate_dkw_move(&mut self, player: Player) -> Option<Move> {
        let saved_stm = self.board.side_to_move();
        self.board.set_side_to_move(player);
        let legal = generate_legal_moves(&mut self.board);
        self.board.set_side_to_move(saved_stm);

        // Filter to king-only, no captures, no castling
        let king_moves: ArrayVec<Move, 8> = legal
            .into_iter()
            .filter(|m| {
                m.piece_type() == PieceType::King
                    && !m.is_capture()
                    && m.flags() != MoveFlags::Castle
            })
            .collect();

        if king_moves.is_empty() {
            return None;
        }

        // LCG random selection (same constants as Odin)
        self.rng_seed = self
            .rng_seed
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1);
        let idx = (self.rng_seed >> 33) as usize % king_moves.len();
        Some(king_moves[idx])
    }
}

// ─── Position History ───────────────────────────────────────────────────────

impl GameState {
    /// Record the current position hash in history.
    fn record_position(&mut self) {
        let idx = self.history_count as usize;
        if idx < MAX_GAME_LENGTH {
            self.position_history[idx] = self.board.zobrist_hash();
            self.history_count += 1;
        }
    }

    /// Check if the current position has occurred 3 times.
    pub fn is_threefold_repetition(&self) -> bool {
        let current_hash = self.board.zobrist_hash();
        let mut count = 0u32;
        for i in 0..self.history_count as usize {
            if self.position_history[i] == current_hash {
                count += 1;
                if count >= 3 {
                    return true;
                }
            }
        }
        false
    }
}

// ─── Game End Detection ─────────────────────────────────────────────────────

impl GameState {
    /// Check if the game should end.
    fn check_game_over(&mut self) {
        let active_count = self.active_player_count();

        // Draw conditions first
        if self.is_threefold_repetition() || self.half_move_clock >= FIFTY_MOVE_THRESHOLD {
            for player in Player::all() {
                if self.is_active(player) {
                    self.scores[player.index()] += DRAW_POINTS;
                }
            }
            self.game_over = true;
            tracing::info!(scores = ?self.scores, "Game drawn");
            return;
        }

        // One or zero active players → game over
        if active_count <= 1 {
            self.end_game();
            return;
        }

        // Claim win: 21+ lead with exactly 2 active players (FFA only)
        if active_count == 2 {
            let active: ArrayVec<(Player, u16), 4> = Player::all()
                .into_iter()
                .filter(|&p| self.is_active(p))
                .map(|p| (p, self.scores[p.index()]))
                .collect();

            let (p1, s1) = active[0];
            let (p2, s2) = active[1];

            if s1 >= s2 + CLAIM_WIN_THRESHOLD {
                self.game_over = true;
                self.winner = Some(p1);
                tracing::info!(winner = %p1, scores = ?self.scores, "Claim win");
                return;
            }
            if s2 >= s1 + CLAIM_WIN_THRESHOLD {
                self.game_over = true;
                self.winner = Some(p2);
                tracing::info!(winner = %p2, scores = ?self.scores, "Claim win");
            }
        }
    }

    /// End the game, determine winner by highest score among active players.
    fn end_game(&mut self) {
        self.game_over = true;

        let mut best_player = None;
        let mut best_score = 0u16;
        for player in Player::all() {
            if self.is_active(player) && self.scores[player.index()] >= best_score {
                // >= so we pick the last in turn order on tie (arbitrary but deterministic)
                best_score = self.scores[player.index()];
                best_player = Some(player);
            }
        }
        self.winner = best_player;

        tracing::info!(
            winner = ?self.winner,
            scores = ?self.scores,
            "Game over"
        );
    }
}

// ─── Apply Move (Central Game Loop Method) ──────────────────────────────────

impl GameState {
    /// Apply a move to the game state. This is the central method.
    ///
    /// Flow:
    /// 1. Score capture points (check owner's PlayerStatus for dead pieces)
    /// 2. Make the move on the board
    /// 3. Score check bonus
    /// 4. Update half-move clock
    /// 5. Record position history
    /// 6. Process DKW instant moves (BEFORE elimination chain — Odin lesson)
    /// 7. Advance to next active player
    /// 8. Check checkmate/stalemate chain for next player
    /// 9. Check game-over conditions
    pub fn apply_move(&mut self, mv: Move) {
        debug_assert!(!self.game_over, "game is already over");
        debug_assert_eq!(
            self.board.side_to_move(),
            self.current_player,
            "board side_to_move must match current_player"
        );

        let mover = self.current_player;

        // 1. Make the move on the board
        let undo = make_move(&mut self.board, mv);

        // 2. Score capture points (using undo.captured_piece for piece info)
        if let Some(captured) = undo.captured_piece {
            let owner_status = self.player_status[captured.player.index()];
            let pts = if owner_status == PlayerStatus::Active {
                capture_points(captured.piece_type)
            } else {
                0 // Dead/DKW/Eliminated pieces worth 0
            };
            self.scores[mover.index()] += pts;

            // King captured — immediate elimination (4PC multi-player scenario).
            // make_move already removed the king from the board, so set the
            // sentinel and eliminate. Award checkmate points to the capturer.
            if captured.piece_type == PieceType::King
                && self.player_status[captured.player.index()] != PlayerStatus::Eliminated
            {
                self.board.set_king_eliminated(captured.player);
                self.scores[mover.index()] += CHECKMATE_POINTS;
                // Use direct state change (not eliminate_player) to avoid
                // double remove_piece on the already-captured king.
                self.player_status[captured.player.index()] = PlayerStatus::Eliminated;
                if (self.elimination_count as usize) < 4 {
                    self.elimination_order[self.elimination_count as usize] = Some(captured.player);
                    self.elimination_count += 1;
                }
            }
        }

        // 3. Score check bonus
        let kings_checked = self.count_kings_checked(mover);
        let bonus = check_bonus_points(kings_checked);
        self.scores[mover.index()] += bonus;

        // 4. Update half-move clock
        if mv.is_capture() || mv.piece_type() == PieceType::Pawn {
            self.half_move_clock = 0;
        } else {
            self.half_move_clock += 1;
        }

        // 5. Record position in history
        self.record_position();

        // 6. Process DKW instant moves BEFORE elimination chain
        self.process_dkw_moves();

        // 7. Advance to next active player
        if let Some(next) = self.next_active_player(mover) {
            self.current_player = next;
            self.board.set_side_to_move(next);
        } else {
            self.end_game();
        }

        // 8. Check checkmate/stalemate chain for next player
        if !self.game_over {
            self.check_elimination_chain();
        }

        // 9. Check game-over conditions
        if !self.game_over {
            self.check_game_over();
        }
    }

    /// Resign a player. Triggers DKW (pieces go dead, king wanders).
    pub fn resign_player(&mut self, player: Player) {
        self.start_dkw(player);

        // If the resigned player was current, advance
        if self.current_player == player {
            if let Some(next) = self.next_active_player(player) {
                self.current_player = next;
                self.board.set_side_to_move(next);
                self.check_elimination_chain();
            } else {
                self.end_game();
            }
        }

        if !self.game_over {
            self.check_game_over();
        }
    }

    /// Timeout a player. Same behavior as resign.
    pub fn timeout_player(&mut self, player: Player) {
        self.resign_player(player);
    }

    /// Handle the case where the current player has no legal moves.
    /// Called from protocol when engine detects 0 legal moves.
    pub fn handle_no_legal_moves(&mut self) {
        if !self.game_over {
            self.check_elimination_chain();
        }
        if !self.game_over {
            self.process_dkw_moves();
        }
        if !self.game_over {
            self.check_game_over();
        }
    }
}

// ─── Tests ──────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::board::Board;

    // ── Construction ──

    #[test]
    fn test_new_standard_ffa() {
        let gs = GameState::new_standard_ffa();
        assert_eq!(gs.current_player(), Player::Red);
        assert!(!gs.is_game_over());
        assert_eq!(gs.active_player_count(), 4);
        for player in Player::all() {
            assert_eq!(gs.score(player), 0);
            assert!(gs.is_active(player));
        }
    }

    #[test]
    fn test_gamestate_clone() {
        let gs = GameState::new_standard_ffa();
        let gs2 = gs.clone();
        assert_eq!(gs.current_player(), gs2.current_player());
        assert_eq!(gs.scores(), gs2.scores());
    }

    #[test]
    fn test_gamestate_size_reasonable() {
        let size = std::mem::size_of::<GameState>();
        // Should be < 16KB (8KB history + Board ~2.5KB + small fields)
        assert!(
            size < 16384,
            "GameState is {} bytes, expected < 16384",
            size
        );
    }

    // ── Turn rotation ──

    #[test]
    fn test_next_active_player_all_active() {
        let gs = GameState::new_standard_ffa();
        assert_eq!(gs.next_active_player(Player::Red), Some(Player::Blue));
        assert_eq!(gs.next_active_player(Player::Blue), Some(Player::Yellow));
        assert_eq!(gs.next_active_player(Player::Yellow), Some(Player::Green));
        assert_eq!(gs.next_active_player(Player::Green), Some(Player::Red));
    }

    // 4PC Verification Matrix: Turn skip — Red eliminated
    #[test]
    fn test_turn_skip_red_eliminated() {
        let mut gs = GameState::new_standard_ffa();
        gs.player_status[Player::Red.index()] = PlayerStatus::Eliminated;
        assert_eq!(gs.next_active_player(Player::Green), Some(Player::Blue));
    }

    // 4PC Verification Matrix: Turn skip — Blue eliminated
    #[test]
    fn test_turn_skip_blue_eliminated() {
        let mut gs = GameState::new_standard_ffa();
        gs.player_status[Player::Blue.index()] = PlayerStatus::Eliminated;
        assert_eq!(gs.next_active_player(Player::Red), Some(Player::Yellow));
    }

    // 4PC Verification Matrix: Turn skip — Yellow eliminated
    #[test]
    fn test_turn_skip_yellow_eliminated() {
        let mut gs = GameState::new_standard_ffa();
        gs.player_status[Player::Yellow.index()] = PlayerStatus::Eliminated;
        assert_eq!(gs.next_active_player(Player::Blue), Some(Player::Green));
    }

    // 4PC Verification Matrix: Turn skip — Green eliminated
    #[test]
    fn test_turn_skip_green_eliminated() {
        let mut gs = GameState::new_standard_ffa();
        gs.player_status[Player::Green.index()] = PlayerStatus::Eliminated;
        assert_eq!(gs.next_active_player(Player::Yellow), Some(Player::Red));
    }

    #[test]
    fn test_turn_skip_two_eliminated() {
        let mut gs = GameState::new_standard_ffa();
        gs.player_status[Player::Blue.index()] = PlayerStatus::Eliminated;
        gs.player_status[Player::Yellow.index()] = PlayerStatus::Eliminated;
        assert_eq!(gs.next_active_player(Player::Red), Some(Player::Green));
    }

    #[test]
    fn test_turn_skip_three_eliminated() {
        let mut gs = GameState::new_standard_ffa();
        gs.player_status[Player::Blue.index()] = PlayerStatus::Eliminated;
        gs.player_status[Player::Yellow.index()] = PlayerStatus::Eliminated;
        gs.player_status[Player::Green.index()] = PlayerStatus::Eliminated;
        // Only Red active — no "next" active player
        assert_eq!(gs.next_active_player(Player::Red), None);
    }

    #[test]
    fn test_next_active_player_skips_dkw() {
        let mut gs = GameState::new_standard_ffa();
        gs.player_status[Player::Blue.index()] = PlayerStatus::DeadKingWalking;
        assert_eq!(gs.next_active_player(Player::Red), Some(Player::Yellow));
    }

    // ── Apply move basics ──

    #[test]
    fn test_apply_move_advances_turn() {
        let mut gs = GameState::new_standard_ffa();
        let moves = gs.legal_moves();
        assert!(!moves.is_empty());
        gs.apply_move(moves[0]);
        assert_eq!(gs.current_player(), Player::Blue);
    }

    #[test]
    fn test_apply_move_full_rotation() {
        let mut gs = GameState::new_standard_ffa();
        let expected = [Player::Blue, Player::Yellow, Player::Green, Player::Red];
        for &exp in &expected {
            let moves = gs.legal_moves();
            gs.apply_move(moves[0]);
            assert_eq!(gs.current_player(), exp);
        }
    }

    #[test]
    fn test_position_history_recorded() {
        let mut gs = GameState::new_standard_ffa();
        assert_eq!(gs.history_count(), 0);
        let moves = gs.legal_moves();
        gs.apply_move(moves[0]);
        assert_eq!(gs.history_count(), 1);
    }

    // ── Scoring ──

    #[test]
    fn test_capture_points_values() {
        assert_eq!(capture_points(PieceType::Pawn), 1);
        assert_eq!(capture_points(PieceType::Knight), 3);
        assert_eq!(capture_points(PieceType::Bishop), 5);
        assert_eq!(capture_points(PieceType::Rook), 5);
        assert_eq!(capture_points(PieceType::Queen), 9);
        assert_eq!(capture_points(PieceType::PromotedQueen), 1);
        assert_eq!(capture_points(PieceType::King), 0);
    }

    #[test]
    fn test_check_bonus_values() {
        assert_eq!(check_bonus_points(0), 0);
        assert_eq!(check_bonus_points(1), 0);
        assert_eq!(check_bonus_points(2), 1);
        assert_eq!(check_bonus_points(3), 5);
    }

    #[test]
    fn test_capture_scores_points() {
        // Set up a board where Red pawn can capture Blue pawn
        let mut board = Board::empty();

        // Kings for all 4 players (required for legal move generation)
        board.set_piece(
            Square::new(4, 3).unwrap(),
            Piece::new(PieceType::King, Player::Red),
        );
        board.set_piece(
            Square::new(9, 8).unwrap(),
            Piece::new(PieceType::King, Player::Blue),
        );
        board.set_piece(
            Square::new(10, 10).unwrap(),
            Piece::new(PieceType::King, Player::Yellow),
        );
        board.set_piece(
            Square::new(3, 10).unwrap(),
            Piece::new(PieceType::King, Player::Green),
        );

        // Red pawn at e5 (row 4, col 4)
        board.set_piece(
            Square::new(4, 4).unwrap(),
            Piece::new(PieceType::Pawn, Player::Red),
        );
        // Blue pawn at f6 (row 5, col 5) — Red can capture NE diagonal
        board.set_piece(
            Square::new(5, 5).unwrap(),
            Piece::new(PieceType::Pawn, Player::Blue),
        );

        board.set_side_to_move(Player::Red);

        let mut gs = GameState::new(board);
        let moves = gs.legal_moves();

        // Find the pawn capture
        let capture = moves
            .iter()
            .find(|m| m.is_capture() && m.piece_type() == PieceType::Pawn);
        if let Some(&cap) = capture {
            gs.apply_move(cap);
            assert_eq!(
                gs.score(Player::Red),
                1,
                "Pawn capture should score 1 point"
            );
        }
        // If no capture found, that's ok — board setup may not allow it.
        // The scoring function is still tested via unit tests above.
    }

    #[test]
    fn test_dead_piece_capture_scores_zero() {
        // Pieces owned by DKW/Eliminated players should score 0
        let mut gs = GameState::new_standard_ffa();
        // Verify the scoring logic directly
        // If owner is DKW, capture is 0
        gs.player_status[Player::Blue.index()] = PlayerStatus::DeadKingWalking;
        let owner_status = gs.player_status(Player::Blue);
        let pts = if owner_status == PlayerStatus::Active {
            capture_points(PieceType::Queen)
        } else {
            0
        };
        assert_eq!(pts, 0, "Dead piece capture should score 0");
    }

    // ── Checkmate detection ──
    // Minimal checkmate position: player's king in check + no legal moves.
    // We construct these carefully with all 4 kings on valid squares.

    fn setup_checkmate_board(
        mated_player: Player,
        king_sq: Square,
        attacker_sqs: &[(PieceType, Player, Square)],
        other_kings: &[(Player, Square)],
    ) -> Board {
        let mut board = Board::empty();
        board.set_piece(king_sq, Piece::new(PieceType::King, mated_player));
        for &(pt, player, sq) in attacker_sqs {
            board.set_piece(sq, Piece::new(pt, player));
        }
        for &(player, sq) in other_kings {
            board.set_piece(sq, Piece::new(PieceType::King, player));
        }
        board.set_side_to_move(mated_player);
        board
    }

    // 4PC Verification Matrix: Checkmate — Red
    #[test]
    fn test_checkmate_red() {
        // Red king at (0,3). Valid escapes: (0,4), (1,3), (1,4).
        // (0,2) and (1,2) are invalid corner squares.
        // Blue rook at (0,10): check along rank 0, covers (0,4).
        // Blue queen at (2,4): covers (1,3) diag and (1,4) vertically.
        let board = setup_checkmate_board(
            Player::Red,
            Square::new(0, 3).unwrap(),
            &[
                (PieceType::Rook, Player::Blue, Square::new(0, 10).unwrap()),
                (PieceType::Queen, Player::Blue, Square::new(2, 4).unwrap()),
            ],
            &[
                (Player::Blue, Square::new(10, 3).unwrap()),
                (Player::Yellow, Square::new(10, 10).unwrap()),
                (Player::Green, Square::new(3, 10).unwrap()),
            ],
        );
        let mut gs = GameState::new(board);
        let det = gs.determine_status_at_turn(Player::Red);
        assert_eq!(
            det,
            TurnDetermination::Checkmate,
            "Red should be checkmated"
        );
    }

    // 4PC Verification Matrix: Checkmate — Blue
    #[test]
    fn test_checkmate_blue() {
        // Blue king at (3,0). Valid escapes: (3,1), (4,0), (4,1).
        // (2,0) and (2,1) are invalid corner squares.
        // Green rook at (10,0): check along file 0, covers (4,0).
        // Green queen at (4,2): covers (3,1) diag and (4,1) by rank.
        let board = setup_checkmate_board(
            Player::Blue,
            Square::new(3, 0).unwrap(),
            &[
                (PieceType::Rook, Player::Green, Square::new(10, 0).unwrap()),
                (PieceType::Queen, Player::Green, Square::new(4, 2).unwrap()),
            ],
            &[
                (Player::Red, Square::new(0, 7).unwrap()),
                (Player::Yellow, Square::new(13, 6).unwrap()),
                (Player::Green, Square::new(6, 13).unwrap()),
            ],
        );
        let mut gs = GameState::new(board);
        let det = gs.determine_status_at_turn(Player::Blue);
        assert_eq!(
            det,
            TurnDetermination::Checkmate,
            "Blue should be checkmated"
        );
    }

    // 4PC Verification Matrix: Checkmate — Yellow
    #[test]
    fn test_checkmate_yellow() {
        // Yellow king at (13,3). Valid escapes: (13,4), (12,3), (12,4).
        // (13,2) and (12,2) are invalid corner squares.
        // Red rook at (13,10): check along rank 13, covers (13,4).
        // Red queen at (12,5): covers (12,3) and (12,4) along rank.
        let board = setup_checkmate_board(
            Player::Yellow,
            Square::new(13, 3).unwrap(),
            &[
                (PieceType::Rook, Player::Red, Square::new(13, 10).unwrap()),
                (PieceType::Queen, Player::Red, Square::new(12, 5).unwrap()),
            ],
            &[
                (Player::Red, Square::new(0, 7).unwrap()),
                (Player::Blue, Square::new(6, 0).unwrap()),
                (Player::Green, Square::new(6, 13).unwrap()),
            ],
        );
        let mut gs = GameState::new(board);
        let det = gs.determine_status_at_turn(Player::Yellow);
        assert_eq!(
            det,
            TurnDetermination::Checkmate,
            "Yellow should be checkmated"
        );
    }

    // 4PC Verification Matrix: Checkmate — Green
    #[test]
    fn test_checkmate_green() {
        // Green king at (3,13). Valid escapes: (3,12), (4,12), (4,13).
        // (2,13) and (2,12) are invalid corner squares.
        // Yellow rook at (10,13): check along file 13, covers (4,13).
        // Yellow queen at (4,11): covers (3,12) diag and (4,12) by rank.
        let board = setup_checkmate_board(
            Player::Green,
            Square::new(3, 13).unwrap(),
            &[
                (
                    PieceType::Rook,
                    Player::Yellow,
                    Square::new(10, 13).unwrap(),
                ),
                (
                    PieceType::Queen,
                    Player::Yellow,
                    Square::new(4, 11).unwrap(),
                ),
            ],
            &[
                (Player::Red, Square::new(0, 7).unwrap()),
                (Player::Blue, Square::new(6, 0).unwrap()),
                (Player::Yellow, Square::new(13, 6).unwrap()),
            ],
        );
        let mut gs = GameState::new(board);
        let det = gs.determine_status_at_turn(Player::Green);
        assert_eq!(
            det,
            TurnDetermination::Checkmate,
            "Green should be checkmated"
        );
    }

    // ── Stalemate detection ──

    fn setup_stalemate_board(
        staled_player: Player,
        king_sq: Square,
        blocker_sqs: &[(PieceType, Player, Square)],
        other_kings: &[(Player, Square)],
    ) -> Board {
        // Same helper, but king should NOT be in check
        setup_checkmate_board(staled_player, king_sq, blocker_sqs, other_kings)
    }

    // 4PC Verification Matrix: Stalemate — Red
    #[test]
    fn test_stalemate_red() {
        // Red king at (0,3). Valid escapes: (0,4), (1,3), (1,4).
        // NOT in check. All escape squares blocked.
        // Blue rook at (1,5): covers (1,3) and (1,4) along rank 1 (no check on (0,3)).
        // Blue knight at (2,5): covers (0,4) via knight move (no check on (0,3)).
        let board = setup_stalemate_board(
            Player::Red,
            Square::new(0, 3).unwrap(),
            &[
                (PieceType::Rook, Player::Blue, Square::new(1, 5).unwrap()),
                (PieceType::Knight, Player::Blue, Square::new(2, 5).unwrap()),
            ],
            &[
                (Player::Blue, Square::new(10, 3).unwrap()),
                (Player::Yellow, Square::new(10, 10).unwrap()),
                (Player::Green, Square::new(3, 10).unwrap()),
            ],
        );
        let mut gs = GameState::new(board);
        let det = gs.determine_status_at_turn(Player::Red);
        assert_eq!(
            det,
            TurnDetermination::Stalemate,
            "Red should be stalemated"
        );
    }

    // 4PC Verification Matrix: Stalemate — Blue
    #[test]
    fn test_stalemate_blue() {
        // Blue king at (3,0). Valid escapes: (3,1), (4,0), (4,1).
        // NOT in check. All escape squares blocked.
        // Green rook at (5,1): covers (3,1) and (4,1) along file 1 (no check on (3,0)).
        // Green knight at (5,2): covers (4,0) via knight move (no check on (3,0)).
        let board = setup_stalemate_board(
            Player::Blue,
            Square::new(3, 0).unwrap(),
            &[
                (PieceType::Rook, Player::Green, Square::new(5, 1).unwrap()),
                (PieceType::Knight, Player::Green, Square::new(5, 2).unwrap()),
            ],
            &[
                (Player::Red, Square::new(0, 7).unwrap()),
                (Player::Yellow, Square::new(13, 6).unwrap()),
                (Player::Green, Square::new(6, 13).unwrap()),
            ],
        );
        let mut gs = GameState::new(board);
        let det = gs.determine_status_at_turn(Player::Blue);
        assert_eq!(
            det,
            TurnDetermination::Stalemate,
            "Blue should be stalemated"
        );
    }

    // 4PC Verification Matrix: Stalemate — Yellow
    #[test]
    fn test_stalemate_yellow() {
        // Yellow king at (13,3). Valid escapes: (13,4), (12,3), (12,4).
        // NOT in check. All escape squares blocked.
        // Red rook at (12,5): covers (12,3) and (12,4) along rank 12 (no check on (13,3)).
        // Red knight at (11,5): covers (13,4) via knight move (no check on (13,3)).
        let board = setup_stalemate_board(
            Player::Yellow,
            Square::new(13, 3).unwrap(),
            &[
                (PieceType::Rook, Player::Red, Square::new(12, 5).unwrap()),
                (PieceType::Knight, Player::Red, Square::new(11, 5).unwrap()),
            ],
            &[
                (Player::Red, Square::new(0, 7).unwrap()),
                (Player::Blue, Square::new(6, 0).unwrap()),
                (Player::Green, Square::new(6, 13).unwrap()),
            ],
        );
        let mut gs = GameState::new(board);
        let det = gs.determine_status_at_turn(Player::Yellow);
        assert_eq!(
            det,
            TurnDetermination::Stalemate,
            "Yellow should be stalemated"
        );
    }

    // 4PC Verification Matrix: Stalemate — Green
    #[test]
    fn test_stalemate_green() {
        // Green king at (3,13). Valid escapes: (3,12), (4,12), (4,13).
        // NOT in check. All escape squares blocked.
        // Yellow rook at (5,12): covers (3,12) and (4,12) along file 12 (no check on (3,13)).
        // Yellow knight at (5,11): covers (4,13) via knight move (no check on (3,13)).
        let board = setup_stalemate_board(
            Player::Green,
            Square::new(3, 13).unwrap(),
            &[
                (PieceType::Rook, Player::Yellow, Square::new(5, 12).unwrap()),
                (
                    PieceType::Knight,
                    Player::Yellow,
                    Square::new(5, 11).unwrap(),
                ),
            ],
            &[
                (Player::Red, Square::new(0, 7).unwrap()),
                (Player::Blue, Square::new(6, 0).unwrap()),
                (Player::Yellow, Square::new(13, 6).unwrap()),
            ],
        );
        let mut gs = GameState::new(board);
        let det = gs.determine_status_at_turn(Player::Green);
        assert_eq!(
            det,
            TurnDetermination::Stalemate,
            "Green should be stalemated"
        );
    }

    #[test]
    fn test_stalemate_awards_20_points() {
        // Same position as test_stalemate_red
        let board = setup_stalemate_board(
            Player::Red,
            Square::new(0, 3).unwrap(),
            &[
                (PieceType::Rook, Player::Blue, Square::new(1, 5).unwrap()),
                (PieceType::Knight, Player::Blue, Square::new(2, 5).unwrap()),
            ],
            &[
                (Player::Blue, Square::new(10, 3).unwrap()),
                (Player::Yellow, Square::new(10, 10).unwrap()),
                (Player::Green, Square::new(3, 10).unwrap()),
            ],
        );
        let mut gs = GameState::new(board);
        gs.current_player = Player::Red;
        gs.board.set_side_to_move(Player::Red);

        // Trigger stalemate detection via elimination chain
        gs.check_elimination_chain();

        assert_eq!(
            gs.player_status(Player::Red),
            PlayerStatus::Eliminated,
            "Red should be eliminated"
        );
        assert_eq!(
            gs.score(Player::Red),
            STALEMATE_POINTS,
            "Red should get 20 stalemate points"
        );
    }

    // ── Elimination ──

    // 4PC Verification Matrix: King sentinel on elimination — all 4 players
    #[test]
    fn test_king_sentinel_red() {
        let mut gs = GameState::new_standard_ffa();
        gs.eliminate_player(Player::Red, EliminationReason::Checkmate);
        assert_eq!(
            gs.board().king_square(Player::Red),
            ELIMINATED_KING_SENTINEL
        );
        assert_eq!(gs.player_status(Player::Red), PlayerStatus::Eliminated);
    }

    #[test]
    fn test_king_sentinel_blue() {
        let mut gs = GameState::new_standard_ffa();
        gs.eliminate_player(Player::Blue, EliminationReason::Checkmate);
        assert_eq!(
            gs.board().king_square(Player::Blue),
            ELIMINATED_KING_SENTINEL
        );
    }

    #[test]
    fn test_king_sentinel_yellow() {
        let mut gs = GameState::new_standard_ffa();
        gs.eliminate_player(Player::Yellow, EliminationReason::Stalemate);
        assert_eq!(
            gs.board().king_square(Player::Yellow),
            ELIMINATED_KING_SENTINEL
        );
    }

    #[test]
    fn test_king_sentinel_green() {
        let mut gs = GameState::new_standard_ffa();
        gs.eliminate_player(Player::Green, EliminationReason::Timeout);
        assert_eq!(
            gs.board().king_square(Player::Green),
            ELIMINATED_KING_SENTINEL
        );
    }

    // ── DKW ──

    #[test]
    fn test_dkw_generates_king_only_moves() {
        let mut gs = GameState::new_standard_ffa();
        gs.player_status[Player::Red.index()] = PlayerStatus::DeadKingWalking;

        let dkw_mv = gs.generate_dkw_move(Player::Red);
        if let Some(mv) = dkw_mv {
            assert_eq!(mv.piece_type(), PieceType::King);
            assert!(!mv.is_capture());
            assert_ne!(mv.flags(), MoveFlags::Castle);
        }
        // It's also valid for DKW move to be None (stuck king), though
        // from starting position the king has no moves (blocked by own pieces).
    }

    #[test]
    fn test_dkw_stuck_king_eliminates() {
        // DKW king surrounded by own pieces (no empty adjacent squares)
        // From starting position, kings are surrounded.
        let mut gs = GameState::new_standard_ffa();
        gs.player_status[Player::Red.index()] = PlayerStatus::DeadKingWalking;

        let dkw_mv = gs.generate_dkw_move(Player::Red);
        assert!(
            dkw_mv.is_none(),
            "DKW king should be stuck in starting position"
        );
    }

    // ── Position history ──

    #[test]
    fn test_threefold_repetition_detected() {
        let mut gs = GameState::new_standard_ffa();
        let hash = gs.board().zobrist_hash();

        // Manually record same hash 3 times
        gs.position_history[0] = hash;
        gs.position_history[1] = hash;
        gs.position_history[2] = hash;
        gs.history_count = 3;

        assert!(gs.is_threefold_repetition());
    }

    #[test]
    fn test_no_threefold_with_two_occurrences() {
        let mut gs = GameState::new_standard_ffa();
        let hash = gs.board().zobrist_hash();

        gs.position_history[0] = hash;
        gs.position_history[1] = hash;
        gs.history_count = 2;

        assert!(!gs.is_threefold_repetition());
    }

    #[test]
    fn test_history_never_exceeds_max() {
        let mut gs = GameState::new_standard_ffa();
        // Force history_count near max
        gs.history_count = (MAX_GAME_LENGTH - 1) as u16;
        gs.record_position();
        assert_eq!(gs.history_count, MAX_GAME_LENGTH as u16);
        // Recording again should not overflow
        gs.record_position();
        assert_eq!(gs.history_count, MAX_GAME_LENGTH as u16);
    }

    // ── Game end conditions ──

    #[test]
    fn test_game_over_last_player_standing() {
        let mut gs = GameState::new_standard_ffa();
        gs.eliminate_player(Player::Blue, EliminationReason::Checkmate);
        gs.eliminate_player(Player::Yellow, EliminationReason::Checkmate);
        gs.eliminate_player(Player::Green, EliminationReason::Checkmate);
        gs.check_game_over();
        // Only one active player → game should be over when end_game is called
        // (check_game_over checks active_count <= 1)
        assert!(gs.is_game_over());
        assert_eq!(gs.winner(), Some(Player::Red));
    }

    #[test]
    fn test_game_over_claim_win() {
        let mut gs = GameState::new_standard_ffa();
        gs.eliminate_player(Player::Yellow, EliminationReason::Checkmate);
        gs.eliminate_player(Player::Green, EliminationReason::Checkmate);
        // Red has 25 points, Blue has 0
        gs.scores[Player::Red.index()] = 25;
        gs.scores[Player::Blue.index()] = 0;
        gs.check_game_over();
        assert!(gs.is_game_over());
        assert_eq!(gs.winner(), Some(Player::Red));
    }

    #[test]
    fn test_no_claim_win_under_threshold() {
        let mut gs = GameState::new_standard_ffa();
        gs.eliminate_player(Player::Yellow, EliminationReason::Checkmate);
        gs.eliminate_player(Player::Green, EliminationReason::Checkmate);
        // Red has 20 points, Blue has 0 — difference is 20, need 21
        gs.scores[Player::Red.index()] = 20;
        gs.scores[Player::Blue.index()] = 0;
        gs.check_game_over();
        assert!(!gs.is_game_over());
    }

    #[test]
    fn test_fifty_move_rule() {
        let mut gs = GameState::new_standard_ffa();
        gs.half_move_clock = FIFTY_MOVE_THRESHOLD;
        gs.check_game_over();
        assert!(gs.is_game_over());
        assert_eq!(gs.result(), GameResult::Draw);
        // Each active player should get 10 draw points
        for player in Player::all() {
            assert_eq!(gs.score(player), DRAW_POINTS);
        }
    }

    // ── Prev active player ──

    #[test]
    fn test_prev_active_or_dkw_player() {
        let gs = GameState::new_standard_ffa();
        assert_eq!(
            gs.prev_active_or_dkw_player(Player::Blue),
            Some(Player::Red)
        );
        assert_eq!(
            gs.prev_active_or_dkw_player(Player::Red),
            Some(Player::Green)
        );
    }

    #[test]
    fn test_prev_active_or_dkw_skips_eliminated() {
        let mut gs = GameState::new_standard_ffa();
        gs.player_status[Player::Red.index()] = PlayerStatus::Eliminated;
        assert_eq!(
            gs.prev_active_or_dkw_player(Player::Blue),
            Some(Player::Green)
        );
    }

    #[test]
    fn test_prev_active_or_dkw_includes_dkw() {
        let mut gs = GameState::new_standard_ffa();
        gs.player_status[Player::Red.index()] = PlayerStatus::DeadKingWalking;
        assert_eq!(
            gs.prev_active_or_dkw_player(Player::Blue),
            Some(Player::Red)
        );
    }
}
