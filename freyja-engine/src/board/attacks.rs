//! Attack query API for four-player chess.
//!
//! Ray-based for sliders, lookup for knights/pawns/king.
//! This is the board boundary — nothing above Board should read `squares[]` directly.

use super::types::*;
use super::{ALL_DIRS, Board, DIAGONAL_DIRS, KNIGHT_OFFSETS, ORTHOGONAL_DIRS, PAWN_CAPTURE_DELTAS};

/// Maximum number of attackers we track per square.
const MAX_ATTACKERS: usize = 16;

/// Information about attackers of a square.
#[derive(Debug, Clone)]
pub struct AttackInfo {
    /// Number of attackers.
    pub count: u8,
    /// Squares of attackers.
    pub attacker_squares: [Option<Square>; MAX_ATTACKERS],
    /// Which players have at least one attacker on this square.
    pub attacking_players: [bool; PLAYERS],
}

impl Default for AttackInfo {
    fn default() -> Self {
        Self::new()
    }
}

impl AttackInfo {
    pub fn new() -> Self {
        Self {
            count: 0,
            attacker_squares: [None; MAX_ATTACKERS],
            attacking_players: [false; PLAYERS],
        }
    }

    /// Whether the square is attacked by anyone.
    pub fn is_attacked(&self) -> bool {
        self.count > 0
    }

    /// Whether the square is attacked by a specific player.
    pub fn is_attacked_by(&self, player: Player) -> bool {
        self.attacking_players[player.index()]
    }

    fn add_attacker(&mut self, sq: Square, player: Player) {
        if (self.count as usize) < MAX_ATTACKERS {
            self.attacker_squares[self.count as usize] = Some(sq);
        }
        self.count += 1;
        self.attacking_players[player.index()] = true;
    }
}

impl Board {
    /// Walk a ray from `sq` in direction `(dr, df)` and return the first piece found.
    /// Skips invalid corner squares — the ray continues on the other side.
    fn ray_find_piece(&self, sq: Square, dr: i8, df: i8) -> Option<(Piece, Square)> {
        let mut rank = sq.rank() as i8 + dr;
        let mut file = sq.file() as i8 + df;
        while rank >= 0 && rank < BOARD_SIZE as i8 && file >= 0 && file < BOARD_SIZE as i8 {
            let r = rank as u8;
            let f = file as u8;
            if !is_valid_square(r, f) {
                // Invalid corner — skip and continue ray
                rank += dr;
                file += df;
                continue;
            }
            let idx = r as usize * BOARD_SIZE + f as usize;
            if let Some(piece) = self.squares[idx] {
                return Some((piece, Square(idx as u8)));
            }
            rank += dr;
            file += df;
        }
        None
    }

