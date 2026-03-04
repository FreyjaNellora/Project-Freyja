//! Zobrist hashing for four-player chess positions.
//!
//! Deterministic key generation with xorshift64 PRNG.
//! Keys: 7 piece types × 4 players × 196 squares + 4 side-to-move + 8 castling + 196 en passant.

use std::sync::OnceLock;

use super::types::*;

/// Zobrist key tables, initialized once at program startup.
pub struct ZobristKeys {
    /// Piece keys: `piece[piece_type][player][square_index]`
    pub piece: [[[u64; TOTAL_SQUARES]; PLAYERS]; PieceType::COUNT],
    /// Side-to-move keys: one per player.
    pub side_to_move: [u64; PLAYERS],
    /// Castling rights keys: one per bit (8 bits).
    pub castling: [u64; CASTLING_RIGHTS_BITS],
    /// En passant keys: one per square index.
    pub en_passant: [u64; TOTAL_SQUARES],
}

/// Simple xorshift64 PRNG for deterministic Zobrist key generation.
struct Xorshift64 {
    state: u64,
}

impl Xorshift64 {
    fn new(seed: u64) -> Self {
        Self {
            state: if seed == 0 { 1 } else { seed },
        }
    }

    fn next(&mut self) -> u64 {
        let mut x = self.state;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.state = x;
        x
    }
}

impl ZobristKeys {
    /// Generate Zobrist keys from a deterministic PRNG seed.
    fn generate(seed: u64) -> Self {
        let mut rng = Xorshift64::new(seed);
        let mut keys = ZobristKeys {
            piece: [[[0u64; TOTAL_SQUARES]; PLAYERS]; PieceType::COUNT],
            side_to_move: [0u64; PLAYERS],
            castling: [0u64; CASTLING_RIGHTS_BITS],
            en_passant: [0u64; TOTAL_SQUARES],
        };

        // Piece-square keys
        for pt in 0..PieceType::COUNT {
            for player in 0..PLAYERS {
                for sq in 0..TOTAL_SQUARES {
                    keys.piece[pt][player][sq] = rng.next();
                }
            }
        }

        // Side-to-move keys
        for player in 0..PLAYERS {
            keys.side_to_move[player] = rng.next();
        }

        // Castling keys
        for bit in 0..CASTLING_RIGHTS_BITS {
            keys.castling[bit] = rng.next();
        }

        // En passant keys
        for sq in 0..TOTAL_SQUARES {
            keys.en_passant[sq] = rng.next();
        }

        keys
    }
}

/// Deterministic seed for Zobrist key generation.
const ZOBRIST_SEED: u64 = 0x4652_4559_4A41_3450; // "FREYJA4P" in hex-ish

/// Global Zobrist key table, lazily initialized.
pub fn zobrist_keys() -> &'static ZobristKeys {
    static KEYS: OnceLock<ZobristKeys> = OnceLock::new();
    KEYS.get_or_init(|| ZobristKeys::generate(ZOBRIST_SEED))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_zobrist_keys_deterministic() {
        let keys1 = zobrist_keys();
        let keys2 = zobrist_keys();
        assert!(std::ptr::eq(keys1, keys2));
    }

    #[test]
    fn test_zobrist_keys_no_duplicates() {
        let keys = zobrist_keys();
        let mut all_keys: Vec<u64> = Vec::new();

        // Piece keys (only valid squares)
        for pt in 0..PieceType::COUNT {
            for player in 0..PLAYERS {
                for sq in 0..TOTAL_SQUARES {
                    if VALID_SQUARE_TABLE[sq] {
                        all_keys.push(keys.piece[pt][player][sq]);
                    }
                }
            }
        }
        // Side-to-move keys
        for &k in keys.side_to_move.iter() {
            all_keys.push(k);
        }
        // Castling keys
        for &k in keys.castling.iter() {
            all_keys.push(k);
        }
        // EP keys (only valid squares)
        for &idx in VALID_SQUARES_LIST.iter() {
            all_keys.push(keys.en_passant[idx as usize]);
        }

        let len_before = all_keys.len();
        all_keys.sort();
        all_keys.dedup();
        assert_eq!(
            all_keys.len(),
            len_before,
            "Found {} duplicate Zobrist keys",
            len_before - all_keys.len()
        );
    }

    #[test]
    fn test_zobrist_keys_nonzero() {
        let keys = zobrist_keys();
        for pt in 0..PieceType::COUNT {
            for player in 0..PLAYERS {
                for sq in 0..TOTAL_SQUARES {
                    if VALID_SQUARE_TABLE[sq] {
                        assert_ne!(
                            keys.piece[pt][player][sq], 0,
                            "Zero key at pt={}, player={}, sq={}",
                            pt, player, sq
                        );
                    }
                }
            }
        }
        for (i, &k) in keys.side_to_move.iter().enumerate() {
            assert_ne!(k, 0, "Zero side-to-move key at index {}", i);
        }
        for (i, &k) in keys.castling.iter().enumerate() {
            assert_ne!(k, 0, "Zero castling key at index {}", i);
        }
    }
}
