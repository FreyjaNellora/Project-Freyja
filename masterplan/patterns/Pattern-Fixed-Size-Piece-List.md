# Pattern: Fixed-Size Piece List

**Discovered:** 2026-03-03
**Stage:** Stage 1
**Category:** Performance

---

## Problem

Chess engines need to iterate over a player's pieces efficiently (e.g., for move generation, evaluation). Using `Vec<T>` incurs heap allocation which is unacceptable in hot paths (ADR-004). Need O(1) add, O(1) remove, and fast iteration without heap allocation.

## Solution

Use a fixed-size array with a count, and swap-remove for O(1) deletion.

```rust
piece_lists: [[Option<(PieceType, Square)>; MAX_PIECES_PER_PLAYER]; PLAYERS],
piece_counts: [u8; PLAYERS],

fn remove_from_piece_list(&mut self, player: Player, sq: Square) -> (PieceType, Square) {
    let idx = player.index();
    let pos = self.find_in_piece_list(player, sq).expect("piece must exist");
    let removed = self.piece_lists[idx][pos].take().unwrap();
    let last = self.piece_counts[idx] as usize - 1;
    if pos != last {
        self.piece_lists[idx][pos] = self.piece_lists[idx][last].take();
    }
    self.piece_counts[idx] -= 1;
    removed
}
```

## When to Use

- Any collection in a hot path that has a known upper bound on size
- Piece lists, move lists, attack info arrays
- Anywhere `Vec<T>` would be the natural choice but heap allocation is prohibited

## When NOT to Use

- Collections with no known upper bound
- Cold paths where clarity matters more than performance
- Large maximum sizes (>256 elements) where the array wastes stack space

## Examples in Codebase

- `Board::piece_lists` in `freyja-engine/src/board/mod.rs` — 32 entries per player
- `AttackInfo::attacker_squares` in `freyja-engine/src/board/attacks.rs` — 16 entries

---

**Related:** [[MASTERPLAN]], [[Component-Board]], [[DECISIONS]] (ADR-004)