    /// Check if a square is attacked by any piece of the given player.
    pub fn is_square_attacked_by(&self, sq: Square, attacker: Player) -> bool {
        let target_rank = sq.rank() as i8;
        let target_file = sq.file() as i8;

        // Check pawn attacks: look for attacker's pawns that can capture onto this square.
        // From the target's perspective, check the reverse of the pawn's capture deltas.
        for &(dr, df) in &PAWN_CAPTURE_DELTAS[attacker.index()] {
            let pawn_rank = target_rank - dr;
            let pawn_file = target_file - df;
            if pawn_rank >= 0
                && pawn_rank < BOARD_SIZE as i8
                && pawn_file >= 0
                && pawn_file < BOARD_SIZE as i8
            {
                let pr = pawn_rank as u8;
                let pf = pawn_file as u8;
                if is_valid_square(pr, pf) {
                    let idx = pr as usize * BOARD_SIZE + pf as usize;
                    if let Some(piece) = self.squares[idx]
                        && piece.player == attacker
                        && piece.piece_type == PieceType::Pawn
                    {
                        return true;
                    }
                }
            }
        }

        // Check knight attacks
        for &(dr, df) in &KNIGHT_OFFSETS {
            let kr = target_rank + dr;
            let kf = target_file + df;
            if kr >= 0 && kr < BOARD_SIZE as i8 && kf >= 0 && kf < BOARD_SIZE as i8 {
                let r = kr as u8;
                let f = kf as u8;
                if is_valid_square(r, f) {
                    let idx = r as usize * BOARD_SIZE + f as usize;
                    if let Some(piece) = self.squares[idx]
                        && piece.player == attacker
                        && piece.piece_type == PieceType::Knight
                    {
                        return true;
                    }
                }
            }
        }

        // Check king attacks (adjacent squares)
        for &(dr, df) in &ALL_DIRS {
            let kr = target_rank + dr;
            let kf = target_file + df;
            if kr >= 0 && kr < BOARD_SIZE as i8 && kf >= 0 && kf < BOARD_SIZE as i8 {
                let r = kr as u8;
                let f = kf as u8;
                if is_valid_square(r, f) {
                    let idx = r as usize * BOARD_SIZE + f as usize;
                    if let Some(piece) = self.squares[idx]
                        && piece.player == attacker
                        && piece.piece_type == PieceType::King
                    {
                        return true;
                    }
                }
            }
        }

        // Check slider attacks (ray-based)
        // Orthogonal rays: rook, queen, promoted queen
        for &(dr, df) in &ORTHOGONAL_DIRS {
            if let Some((piece, _)) = self.ray_find_piece(sq, dr, df)
                && piece.player == attacker
            {
                match piece.piece_type {
                    PieceType::Rook | PieceType::Queen | PieceType::PromotedQueen => {
                        return true;
                    }
                    _ => {}
                }
            }
        }

        // Diagonal rays: bishop, queen, promoted queen
        for &(dr, df) in &DIAGONAL_DIRS {
            if let Some((piece, _)) = self.ray_find_piece(sq, dr, df)
                && piece.player == attacker
            {
                match piece.piece_type {
                    PieceType::Bishop | PieceType::Queen | PieceType::PromotedQueen => {
                        return true;
                    }
                    _ => {}
                }
            }
        }

        false
    }

    /// Get full information about all pieces attacking a square.
    pub fn attackers_of(&self, sq: Square) -> AttackInfo {
        let mut info = AttackInfo::new();
        let target_rank = sq.rank() as i8;
        let target_file = sq.file() as i8;

        // Check pawn attacks from all players
        for &attacker in &Player::all() {
            for &(dr, df) in &PAWN_CAPTURE_DELTAS[attacker.index()] {
                let pawn_rank = target_rank - dr;
                let pawn_file = target_file - df;
                if pawn_rank >= 0
                    && pawn_rank < BOARD_SIZE as i8
                    && pawn_file >= 0
                    && pawn_file < BOARD_SIZE as i8
                {
                    let pr = pawn_rank as u8;
                    let pf = pawn_file as u8;
                    if is_valid_square(pr, pf) {
                        let idx = pr as usize * BOARD_SIZE + pf as usize;
                        if let Some(piece) = self.squares[idx]
                            && piece.player == attacker
                            && piece.piece_type == PieceType::Pawn
                        {
                            info.add_attacker(Square(idx as u8), attacker);
                        }
                    }
                }
            }
        }

        // Check knight attacks from all players
        for &(dr, df) in &KNIGHT_OFFSETS {
            let kr = target_rank + dr;
            let kf = target_file + df;
            if kr >= 0 && kr < BOARD_SIZE as i8 && kf >= 0 && kf < BOARD_SIZE as i8 {
                let r = kr as u8;
                let f = kf as u8;
                if is_valid_square(r, f) {
                    let idx = r as usize * BOARD_SIZE + f as usize;
                    if let Some(piece) = self.squares[idx]
                        && piece.piece_type == PieceType::Knight
                    {
                        info.add_attacker(Square(idx as u8), piece.player);
                    }
                }
            }
        }

        // Check king attacks from all players
        for &(dr, df) in &ALL_DIRS {
            let kr = target_rank + dr;
            let kf = target_file + df;
            if kr >= 0 && kr < BOARD_SIZE as i8 && kf >= 0 && kf < BOARD_SIZE as i8 {
                let r = kr as u8;
                let f = kf as u8;
                if is_valid_square(r, f) {
                    let idx = r as usize * BOARD_SIZE + f as usize;
                    if let Some(piece) = self.squares[idx]
                        && piece.piece_type == PieceType::King
                    {
                        info.add_attacker(Square(idx as u8), piece.player);
                    }
                }
            }
        }

        // Check slider attacks (ray-based)
        for &(dr, df) in &ORTHOGONAL_DIRS {
            if let Some((piece, piece_sq)) = self.ray_find_piece(sq, dr, df) {
                match piece.piece_type {
                    PieceType::Rook | PieceType::Queen | PieceType::PromotedQueen => {
                        info.add_attacker(piece_sq, piece.player);
                    }
                    _ => {}
                }
            }
        }

        for &(dr, df) in &DIAGONAL_DIRS {
            if let Some((piece, piece_sq)) = self.ray_find_piece(sq, dr, df) {
                match piece.piece_type {
                    PieceType::Bishop | PieceType::Queen | PieceType::PromotedQueen => {
                        info.add_attacker(piece_sq, piece.player);
                    }
                    _ => {}
                }
            }
        }

        info
    }

