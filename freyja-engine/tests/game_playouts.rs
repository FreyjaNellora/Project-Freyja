//! Integration test: random game playouts.
//!
//! Runs 1000+ games from the starting position with random moves.
//! Every game must terminate without panic (checkmate, stalemate, draw, or claim win).
//! Uses a seeded LCG for reproducibility.

use freyja_engine::game_state::GameState;

/// LCG for deterministic move selection.
fn lcg_next(seed: &mut u64) -> u64 {
    *seed = seed.wrapping_mul(6_364_136_223_846_793_005).wrapping_add(1);
    *seed
}

/// Play a single game with random moves, return the number of half-moves played.
fn play_random_game(seed: &mut u64) -> usize {
    let mut gs = GameState::new_standard_ffa();
    let mut half_moves = 0;

    // Safety limit to prevent infinite loops in case of bugs
    const MAX_HALF_MOVES: usize = 2000;

    while !gs.is_game_over() && half_moves < MAX_HALF_MOVES {
        let moves = gs.legal_moves();
        if moves.is_empty() {
            gs.handle_no_legal_moves();
            if gs.is_game_over() {
                break;
            }
            // If still not over after handling, something unexpected happened
            // but we continue to let check_elimination_chain resolve it.
            continue;
        }

        let idx = (lcg_next(seed) >> 33) as usize % moves.len();
        gs.apply_move(moves[idx]);
        half_moves += 1;
    }

    assert!(
        gs.is_game_over(),
        "Game did not terminate within {} half-moves (seed state: {})",
        MAX_HALF_MOVES,
        seed
    );

    half_moves
}

#[test]
fn test_1000_random_playouts() {
    let mut seed: u64 = 0xF4E7_7A40_2026;
    let mut total_moves = 0usize;
    let mut shortest = usize::MAX;
    let mut longest = 0usize;

    const NUM_GAMES: usize = 1000;

    for _game in 0..NUM_GAMES {
        let moves = play_random_game(&mut seed);
        total_moves += moves;
        shortest = shortest.min(moves);
        longest = longest.max(moves);
    }

    let avg = total_moves / NUM_GAMES;
    eprintln!(
        "\n=== Random Playout Summary ===\n\
         Games:    {}\n\
         Shortest: {} half-moves\n\
         Longest:  {} half-moves\n\
         Average:  {} half-moves\n\
         Total:    {} half-moves\n",
        NUM_GAMES, shortest, longest, avg, total_moves
    );
}
