//! Transposition table for caching search results.
//!
//! Fixed-size hash table indexed by Zobrist key. Power-of-2 sizing for fast modulo.
//! Stores 4-vector scores (Max^n) with flag semantics for exact vs bound entries.

use crate::move_gen::Move;
use crate::search::Score4;

// ─── Constants ─────────────────────────────────────────────────────────────

/// Default TT size in megabytes.
pub const DEFAULT_TT_SIZE_MB: usize = 16;

/// Empty key verification value (indicates unused entry).
const EMPTY_KEY: u32 = 0;

// ─── TTFlag ────────────────────────────────────────────────────────────────

/// Type of score stored in a TT entry.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum TTFlag {
    /// All 4 scores are exact at this depth.
    Exact = 0,
    /// Current player's score is a lower bound (beta cutoff in negamax).
    LowerBound = 1,
    /// Current player's score is an upper bound (failed to improve alpha in negamax).
    UpperBound = 2,
}

impl TTFlag {
    fn from_u8(v: u8) -> Self {
        match v {
            1 => TTFlag::LowerBound,
            2 => TTFlag::UpperBound,
            _ => TTFlag::Exact,
        }
    }
}

// ─── TTEntry ───────────────────────────────────────────────────────────────

/// A single transposition table entry.
///
/// Layout is compact: key_verify uses upper 32 bits of Zobrist hash
/// (lower bits are the index), giving 64 total bits of collision detection.
#[derive(Clone, Copy)]
#[repr(C)]
pub struct TTEntry {
    /// Upper 32 bits of Zobrist hash for collision detection.
    key_verify: u32,
    /// Best move found (raw Move bits), 0 = no move.
    best_move: u32,
    /// Score vector [Red, Blue, Yellow, Green] in centipawns.
    scores: [i16; 4],
    /// Search depth at which this entry was stored.
    depth: u8,
    /// Score type flag (Exact, LowerBound, UpperBound).
    flag: u8,
    /// Generation counter for replacement policy.
    generation: u8,
    /// Padding for alignment.
    _pad: u8,
}

impl TTEntry {
    /// Create an empty entry.
    const fn empty() -> Self {
        Self {
            key_verify: EMPTY_KEY,
            best_move: 0,
            scores: [0; 4],
            depth: 0,
            flag: 0,
            generation: 0,
            _pad: 0,
        }
    }

    /// Get the stored depth.
    #[inline]
    pub fn depth(&self) -> u8 {
        self.depth
    }

    /// Get the stored flag.
    #[inline]
    pub fn flag(&self) -> TTFlag {
        TTFlag::from_u8(self.flag)
    }

    /// Get the stored scores.
    #[inline]
    pub fn scores(&self) -> &Score4 {
        &self.scores
    }

    /// Get the best move, or None if no move stored.
    #[inline]
    pub fn best_move(&self) -> Option<Move> {
        if self.best_move == 0 {
            None
        } else {
            Some(Move(self.best_move))
        }
    }

    /// Check if this entry is empty (never written).
    #[inline]
    fn is_empty(&self) -> bool {
        self.key_verify == EMPTY_KEY && self.best_move == 0 && self.depth == 0
    }
}

// ─── TranspositionTable ────────────────────────────────────────────────────

/// Fixed-size hash table for caching search results.
///
/// Allocated once at searcher construction, never cloned during search.
/// Uses Vec internally — this is acceptable per ADR-004 because the TT
/// lives in MaxnSearcher (not in Board, GameState, or MoveUndo).
pub struct TranspositionTable {
    /// Entry storage. Size is always a power of 2.
    entries: Vec<TTEntry>,
    /// Bitmask for fast index computation: `hash as usize & mask`.
    mask: usize,
    /// Current generation. Incremented each new search for replacement policy.
    generation: u8,
    /// Probe count for hit rate statistics.
    probes: u64,
    /// Hit count (key matched on probe).
    hits: u64,
}

impl TranspositionTable {
    /// Create a new TT with the given size in megabytes.
    ///
    /// Size is rounded down to the nearest power of 2 in entries.
    pub fn new(size_mb: usize) -> Self {
        let entry_size = std::mem::size_of::<TTEntry>();
        let total_bytes = size_mb * 1024 * 1024;
        let num_entries_raw = total_bytes / entry_size;

        // Round down to power of 2
        let num_entries = if num_entries_raw == 0 {
            1
        } else {
            1usize << (usize::BITS - 1 - num_entries_raw.leading_zeros())
        };

        Self {
            entries: vec![TTEntry::empty(); num_entries],
            mask: num_entries - 1,
            generation: 0,
            probes: 0,
            hits: 0,
        }
    }