    /// Check if the given player's king is in check (attacked by any opponent).
    pub fn is_in_check(&self, player: Player) -> bool {
        let king_sq = self.king_squares[player.index()];
        if king_sq == ELIMINATED_KING_SENTINEL {
            return false;
        }
        let sq = Square(king_sq);
        for opponent in player.opponents() {
            if self.is_square_attacked_by(sq, opponent) {
                return true;
            }
        }
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn empty_board_with_hash() -> Board {
        let mut board = Board::empty();
        board.zobrist = board.compute_full_hash();
        board
    }

    // ── Pawn attack tests: 4PC Verification Matrix ──

    #[test]
    fn test_pawn_attack_red_ne() {
        let mut board = empty_board_with_hash();
        let pawn_sq = Square::from_notation("e5").unwrap();
        board.set_piece(pawn_sq, Piece::new(PieceType::Pawn, Player::Red));
        let target = Square::from_notation("f6").unwrap();
        assert!(board.is_square_attacked_by(target, Player::Red));
    }

    #[test]
    fn test_pawn_attack_red_nw() {
        let mut board = empty_board_with_hash();
        let pawn_sq = Square::from_notation("e5").unwrap();
        board.set_piece(pawn_sq, Piece::new(PieceType::Pawn, Player::Red));
        let target = Square::from_notation("d6").unwrap();
        assert!(board.is_square_attacked_by(target, Player::Red));
    }

    #[test]
    fn test_pawn_attack_red_does_not_attack_south() {
        let mut board = empty_board_with_hash();
        let pawn_sq = Square::from_notation("e5").unwrap();
        board.set_piece(pawn_sq, Piece::new(PieceType::Pawn, Player::Red));
        let behind = Square::from_notation("e4").unwrap();
        assert!(!board.is_square_attacked_by(behind, Player::Red));
        // Also check diagonal behind
        let diag_behind = Square::from_notation("d4").unwrap();
        assert!(!board.is_square_attacked_by(diag_behind, Player::Red));
    }

    #[test]
    fn test_pawn_attack_blue_ne() {
        let mut board = empty_board_with_hash();
        let pawn_sq = Square::from_notation("e5").unwrap();
        board.set_piece(pawn_sq, Piece::new(PieceType::Pawn, Player::Blue));
        // Blue attacks NE (+1,+1) and SE (-1,+1)
        let target_ne = Square::from_notation("f6").unwrap();
        assert!(board.is_square_attacked_by(target_ne, Player::Blue));
    }

    #[test]
    fn test_pawn_attack_blue_se() {
        let mut board = empty_board_with_hash();
        let pawn_sq = Square::from_notation("e5").unwrap();
        board.set_piece(pawn_sq, Piece::new(PieceType::Pawn, Player::Blue));
        let target_se = Square::from_notation("f4").unwrap();
        assert!(board.is_square_attacked_by(target_se, Player::Blue));
    }

    #[test]
    fn test_pawn_attack_blue_does_not_attack_west() {
        let mut board = empty_board_with_hash();
        let pawn_sq = Square::from_notation("e5").unwrap();
        board.set_piece(pawn_sq, Piece::new(PieceType::Pawn, Player::Blue));
        let west = Square::from_notation("d5").unwrap();
        assert!(!board.is_square_attacked_by(west, Player::Blue));
    }

    #[test]
    fn test_pawn_attack_yellow_se() {
        let mut board = empty_board_with_hash();
        let pawn_sq = Square::from_notation("e9").unwrap();
        board.set_piece(pawn_sq, Piece::new(PieceType::Pawn, Player::Yellow));
        // Yellow attacks SE (-1,+1) and SW (-1,-1)
        let target_se = Square::from_notation("f8").unwrap();
        assert!(board.is_square_attacked_by(target_se, Player::Yellow));
    }

    #[test]
    fn test_pawn_attack_yellow_sw() {
        let mut board = empty_board_with_hash();
        let pawn_sq = Square::from_notation("e9").unwrap();
        board.set_piece(pawn_sq, Piece::new(PieceType::Pawn, Player::Yellow));
        let target_sw = Square::from_notation("d8").unwrap();
        assert!(board.is_square_attacked_by(target_sw, Player::Yellow));
    }

    #[test]
    fn test_pawn_attack_yellow_does_not_attack_north() {
        let mut board = empty_board_with_hash();
        let pawn_sq = Square::from_notation("e9").unwrap();
        board.set_piece(pawn_sq, Piece::new(PieceType::Pawn, Player::Yellow));
        let north = Square::from_notation("e10").unwrap();
        assert!(!board.is_square_attacked_by(north, Player::Yellow));
    }

    #[test]
    fn test_pawn_attack_green_nw() {
        let mut board = empty_board_with_hash();
        let pawn_sq = Square::from_notation("i5").unwrap();
        board.set_piece(pawn_sq, Piece::new(PieceType::Pawn, Player::Green));
        // Green attacks NW (+1,-1) and SW (-1,-1)
        let target_nw = Square::from_notation("h6").unwrap();
        assert!(board.is_square_attacked_by(target_nw, Player::Green));
    }

    #[test]
    fn test_pawn_attack_green_sw() {
        let mut board = empty_board_with_hash();
        let pawn_sq = Square::from_notation("i5").unwrap();
        board.set_piece(pawn_sq, Piece::new(PieceType::Pawn, Player::Green));
        let target_sw = Square::from_notation("h4").unwrap();
        assert!(board.is_square_attacked_by(target_sw, Player::Green));
    }

    #[test]
    fn test_pawn_attack_green_does_not_attack_east() {
        let mut board = empty_board_with_hash();
        let pawn_sq = Square::from_notation("i5").unwrap();
        board.set_piece(pawn_sq, Piece::new(PieceType::Pawn, Player::Green));
        let east = Square::from_notation("j5").unwrap();
        assert!(!board.is_square_attacked_by(east, Player::Green));
    }

    // ── Knight attacks ──

    #[test]
    fn test_knight_attack_all_8_jumps() {
        let mut board = empty_board_with_hash();
        // Knight in the center at g7 (rank 6, file 6)
        let knight_sq = Square::from_notation("g7").unwrap();
        board.set_piece(knight_sq, Piece::new(PieceType::Knight, Player::Red));

        // All 8 L-shaped targets from (6,6): (8,7),(8,5),(4,7),(4,5),(7,8),(7,4),(5,8),(5,4)
        let targets = [
            (8, 7),
            (8, 5),
            (4, 7),
            (4, 5),
            (7, 8),
            (7, 4),
            (5, 8),
            (5, 4),
        ];
        for (r, f) in targets {
            let sq = Square::new(r, f).unwrap();
            assert!(
                board.is_square_attacked_by(sq, Player::Red),
                "Knight at g7 should attack ({}, {})",
                r,
                f
            );
        }
    }

    #[test]
    fn test_knight_attack_near_sw_corner() {
        let mut board = empty_board_with_hash();
        // Knight at d4 (rank 3, file 3) — near SW corner
        let knight_sq = Square::from_notation("d4").unwrap();
        board.set_piece(knight_sq, Piece::new(PieceType::Knight, Player::Red));

        // Some targets land on invalid corners
        // (1,4)=e2 valid, (1,2)=c2 INVALID (SW corner), (5,4)=e6, etc.
        let target_e2 = Square::from_notation("e2").unwrap();
        assert!(board.is_square_attacked_by(target_e2, Player::Red));

        // c2 is invalid — knight can't attack invalid squares
        assert!(Square::new(1, 2).is_none());
    }

    // ── Slider attacks ──

    #[test]
    fn test_rook_attack_horizontal_and_vertical() {
        let mut board = empty_board_with_hash();
        let rook_sq = Square::from_notation("g7").unwrap();
        board.set_piece(rook_sq, Piece::new(PieceType::Rook, Player::Red));

        // Horizontal: same rank
        assert!(board.is_square_attacked_by(Square::from_notation("d7").unwrap(), Player::Red));
        assert!(board.is_square_attacked_by(Square::from_notation("k7").unwrap(), Player::Red));
        // Vertical: same file
        assert!(board.is_square_attacked_by(Square::from_notation("g4").unwrap(), Player::Red));
        assert!(board.is_square_attacked_by(Square::from_notation("g10").unwrap(), Player::Red));
        // Diagonal: NOT attacked by rook
        assert!(!board.is_square_attacked_by(Square::from_notation("h8").unwrap(), Player::Red));
    }

    #[test]
    fn test_bishop_attack_diagonal() {
        let mut board = empty_board_with_hash();
        let bishop_sq = Square::from_notation("g7").unwrap();
        board.set_piece(bishop_sq, Piece::new(PieceType::Bishop, Player::Red));

        // Diagonals
        assert!(board.is_square_attacked_by(Square::from_notation("h8").unwrap(), Player::Red));
        assert!(board.is_square_attacked_by(Square::from_notation("f6").unwrap(), Player::Red));
        assert!(board.is_square_attacked_by(Square::from_notation("h6").unwrap(), Player::Red));
        assert!(board.is_square_attacked_by(Square::from_notation("f8").unwrap(), Player::Red));
        // Orthogonal: NOT attacked by bishop
        assert!(!board.is_square_attacked_by(Square::from_notation("g8").unwrap(), Player::Red));
    }

    #[test]
    fn test_rook_attack_blocked_by_piece() {
        let mut board = empty_board_with_hash();
        let rook_sq = Square::from_notation("g7").unwrap();
        board.set_piece(rook_sq, Piece::new(PieceType::Rook, Player::Red));
        // Blocker at g9
        let blocker_sq = Square::from_notation("g9").unwrap();
        board.set_piece(blocker_sq, Piece::new(PieceType::Pawn, Player::Blue));

        // g8 is attacked (before blocker)
        assert!(board.is_square_attacked_by(Square::from_notation("g8").unwrap(), Player::Red));
        // g9 is attacked (the blocker itself)
        assert!(board.is_square_attacked_by(blocker_sq, Player::Red));
        // g10 is NOT attacked (behind blocker)
        assert!(!board.is_square_attacked_by(Square::from_notation("g10").unwrap(), Player::Red));
    }

    #[test]
    fn test_queen_attack_all_directions() {
        let mut board = empty_board_with_hash();
        let queen_sq = Square::from_notation("g7").unwrap();
        board.set_piece(queen_sq, Piece::new(PieceType::Queen, Player::Red));

        // Orthogonal
        assert!(board.is_square_attacked_by(Square::from_notation("g10").unwrap(), Player::Red));
        assert!(board.is_square_attacked_by(Square::from_notation("d7").unwrap(), Player::Red));
        // Diagonal
        assert!(board.is_square_attacked_by(Square::from_notation("i9").unwrap(), Player::Red));
        assert!(board.is_square_attacked_by(Square::from_notation("e5").unwrap(), Player::Red));
    }

    #[test]
    fn test_promoted_queen_attacks_like_queen() {
        let mut board = empty_board_with_hash();
        let sq = Square::from_notation("g7").unwrap();
        board.set_piece(sq, Piece::new(PieceType::PromotedQueen, Player::Red));

        assert!(board.is_square_attacked_by(Square::from_notation("g10").unwrap(), Player::Red));
        assert!(board.is_square_attacked_by(Square::from_notation("i9").unwrap(), Player::Red));
    }

    // ── Slider crosses corners ──

    #[test]
    fn test_bishop_crosses_sw_corner() {
        let mut board = empty_board_with_hash();
        // Bishop at d4 (rank 3, file 3), diagonal SW crosses invalid corner
        let bishop_sq = Square::from_notation("d4").unwrap();
        board.set_piece(bishop_sq, Piece::new(PieceType::Bishop, Player::Red));
        // c3 (rank 2, file 2) is in the SW corner — INVALID, ray skips it
        assert!(Square::new(2, 2).is_none());
        // b2 (rank 1, file 1) is also invalid
        assert!(Square::new(1, 1).is_none());
        // a1 (rank 0, file 0) is also invalid — ray exits board
        // No valid square on the other side for this direction
    }

    #[test]
    fn test_bishop_crosses_ne_corner() {
        let mut board = empty_board_with_hash();
        // Bishop at k12 (rank 11, file 10), diagonal NE goes toward invalid corner
        let bishop_sq = Square::from_notation("k12").unwrap();
        board.set_piece(bishop_sq, Piece::new(PieceType::Bishop, Player::Yellow));
        // (12,11) = l13 is invalid, ray skips
        assert!(Square::new(12, 11).is_none());
        // (13,12) = m14 is also invalid — ray exits board
    }

    #[test]
    fn test_bishop_crosses_sw_corner_attacks_far_side() {
        let mut board = empty_board_with_hash();
        // Bishop at e4 (rank 3, file 4), diagonal SW: d3 valid, c2 invalid, b1 valid
        let bishop_sq = Square::from_notation("e4").unwrap();
        board.set_piece(bishop_sq, Piece::new(PieceType::Bishop, Player::Red));
        // d3 (rank 2, file 3) is valid — attacked
        let d3 = Square::from_notation("d3").unwrap();
        assert!(board.is_square_attacked_by(d3, Player::Red));
        // c2 (rank 1, file 2) is INVALID — skipped
        assert!(Square::new(1, 2).is_none());
        // b1 (rank 0, file 1) is INVALID — skipped, ray exits board
    }

    #[test]
    fn test_bishop_near_nw_corner_stops_at_invalid() {
        let mut board = empty_board_with_hash();
        // Bishop at d11 (rank 10, file 3), diagonal NW: c12 is (11,2) — INVALID (NW corner)
        let bishop_sq = Square::from_notation("d11").unwrap();
        board.set_piece(bishop_sq, Piece::new(PieceType::Bishop, Player::Blue));
        // c12 (rank 11, file 2) is in the NW corner — invalid
        assert!(Square::new(11, 2).is_none());
        // Ray skips invalid and exits board — no squares attacked in that direction
    }

    #[test]
    fn test_queen_near_se_corner_stops_at_invalid() {
        let mut board = empty_board_with_hash();
        // Queen at k4 (rank 3, file 10), diagonal SE: l3 is (2,11) — INVALID (SE corner)
        let queen_sq = Square::from_notation("k4").unwrap();
        board.set_piece(queen_sq, Piece::new(PieceType::Queen, Player::Green));
        // l3 (rank 2, file 11) is in the SE corner — invalid
        assert!(Square::new(2, 11).is_none());
        // Ray skips invalid and exits board — no squares attacked in that direction
    }

    #[test]
    fn test_rook_does_not_cross_corner_orthogonally() {
        let mut board = empty_board_with_hash();
        // Rook on d1 (rank 0, file 3), moving west into SW corner area
        // Orthogonal rays along rank 0: c1 (0,2) is INVALID, b1 (0,1) is INVALID, a1 (0,0) is INVALID
        // Rook should skip invalid squares but there are none valid on the other side
        let rook_sq = Square::from_notation("d1").unwrap();
        board.set_piece(rook_sq, Piece::new(PieceType::Rook, Player::Red));
        // e1 (rank 0, file 4) going east is valid — attacked
        let e1 = Square::from_notation("e1").unwrap();
        assert!(board.is_square_attacked_by(e1, Player::Red));
    }

    #[test]
    fn test_bishop_cross_corner_blocked_by_piece() {
        let mut board = empty_board_with_hash();
        // Bishop at e4 (rank 3, file 4), diagonal SW toward corner
        let bishop_sq = Square::from_notation("e4").unwrap();
        board.set_piece(bishop_sq, Piece::new(PieceType::Bishop, Player::Red));
        // Blocker at d3 (rank 2, file 3) — before the corner
        let blocker_sq = Square::from_notation("d3").unwrap();
        board.set_piece(blocker_sq, Piece::new(PieceType::Pawn, Player::Blue));
        // d3 is attacked (the blocker itself can be captured)
        assert!(board.is_square_attacked_by(blocker_sq, Player::Red));
        // Ray stops at blocker, doesn't continue across corner
    }

    // ── King attacks ──

    #[test]
    fn test_king_attack_all_8_adjacent() {
        let mut board = empty_board_with_hash();
        let king_sq = Square::from_notation("g7").unwrap();
        board.set_piece(king_sq, Piece::new(PieceType::King, Player::Red));

        for &(dr, df) in &ALL_DIRS {
            let r = 6i8 + dr;
            let f = 6i8 + df;
            if r >= 0 && r < 14 && f >= 0 && f < 14 {
                if let Some(sq) = Square::new(r as u8, f as u8) {
                    assert!(
                        board.is_square_attacked_by(sq, Player::Red),
                        "King at g7 should attack ({}, {})",
                        r,
                        f
                    );
                }
            }
        }
    }

    // ── is_in_check ──

    #[test]
    fn test_starting_position_no_player_in_check() {
        let board = Board::starting_position();
        for player in Player::all() {
            assert!(
                !board.is_in_check(player),
                "{} should not be in check in starting position",
                player
            );
        }
    }

    #[test]
    fn test_is_in_check_by_rook() {
        let mut board = empty_board_with_hash();
        // Red king at h1
        let king_sq = Square::from_notation("h5").unwrap();
        board.set_piece(king_sq, Piece::new(PieceType::King, Player::Red));
        // Blue rook on same file
        let rook_sq = Square::from_notation("h10").unwrap();
        board.set_piece(rook_sq, Piece::new(PieceType::Rook, Player::Blue));

        assert!(board.is_in_check(Player::Red));
    }

    #[test]
    fn test_is_in_check_eliminated_returns_false() {
        let board = Board::empty(); // All king squares = ELIMINATED_KING_SENTINEL
        assert!(!board.is_in_check(Player::Red));
        assert!(!board.is_in_check(Player::Blue));
        assert!(!board.is_in_check(Player::Yellow));
        assert!(!board.is_in_check(Player::Green));
    }

    #[test]
    fn test_is_in_check_three_opponent_check() {
        let mut board = empty_board_with_hash();
        // Red king in center
        let king_sq = Square::from_notation("g7").unwrap();
        board.set_piece(king_sq, Piece::new(PieceType::King, Player::Red));
        // Blue rook attacks along rank
        board.set_piece(
            Square::from_notation("d7").unwrap(),
            Piece::new(PieceType::Rook, Player::Blue),
        );
        // Yellow bishop attacks diagonally
        board.set_piece(
            Square::from_notation("i9").unwrap(),
            Piece::new(PieceType::Bishop, Player::Yellow),
        );
        // Green knight attacks with L-shape: (6+2,6+1)=(8,7)=h9
        board.set_piece(
            Square::from_notation("h9").unwrap(),
            Piece::new(PieceType::Knight, Player::Green),
        );

        assert!(board.is_in_check(Player::Red));

        // Verify all three opponents attack the king square
        let info = board.attackers_of(king_sq);
        assert!(info.is_attacked_by(Player::Blue));
        assert!(info.is_attacked_by(Player::Yellow));
        assert!(info.is_attacked_by(Player::Green));
        assert!(!info.is_attacked_by(Player::Red));
    }

    // ── attackers_of ──

    #[test]
    fn test_attackers_of_multiple_pieces() {
        let mut board = empty_board_with_hash();
        let target = Square::from_notation("g7").unwrap();

        // Place a Red rook attacking horizontally
        board.set_piece(
            Square::from_notation("d7").unwrap(),
            Piece::new(PieceType::Rook, Player::Red),
        );
        // Place a Blue bishop attacking diagonally
        board.set_piece(
            Square::from_notation("e5").unwrap(),
            Piece::new(PieceType::Bishop, Player::Blue),
        );

        let info = board.attackers_of(target);
        assert_eq!(info.count, 2);
        assert!(info.is_attacked_by(Player::Red));
        assert!(info.is_attacked_by(Player::Blue));
    }

    // ── Corner-edge knight tests for all 4 corners ──

    #[test]
    fn test_knight_near_nw_corner() {
        let mut board = empty_board_with_hash();
        // Knight at d12 (rank 11, file 3) — near NW corner
        let knight_sq = Square::from_notation("d12").unwrap();
        board.set_piece(knight_sq, Piece::new(PieceType::Knight, Player::Blue));
        // Targets: (13,4)=e14 valid, (13,2)=c14 INVALID
        assert!(Square::new(13, 2).is_none()); // c14 invalid
        let e14 = Square::from_notation("e14").unwrap();
        assert!(board.is_square_attacked_by(e14, Player::Blue));
    }

    #[test]
    fn test_knight_near_se_corner() {
        let mut board = empty_board_with_hash();
        // Knight at k4 (rank 3, file 10) — near SE corner
        let knight_sq = Square::from_notation("k4").unwrap();
        board.set_piece(knight_sq, Piece::new(PieceType::Knight, Player::Green));
        // (1,11)=l2 is INVALID (SE corner)
        assert!(Square::new(1, 11).is_none());
        // (5,11)=l6 is valid
        let l6 = Square::from_notation("l6").unwrap();
        assert!(board.is_square_attacked_by(l6, Player::Green));
    }

    #[test]
    fn test_knight_near_ne_corner() {
        let mut board = empty_board_with_hash();
        // Knight at k12 (rank 11, file 10) — near NE corner
        let knight_sq = Square::from_notation("k12").unwrap();
        board.set_piece(knight_sq, Piece::new(PieceType::Knight, Player::Yellow));
        // (13,11)=l14 is INVALID
        assert!(Square::new(13, 11).is_none());
    }
}
