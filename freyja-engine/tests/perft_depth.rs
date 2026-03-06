//! Perft integration tests — permanent invariants (Stage 2 rule: perft values are forever).
//!
//! These values were established during Stage 2 and must NEVER change.
//! Any change to a perft value indicates a bug in move generation.

use freyja_engine::board::Board;
use freyja_engine::move_gen::perft;

#[test]
fn perft_depth_1_starting_position() {
    let mut board = Board::starting_position();
    assert_eq!(perft(&mut board, 1), 20);
}

#[test]
fn perft_depth_2_starting_position() {
    let mut board = Board::starting_position();
    assert_eq!(perft(&mut board, 2), 395);
}

#[test]
fn perft_depth_3_starting_position() {
    let mut board = Board::starting_position();
    assert_eq!(perft(&mut board, 3), 7800);
}

#[test]
fn perft_depth_4_starting_position() {
    let mut board = Board::starting_position();
    assert_eq!(perft(&mut board, 4), 152050);
}

#[test]
fn perft_board_state_preserved() {
    let mut board = Board::starting_position();
    let original = board.clone();
    let original_hash = board.zobrist_hash();

    let _ = perft(&mut board, 4);

    assert_eq!(board, original, "Board must be unchanged after perft");
    assert_eq!(
        board.zobrist_hash(),
        original_hash,
        "Zobrist must be unchanged"
    );
    board.assert_piece_list_sync();
}