    /// Clear all entries and reset statistics.
    pub fn clear(&mut self) {
        for entry in self.entries.iter_mut() {
            *entry = TTEntry::empty();
        }
        self.generation = 0;
        self.probes = 0;
        self.hits = 0;
    }

    /// Signal a new search. Increments generation for replacement policy.
    /// Resets per-search statistics.
    pub fn new_search(&mut self) {
        self.generation = self.generation.wrapping_add(1);
        self.probes = 0;
        self.hits = 0;
    }

    /// Compute the table index from a Zobrist hash.
    #[inline]
    fn index(&self, hash: u64) -> usize {
        hash as usize & self.mask
    }

    /// Compute the verification key from a Zobrist hash (upper 32 bits).
    #[inline]
    fn verify_key(hash: u64) -> u32 {
        (hash >> 32) as u32
    }

    /// Probe the TT for a position.
    ///
    /// Returns the entry if the key matches (collision check passes).
    /// Returns None if no matching entry found.
    #[inline]
    pub fn probe(&mut self, hash: u64) -> Option<&TTEntry> {
        self.probes += 1;
        let idx = self.index(hash);
        let verify = Self::verify_key(hash);
        let entry = &self.entries[idx];

        if entry.key_verify == verify && !entry.is_empty() {
            self.hits += 1;
            Some(entry)
        } else {
            None
        }
    }

    /// Store a result in the TT.
    ///
    /// Replacement policy:
    /// 1. Always replace if key matches (same position, updated result)
    /// 2. Replace if new entry has >= depth (deeper search is more valuable)
    /// 3. Replace if existing entry is from an older generation (stale)
    /// 4. Otherwise, keep existing entry
    pub fn store(
        &mut self,
        hash: u64,
        depth: u8,
        flag: TTFlag,
        scores: Score4,
        best_move: Option<Move>,
    ) {
        let idx = self.index(hash);
        let verify = Self::verify_key(hash);
        let entry = &self.entries[idx];

        // Decide whether to replace
        let should_replace = entry.is_empty()
            || entry.key_verify == verify                    // same position
            || depth >= entry.depth                          // deeper or equal
            || entry.generation != self.generation; // stale entry

        if !should_replace {
            return;
        }

        self.entries[idx] = TTEntry {
            key_verify: verify,
            best_move: best_move.map_or(0, |m| m.0),
            scores,
            depth,
            flag: flag as u8,
            generation: self.generation,
            _pad: 0,
        };
    }

    /// Get TT hit statistics: (hits, probes).
    #[inline]
    pub fn stats(&self) -> (u64, u64) {
        (self.hits, self.probes)
    }

    /// Get TT hit rate as a percentage (0.0 - 100.0).
    #[inline]
    pub fn hit_rate_pct(&self) -> f64 {
        if self.probes == 0 {
            0.0
        } else {
            (self.hits as f64 / self.probes as f64) * 100.0
        }
    }

    /// Number of entries in the table.
    #[inline]
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Whether the table has zero entries.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

// ─── Tests ─────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_entry_size() {
        assert!(
            std::mem::size_of::<TTEntry>() <= 24,
            "TTEntry should be <= 24 bytes, got {}",
            std::mem::size_of::<TTEntry>()
        );
    }

    #[test]
    fn test_new_creates_power_of_2() {
        let tt = TranspositionTable::new(1); // 1 MB
        assert!(tt.len().is_power_of_two());
        assert!(tt.len() > 0);
    }

    #[test]
    fn test_store_and_probe_roundtrip() {
        let mut tt = TranspositionTable::new(1);
        let hash: u64 = 0xDEAD_BEEF_1234_5678;
        let scores: Score4 = [100, -50, 200, 300];
        let mv = Move(0x0000_1234);

        tt.store(hash, 5, TTFlag::Exact, scores, Some(mv));

        let entry = tt.probe(hash).expect("Should find stored entry");
        assert_eq!(*entry.scores(), scores);
        assert_eq!(entry.depth(), 5);
        assert_eq!(entry.flag(), TTFlag::Exact);
        assert_eq!(entry.best_move(), Some(mv));
    }

    #[test]
    fn test_probe_miss() {
        let mut tt = TranspositionTable::new(1);
        let hash: u64 = 0xDEAD_BEEF_1234_5678;
        assert!(tt.probe(hash).is_none());
    }

    #[test]
    fn test_collision_rejection() {
        let mut tt = TranspositionTable::new(1);
        let hash1: u64 = 0xAAAA_BBBB_0000_0001;
        let hash2: u64 = 0xCCCC_DDDD_0000_0001; // Same index, different verify key

        tt.store(hash1, 5, TTFlag::Exact, [100; 4], None);

        // hash2 maps to the same index but has different upper bits
        // Probe should miss (collision detection works)
        let entry = tt.probe(hash2);
        // If the mask is large enough that they map to different indices, this is fine.
        // If they map to the same index, entry should be None due to key_verify mismatch.
        if entry.is_some() {
            // If we got a hit, verify it's actually for hash1's data
            // (this would be a collision detection failure)
            panic!(
                "Should not match: stored verify={:08x}, probe verify={:08x}",
                (hash1 >> 32) as u32,
                (hash2 >> 32) as u32
            );
        }
    }

    #[test]
    fn test_replace_if_deeper() {
        let mut tt = TranspositionTable::new(1);
        let hash: u64 = 0xDEAD_BEEF_1234_5678;

        // Store at depth 4
        tt.store(hash, 4, TTFlag::Exact, [100; 4], None);

        // Store at depth 5 (should replace)
        tt.store(hash, 5, TTFlag::Exact, [200; 4], None);
        let entry = tt.probe(hash).unwrap();
        assert_eq!(*entry.scores(), [200; 4]);
        assert_eq!(entry.depth(), 5);
    }

    #[test]
    fn test_no_replace_if_shallower() {
        let mut tt = TranspositionTable::new(1);

        // Two hashes that map to the same index but different verify keys
        // We need hashes where lower bits are equal but upper bits differ
        let mask = tt.mask;
        let base_idx = 42usize & mask;
        let hash1: u64 = 0xAAAA_0000_0000_0000 | base_idx as u64;
        let hash2: u64 = 0xBBBB_0000_0000_0000 | base_idx as u64;

        // Store at depth 5
        tt.store(hash1, 5, TTFlag::Exact, [100; 4], None);

        // Try to store different position at depth 4 (should NOT replace — deeper wins)
        tt.store(hash2, 4, TTFlag::Exact, [200; 4], None);

        // Original should still be there
        let entry = tt.probe(hash1).unwrap();
        assert_eq!(*entry.scores(), [100; 4]);
    }

    #[test]
    fn test_replace_stale_generation() {
        let mut tt = TranspositionTable::new(1);
        let mask = tt.mask;
        let base_idx = 42usize & mask;
        let hash1: u64 = 0xAAAA_0000_0000_0000 | base_idx as u64;
        let hash2: u64 = 0xBBBB_0000_0000_0000 | base_idx as u64;

        // Store at depth 5, generation 0
        tt.store(hash1, 5, TTFlag::Exact, [100; 4], None);

        // New search (bumps generation)
        tt.new_search();

        // Store at depth 1 (would NOT replace normally, but old entry is stale)
        tt.store(hash2, 1, TTFlag::Exact, [200; 4], None);

        // New entry should be there
        let entry = tt.probe(hash2).unwrap();
        assert_eq!(*entry.scores(), [200; 4]);
    }

    #[test]
    fn test_best_move_none() {
        let mut tt = TranspositionTable::new(1);
        let hash: u64 = 0xDEAD_BEEF_1234_5678;

        tt.store(hash, 3, TTFlag::Exact, [50; 4], None);
        let entry = tt.probe(hash).unwrap();
        assert_eq!(entry.best_move(), None);
    }

    #[test]
    fn test_flag_lower_bound() {
        let mut tt = TranspositionTable::new(1);
        let hash: u64 = 0xDEAD_BEEF_1234_5678;

        tt.store(hash, 3, TTFlag::LowerBound, [50; 4], None);
        let entry = tt.probe(hash).unwrap();
        assert_eq!(entry.flag(), TTFlag::LowerBound);
    }

    #[test]
    fn test_flag_upper_bound() {
        let mut tt = TranspositionTable::new(1);
        let hash: u64 = 0xDEAD_BEEF_1234_5678;

        tt.store(hash, 3, TTFlag::UpperBound, [50; 4], None);
        let entry = tt.probe(hash).unwrap();
        assert_eq!(entry.flag(), TTFlag::UpperBound);
    }

    #[test]
    fn test_clear_resets_table() {
        let mut tt = TranspositionTable::new(1);
        let hash: u64 = 0xDEAD_BEEF_1234_5678;

        tt.store(hash, 5, TTFlag::Exact, [100; 4], None);
        assert!(tt.probe(hash).is_some());

        tt.clear();
        assert!(tt.probe(hash).is_none());
    }

    #[test]
    fn test_stats_tracking() {
        let mut tt = TranspositionTable::new(1);
        let hash: u64 = 0xDEAD_BEEF_1234_5678;

        tt.store(hash, 5, TTFlag::Exact, [100; 4], None);

        // Probe hit
        let _ = tt.probe(hash);
        // Probe miss
        let _ = tt.probe(0x1111_2222_3333_4444);

        let (hits, probes) = tt.stats();
        assert_eq!(hits, 1);
        assert_eq!(probes, 2);
    }
}
