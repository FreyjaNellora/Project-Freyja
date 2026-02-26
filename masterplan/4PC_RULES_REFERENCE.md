# Four-Player Chess Rules Reference
## Per chess.com Implementation — Freyja Engine Specification

**Version:** 1.0
**Created:** 2026-02-25
**Status:** Active
**Heritage:** Derived from Project Odin 4PC_RULES_REFERENCE, with expanded coordinate precision for Freyja.

---

## 1. BOARD GEOMETRY

### 1.1 Dimensions

- **Grid:** 14 rows (ranks) x 14 columns (files)
- **Total squares:** 196
- **Invalid squares:** 36 (four 3x3 corners removed)
- **Valid (playable) squares:** 160
- **Shape:** Cross — an 8x8 center with 3-column extensions on all four sides

### 1.2 Coordinate System

**Display coordinates** (what humans and protocols use):
- Files: `a` through `n` (14 columns, left to right)
- Ranks: `1` through `14` (14 rows, bottom to top)
- Square notation: `<file><rank>` — e.g., `d1`, `a7`, `n11`

**Internal coordinates** (what the engine uses):
- File index: `a`=0, `b`=1, `c`=2, ..., `n`=13
- Rank index: `1`=0, `2`=1, `3`=2, ..., `14`=13
- **Square index formula:** `rank_index * 14 + file_index`
- Range: 0 to 195 (196 total, 36 are invalid)

**Conversion:**
```
display_file = (char)('a' + file_index)
display_rank = rank_index + 1
file_index   = display_file - 'a'
rank_index   = display_rank - 1
square_index = rank_index * 14 + file_index
```

### 1.3 Invalid Corner Squares — EXHAUSTIVE LIST

Every square in each 3x3 corner is invalid. Pieces cannot exist on, move to, or move through these squares.

**Southwest corner** (files a-c, ranks 1-3):

| Display | File | Rank | Index |
|---------|------|------|-------|
| a1 | 0 | 0 | 0 |
| b1 | 1 | 0 | 1 |
| c1 | 2 | 0 | 2 |
| a2 | 0 | 1 | 14 |
| b2 | 1 | 1 | 15 |
| c2 | 2 | 1 | 16 |
| a3 | 0 | 2 | 28 |
| b3 | 1 | 2 | 29 |
| c3 | 2 | 2 | 30 |

**Southeast corner** (files l-n, ranks 1-3):

| Display | File | Rank | Index |
|---------|------|------|-------|
| l1 | 11 | 0 | 11 |
| m1 | 12 | 0 | 12 |
| n1 | 13 | 0 | 13 |
| l2 | 11 | 1 | 25 |
| m2 | 12 | 1 | 26 |
| n2 | 13 | 1 | 27 |
| l3 | 11 | 2 | 39 |
| m3 | 12 | 2 | 40 |
| n3 | 13 | 2 | 41 |

**Northwest corner** (files a-c, ranks 12-14):

| Display | File | Rank | Index |
|---------|------|------|-------|
| a12 | 0 | 11 | 154 |
| b12 | 1 | 11 | 155 |
| c12 | 2 | 11 | 156 |
| a13 | 0 | 12 | 168 |
| b13 | 1 | 12 | 169 |
| c13 | 2 | 12 | 170 |
| a14 | 0 | 13 | 182 |
| b14 | 1 | 13 | 183 |
| c14 | 2 | 13 | 184 |

**Northeast corner** (files l-n, ranks 12-14):

| Display | File | Rank | Index |
|---------|------|------|-------|
| l12 | 11 | 11 | 165 |
| m12 | 12 | 11 | 166 |
| n12 | 13 | 11 | 167 |
| l13 | 11 | 12 | 179 |
| m13 | 12 | 12 | 180 |
| n13 | 13 | 12 | 181 |
| l14 | 11 | 13 | 193 |
| m14 | 12 | 13 | 194 |
| n14 | 13 | 13 | 195 |

**Validity check (pseudocode):**
```rust
fn is_valid_square(rank: u8, file: u8) -> bool {
    if rank > 13 || file > 13 { return false; }
    // Corners: rank < 3 AND file < 3 (SW), rank < 3 AND file > 10 (SE),
    //          rank > 10 AND file < 3 (NW), rank > 10 AND file > 10 (NE)
    let in_sw = rank < 3 && file < 3;
    let in_se = rank < 3 && file > 10;
    let in_nw = rank > 10 && file < 3;
    let in_ne = rank > 10 && file > 10;
    !(in_sw || in_se || in_nw || in_ne)
}
```

### 1.4 Board Diagram (starting position)

```
    a   b   c   d   e   f   g   h   i   j   k   l   m   n
14  .   .   .   rR  yN  yB  yK  yQ  yB  yN  yR  .   .   .   14
13  .   .   .   yP  yP  yP  yP  yP  yP  yP  yP  .   .   .   13
12  .   .   .                                       .   .   .   12
11  bR  bP                                          gP  gR  11
10  bN  bP                                          gP  gN  10
 9  bB  bP                                          gP  gB   9
 8  bQ  bP                                          gP  gQ   8
 7  bK  bP                                          gP  gK   7
 6  bB  bP                                          gP  gB   6
 5  bN  bP                                          gP  gN   5
 4  bR  bP                                          gP  gR   4
 3  .   .   .                                       .   .   .    3
 2  .   .   .   rP  rP  rP  rP  rP  rP  rP  rP  .   .   .    2
 1  .   .   .   rR  rN  rB  rQ  rK  rB  rN  rR  .   .   .    1
    a   b   c   d   e   f   g   h   i   j   k   l   m   n
```

Legend: `r`=Red, `b`=Blue, `y`=Yellow, `g`=Green. `.`=invalid corner. Empty valid squares are blank.

---

## 2. PLAYERS

### 2.1 Player Definitions

| Player | Color | Side | Index | Push Direction |
|--------|-------|------|-------|---------------|
| Red | Red | South (bottom) | 0 | North (+rank) |
| Blue | Blue | West (left) | 1 | East (+file) |
| Yellow | Yellow | North (top) | 2 | South (-rank) |
| Green | Green | East (right) | 3 | West (-file) |

### 2.2 Turn Order

**Red → Blue → Yellow → Green → Red → ...** (clockwise)

When a player is eliminated, their turn is skipped. The rotation continues with the remaining active players.

---

## 3. STARTING POSITIONS — EXACT SQUARES

### 3.1 Red (South)

**Back rank (rank 1):**

| Square | File | Rank | Index | Piece |
|--------|------|------|-------|-------|
| d1 | 3 | 0 | 3 | Rook |
| e1 | 4 | 0 | 4 | Knight |
| f1 | 5 | 0 | 5 | Bishop |
| g1 | 6 | 0 | 6 | **Queen** |
| h1 | 7 | 0 | 7 | **King** |
| i1 | 8 | 0 | 8 | Bishop |
| j1 | 9 | 0 | 9 | Knight |
| k1 | 10 | 0 | 10 | Rook |

**Pawn rank (rank 2):**

| Square | File | Rank | Index | Piece |
|--------|------|------|-------|-------|
| d2 | 3 | 1 | 17 | Pawn |
| e2 | 4 | 1 | 18 | Pawn |
| f2 | 5 | 1 | 19 | Pawn |
| g2 | 6 | 1 | 20 | Pawn |
| h2 | 7 | 1 | 21 | Pawn |
| i2 | 8 | 1 | 22 | Pawn |
| j2 | 9 | 1 | 23 | Pawn |
| k2 | 10 | 1 | 24 | Pawn |

**Order: R N B Q K B N R** (Queen left of King from Red's perspective)

### 3.2 Blue (West)

**Back rank (file a, ranks 4-11):**

| Square | File | Rank | Index | Piece |
|--------|------|------|-------|-------|
| a4 | 0 | 3 | 42 | Rook |
| a5 | 0 | 4 | 56 | Knight |
| a6 | 0 | 5 | 70 | Bishop |
| a7 | 0 | 6 | 84 | **King** |
| a8 | 0 | 7 | 98 | **Queen** |
| a9 | 0 | 8 | 112 | Bishop |
| a10 | 0 | 9 | 126 | Knight |
| a11 | 0 | 10 | 140 | Rook |

**Pawn file (file b, ranks 4-11):**

| Square | File | Rank | Index | Piece |
|--------|------|------|-------|-------|
| b4 | 1 | 3 | 43 | Pawn |
| b5 | 1 | 4 | 57 | Pawn |
| b6 | 1 | 5 | 71 | Pawn |
| b7 | 1 | 6 | 85 | Pawn |
| b8 | 1 | 7 | 99 | Pawn |
| b9 | 1 | 8 | 113 | Pawn |
| b10 | 1 | 9 | 127 | Pawn |
| b11 | 1 | 10 | 141 | Pawn |

**Order: R N B K Q B N R** (King and Queen swapped vs Red — maintains "queen on her own color" from Blue's perspective)

### 3.3 Yellow (North)

**Back rank (rank 14, files d-k):**

| Square | File | Rank | Index | Piece |
|--------|------|------|-------|-------|
| d14 | 3 | 13 | 185 | Rook |
| e14 | 4 | 13 | 186 | Knight |
| f14 | 5 | 13 | 187 | Bishop |
| g14 | 6 | 13 | 188 | **King** |
| h14 | 7 | 13 | 189 | **Queen** |
| i14 | 8 | 13 | 190 | Bishop |
| j14 | 9 | 13 | 191 | Knight |
| k14 | 10 | 13 | 192 | Rook |

**Pawn rank (rank 13, files d-k):**

| Square | File | Rank | Index | Piece |
|--------|------|------|-------|-------|
| d13 | 3 | 12 | 171 | Pawn |
| e13 | 4 | 12 | 172 | Pawn |
| f13 | 5 | 12 | 173 | Pawn |
| g13 | 6 | 12 | 174 | Pawn |
| h13 | 7 | 12 | 175 | Pawn |
| i13 | 8 | 12 | 176 | Pawn |
| j13 | 9 | 12 | 177 | Pawn |
| k13 | 10 | 12 | 178 | Pawn |

**Order: R N B K Q B N R** (King and Queen swapped vs Red — mirrors Blue's convention)

### 3.4 Green (East)

**Back rank (file n, ranks 4-11):**

| Square | File | Rank | Index | Piece |
|--------|------|------|-------|-------|
| n4 | 13 | 3 | 55 | Rook |
| n5 | 13 | 4 | 69 | Knight |
| n6 | 13 | 5 | 83 | Bishop |
| n7 | 13 | 6 | 97 | **Queen** |
| n8 | 13 | 7 | 111 | **King** |
| n9 | 13 | 8 | 125 | Bishop |
| n10 | 13 | 9 | 139 | Knight |
| n11 | 13 | 10 | 153 | Rook |

**Pawn file (file m, ranks 4-11):**

| Square | File | Rank | Index | Piece |
|--------|------|------|-------|-------|
| m4 | 12 | 3 | 54 | Pawn |
| m5 | 12 | 4 | 68 | Pawn |
| m6 | 12 | 5 | 82 | Pawn |
| m7 | 12 | 6 | 96 | Pawn |
| m8 | 12 | 7 | 110 | Pawn |
| m9 | 12 | 8 | 124 | Pawn |
| m10 | 12 | 9 | 138 | Pawn |
| m11 | 12 | 10 | 152 | Pawn |

**Order: R N B Q K B N R** (same as Red — Queen left of King from Green's perspective)

### 3.5 King/Queen Position Summary

| Player | King Square | King Index | Queen Square | Queen Index | K/Q Order |
|--------|-------------|-----------|--------------|-------------|-----------|
| Red | h1 | 7 | g1 | 6 | Q then K (left to right) |
| Blue | a7 | 84 | a8 | 98 | K then Q (bottom to top) |
| Yellow | g14 | 188 | h14 | 189 | K then Q (left to right) |
| Green | n8 | 111 | n7 | 97 | Q then K (bottom to top) |

**Rule:** Blue and Yellow have King and Queen positions swapped relative to Red and Green. This maintains the "queen on her own color" convention when viewed from each player's perspective.

---

## 4. PIECE TYPES

### 4.1 Piece Enumeration

| Piece | Enum Value | Eval (cp) | Capture (FFA pts) | Notes |
|-------|-----------|----------|-------------------|-------|
| Pawn | 0 | 100 | 1 | Direction depends on player |
| Knight | 1 | 300 | 3 | Standard L-shape, corner-aware |
| Bishop | 2 | 350 | 5 | Diagonal slider, corner-aware |
| Rook | 3 | 500 | 5 | Orthogonal slider |
| Queen | 4 | 900 | 9 | All 8 directions |
| King | 5 | N/A | 0 (DKW) | 1 step any direction |
| PromotedQueen | 6 | 900 | **1** | Moves like queen, worth 1pt on capture |

**Critical dual-value:** PromotedQueen has eval weight 900cp (for search/NNUE) but capture value of only 1 FFA point. These are TWO SEPARATE SCORING SYSTEMS that must not be conflated.

### 4.2 Standard Movement

All pieces move according to standard chess rules on the 160-square board:
- **Rook:** Horizontal and vertical sliding
- **Bishop:** Diagonal sliding
- **Queen:** All 8 directions sliding
- **Knight:** L-shaped jump (2+1), landing must be valid
- **King:** 1 step in any of 8 directions
- **Pawn:** See Section 5

Sliders are blocked by any piece and by invalid corner squares. Knights that would land on an invalid corner have that move removed (not just blocked — the square doesn't exist).

---

## 5. PAWN RULES — ALL 4 ORIENTATIONS

### 5.1 Push Directions

| Player | Push Direction | Delta (rank, file) | Example |
|--------|---------------|-------------------|---------|
| Red | North (+rank) | (+1, 0) | d2 → d3 |
| Blue | East (+file) | (0, +1) | b4 → c4 |
| Yellow | South (-rank) | (-1, 0) | d13 → d12 |
| Green | West (-file) | (0, -1) | m4 → l4 |

### 5.2 Capture Directions

Each pawn captures one step forward and one step to either side (diagonal from the player's perspective):

| Player | Capture 1 (delta rank, file) | Capture 2 (delta rank, file) |
|--------|------------------------------|------------------------------|
| Red | (+1, +1) — northeast | (+1, -1) — northwest |
| Blue | (+1, +1) — northeast | (-1, +1) — southeast |
| Yellow | (-1, +1) — southeast | (-1, -1) — southwest |
| Green | (+1, -1) — northwest | (-1, -1) — southwest |

**Pattern:** Each capture direction is the push direction combined with one perpendicular step.

### 5.3 Double Step

Pawns may advance two squares on their first move (from starting rank/file only):

| Player | Starting Rank/File | From → To Example | Condition |
|--------|-------------------|-------------------|-----------|
| Red | Rank 2 (index 1) | d2 → d4 | Rank index == 1 AND file 3-10 |
| Blue | File b (index 1) | b4 → d4 | File index == 1 AND rank 3-10 |
| Yellow | Rank 13 (index 12) | d13 → d11 | Rank index == 12 AND file 3-10 |
| Green | File m (index 12) | m4 → k4 | File index == 12 AND rank 3-10 |

The intermediate square must be empty (same as standard chess).

### 5.4 Promotion

**FFA Mode:** Pawns promote on the 8th rank/file from their starting position (effectively the middle of the board).

| Player | Promotion Condition | Promotion Rank/File | Index |
|--------|--------------------|--------------------|-------|
| Red | Rank == 9 | Rank 9 (index 8) | rank_index == 8 |
| Blue | File == i | File i (index 8) | file_index == 8 |
| Yellow | Rank == 6 | Rank 6 (index 5) | rank_index == 5 |
| Green | File == f | File f (index 5) | file_index == 5 |

**Teams Mode:** Promote on the 11th rank/file from starting position.

| Player | Promotion Rank/File (Teams) | Index |
|--------|----------------------------|-------|
| Red | Rank 12 (index 11) | rank_index == 11 |
| Blue | File l (index 11) | file_index == 11 |
| Yellow | Rank 3 (index 2) | rank_index == 2 |
| Green | File c (index 2) | file_index == 2 |

**Default promotion:** PromotedQueen (moves like queen, worth 1 FFA point on capture).

**Underpromotion available:** Knight, Bishop, Rook.

### 5.5 En Passant

When a pawn makes a double-step, an en passant (EP) target square is set. The EP square is the square the double-stepping pawn passed through.

| Player | Double-Step Example | EP Target Square |
|--------|--------------------|--------------------|
| Red | d2 → d4 | d3 (rank index 2) |
| Blue | b7 → d7 | c7 (file index 2) |
| Yellow | h13 → h11 | h12 (rank index 11) |
| Green | m5 → k5 | l5 (file index 10) |

**EP capture rules:**
- An opposing pawn adjacent to the double-stepped pawn (on the landing rank/file) may capture by moving to the EP target square
- The capturing pawn's capture direction must include the EP target square
- **EP expires after 1 ply** (only the very next player to move may capture EP; if they don't, EP is cleared)
- The Board stores: `en_passant: Option<Square>` (the target square) and `en_passant_pushing_player: Option<Player>` (which player created the EP opportunity)
- EP target is cleared whenever ANY move is made (standard 1-ply rule)

**Any opponent may capture en passant**, not just the immediately adjacent players. If Blue double-steps and creates an EP target, Green (moving next after Yellow, if Blue moved before Yellow) may be too late — EP was cleared after Yellow's move.

---

## 6. CASTLING — ALL 8 VARIANTS

### 6.1 General Rules

Standard chess castling rules apply per player:
1. King and rook must not have moved previously
2. No pieces between king and rook
3. King must not be in check
4. King must not pass through a square attacked by any opponent
5. King must not land on a square attacked by any opponent
6. **3-opponent check:** all THREE opponents' attacks must be considered, not just one

### 6.2 Castling Rights Encoding

8 bits total: 2 bits per player (kingside, queenside).

```
Bit 0: Red kingside
Bit 1: Red queenside
Bit 2: Blue kingside
Bit 3: Blue queenside
Bit 4: Yellow kingside
Bit 5: Yellow queenside
Bit 6: Green kingside
Bit 7: Green queenside
```

### 6.3 Castling Paths — Exact Squares

#### Red Kingside (O-O)

| | Square | Index |
|---|--------|-------|
| King from | h1 | 7 |
| King to | j1 | 9 |
| Rook from | k1 | 10 |
| Rook to | i1 | 8 |
| Must be empty | i1, j1 | 8, 9 |
| King passes through | h1, i1, j1 | 7, 8, 9 |

#### Red Queenside (O-O-O)

| | Square | Index |
|---|--------|-------|
| King from | h1 | 7 |
| King to | f1 | 5 |
| Rook from | d1 | 3 |
| Rook to | g1 | 6 |
| Must be empty | e1, f1, g1 | 4, 5, 6 |
| King passes through | h1, g1, f1 | 7, 6, 5 |

#### Blue Kingside (O-O)

| | Square | Index |
|---|--------|-------|
| King from | a7 | 84 |
| King to | a5 | 56 |
| Rook from | a4 | 42 |
| Rook to | a6 | 70 |
| Must be empty | a5, a6 | 56, 70 |
| King passes through | a7, a6, a5 | 84, 70, 56 |

#### Blue Queenside (O-O-O)

| | Square | Index |
|---|--------|-------|
| King from | a7 | 84 |
| King to | a9 | 112 |
| Rook from | a11 | 140 |
| Rook to | a8 | 98 |
| Must be empty | a8, a9, a10 | 98, 112, 126 |
| King passes through | a7, a8, a9 | 84, 98, 112 |

#### Yellow Kingside (O-O)

| | Square | Index |
|---|--------|-------|
| King from | g14 | 188 |
| King to | e14 | 186 |
| Rook from | d14 | 185 |
| Rook to | f14 | 187 |
| Must be empty | e14, f14 | 186, 187 |
| King passes through | g14, f14, e14 | 188, 187, 186 |

#### Yellow Queenside (O-O-O)

| | Square | Index |
|---|--------|-------|
| King from | g14 | 188 |
| King to | i14 | 190 |
| Rook from | k14 | 192 |
| Rook to | h14 | 189 |
| Must be empty | h14, i14, j14 | 189, 190, 191 |
| King passes through | g14, h14, i14 | 188, 189, 190 |

#### Green Kingside (O-O)

| | Square | Index |
|---|--------|-------|
| King from | n8 | 111 |
| King to | n10 | 139 |
| Rook from | n11 | 153 |
| Rook to | n9 | 125 |
| Must be empty | n9, n10 | 125, 139 |
| King passes through | n8, n9, n10 | 111, 125, 139 |

#### Green Queenside (O-O-O)

| | Square | Index |
|---|--------|-------|
| King from | n8 | 111 |
| King to | n6 | 83 |
| Rook from | n4 | 55 |
| Rook to | n7 | 97 |
| Must be empty | n5, n6, n7 | 69, 83, 97 |
| King passes through | n8, n7, n6 | 111, 97, 83 |

### 6.4 Castling Summary Table

| Player | Side | King Move | Rook Move | Empty Squares | Axis |
|--------|------|-----------|-----------|---------------|------|
| Red | KS | h1→j1 | k1→i1 | i1, j1 | Horizontal |
| Red | QS | h1→f1 | d1→g1 | e1, f1, g1 | Horizontal |
| Blue | KS | a7→a5 | a4→a6 | a5, a6 | Vertical |
| Blue | QS | a7→a9 | a11→a8 | a8, a9, a10 | Vertical |
| Yellow | KS | g14→e14 | d14→f14 | e14, f14 | Horizontal |
| Yellow | QS | g14→i14 | k14→h14 | h14, i14, j14 | Horizontal |
| Green | KS | n8→n10 | n11→n9 | n9, n10 | Vertical |
| Green | QS | n8→n6 | n4→n7 | n5, n6, n7 | Vertical |

**Note:** Blue and Green castle vertically (along a file). Red and Yellow castle horizontally (along a rank). Kingside for each player is toward the rook on the king's side of the starting arrangement.

---

## 7. CHECK, CHECKMATE, AND ELIMINATION

### 7.1 Check

- Any player can check any opponent's king
- A single move can check multiple kings simultaneously (bonus points in FFA)
- **Three-way check obligation:** when checking if a player is in check, ALL THREE opponents' attacks must be considered. Missing even one opponent's attack = critical bug.

### 7.2 Checkmate Timing (Critical)

**Checkmate is NOT confirmed when the checking move is made.** It is confirmed when the affected player's turn arrives and they have no legal moves while in check.

Between the check and the affected player's turn, **intervening players may alter the position** — blocking the check, capturing the checking piece, or otherwise rescuing the king. This is unique to multi-player chess and must be handled correctly.

### 7.3 Stalemate

A player is stalemated when it is their turn, they are NOT in check, and they have no legal moves. In FFA mode, stalemate awards **+20 points** to the stalemated player (not zero, not a draw).

### 7.4 Elimination

A player is eliminated by:
1. **Checkmate** — When their turn arrives and they are in check with no legal moves
2. **Stalemate** — When their turn arrives and they are not in check with no legal moves
3. **Resignation** — Player voluntarily leaves
4. **Timeout** — Player runs out of time

When eliminated:
- Player's turn is skipped in future rotations
- King square set to 255 sentinel (not stale position value)
- `generate_legal_moves` must NEVER be called for an eliminated player (= crash on kingless board)

---

## 8. DEAD KING WALKING (DKW)

When a player resigns or times out (NOT checkmate/stalemate):
1. Their pieces turn grey, worth **0 points** on capture
2. Their king remains "live" and makes **random instant moves** (including captures, but earns 0 points)
3. DKW king moves happen **BETWEEN turns**, not as a full turn (timing critical)
4. Checkmating a dead king still awards points to the checkmating player
5. Dead kings cannot earn points

**Processing order:** DKW moves execute BEFORE elimination checks each turn. (Odin lesson: getting this order wrong causes desynchronized game state.)

---

## 9. SCORING SYSTEM (FFA)

### 9.1 Capture Points

| Captured Piece | Points |
|----------------|--------|
| Pawn | +1 |
| Knight | +3 |
| Bishop | +5 |
| Rook | +5 |
| Queen (original) | +9 |
| PromotedQueen (1-pt queen) | +1 |
| Dead/grey piece | 0 |

### 9.2 Event Points

| Event | Points |
|-------|--------|
| Checkmate active king | +20 |
| Self-stalemate | +20 (to stalemated player) |
| Check 2 live kings (1 move) | +1 |
| Check 3 live kings (1 move) | +5 |
| Draw (repetition/50-move/insufficient) | +10 each |

### 9.3 Points vs Eval

**These are two separate systems:**

| System | Purpose | Unit | Used By |
|--------|---------|------|---------|
| FFA Points | Game scoring, win/loss | integer points | GameState, Protocol, UI |
| Eval (centipawns) | Search evaluation | i16 centipawns | Evaluator, Searcher, NNUE |

A PromotedQueen is worth 900cp in eval (it moves like a queen) but only 1 point in FFA scoring (it was a pawn). Do not confuse these systems.

---

## 10. GAME END CONDITIONS (FFA)

1. **Three players eliminated:** Game ends. Winner = player with most points (last standing or highest score).
2. **Two players remain, one leads by 21+ points:** "Claim Win" available. The leading player may claim victory.
3. **Autoclaim:** When the eliminated 2nd-place player leads 3rd-place by 21+ points, autoclaim triggers.
4. **Draw conditions:** Threefold repetition, 50-move rule (no captures/pawn moves), insufficient material.

---

## 11. TERRAIN MODE (Freyja-Specific Game Variant)

When a player is eliminated (by any means), their remaining pieces stay on the board permanently as immovable terrain:
- Cannot be captured or moved by any player
- Block movement as if they were walls (sliders cannot pass through)
- The eliminated **king is REMOVED** (not converted to terrain)
- This is a `GameMode` variant, not the default behavior

---

## 12. CHESS960 ADAPTATION

- Back-rank pieces randomized per standard Chess960 constraints:
  - Bishops on opposite colors
  - King between rooks
- All four players receive the **same randomization**, rotated to their respective sides
- Castling rules follow Chess960 conventions (king moves to standard castling target square regardless of starting position)
- This is a future game mode variant

---

## 13. 4PC VERIFICATION MATRIX TEMPLATE

Every rule in this document must be independently tested for all 4 players. Use this matrix as a tracking tool during implementation.

| Rule | Red | Blue | Yellow | Green |
|------|-----|------|--------|-------|
| Pawn push direction | ☐ | ☐ | ☐ | ☐ |
| Pawn capture NE diagonal | ☐ | ☐ | ☐ | ☐ |
| Pawn capture NW/SE diagonal | ☐ | ☐ | ☐ | ☐ |
| Pawn double step | ☐ | ☐ | ☐ | ☐ |
| Pawn double step blocked | ☐ | ☐ | ☐ | ☐ |
| Pawn promotion rank | ☐ | ☐ | ☐ | ☐ |
| Pawn promotion piece | ☐ | ☐ | ☐ | ☐ |
| En passant creation | ☐ | ☐ | ☐ | ☐ |
| En passant capture | ☐ | ☐ | ☐ | ☐ |
| En passant expiry (1 ply) | ☐ | ☐ | ☐ | ☐ |
| Castling kingside | ☐ | ☐ | ☐ | ☐ |
| Castling queenside | ☐ | ☐ | ☐ | ☐ |
| Castling blocked by piece | ☐ | ☐ | ☐ | ☐ |
| Castling blocked by check | ☐ | ☐ | ☐ | ☐ |
| Castling through check | ☐ | ☐ | ☐ | ☐ |
| Castling rights revoked (king move) | ☐ | ☐ | ☐ | ☐ |
| Castling rights revoked (rook move) | ☐ | ☐ | ☐ | ☐ |
| Castling rights revoked (rook captured) | ☐ | ☐ | ☐ | ☐ |
| King square tracking | ☐ | ☐ | ☐ | ☐ |
| Attack detection (as attacker) | ☐ | ☐ | ☐ | ☐ |
| Attack detection (as defender) | ☐ | ☐ | ☐ | ☐ |
| Check detection (3-opponent) | ☐ | ☐ | ☐ | ☐ |
| Checkmate detection | ☐ | ☐ | ☐ | ☐ |
| Stalemate detection | ☐ | ☐ | ☐ | ☐ |
| Elimination handling | ☐ | ☐ | ☐ | ☐ |
| Turn skip after elimination | ☐ | ☐ | ☐ | ☐ |
| DKW king movement | ☐ | ☐ | ☐ | ☐ |
| Knight on corner edge | ☐ | ☐ | ☐ | ☐ |
| Slider blocked by corner | ☐ | ☐ | ☐ | ☐ |
| Starting position correct | ☐ | ☐ | ☐ | ☐ |
| Zobrist side-to-move (4 values) | ☐ | ☐ | ☐ | ☐ |
| FEN4 round-trip | ☐ | ☐ | ☐ | ☐ |

**Rule:** Every ☐ must become ✅ with a dedicated test before a stage using that rule is marked complete. Not one test that loops — four explicit tests per row, one per player.

---

## 14. NAMED CONSTANTS

These constants must be defined in the engine and used everywhere. No magic numbers.

| Constant | Value | Meaning |
|----------|-------|---------|
| `BOARD_SIZE` | 14 | Rank/file count |
| `TOTAL_SQUARES` | 196 | 14 × 14 |
| `VALID_SQUARES` | 160 | 196 - 36 |
| `INVALID_CORNER_COUNT` | 36 | 4 × 9 |
| `CORNER_SIZE` | 3 | 3×3 corners |
| `PLAYERS` | 4 | Number of players |
| `PIECES_PER_PLAYER` | 16 | 8 back rank + 8 pawns |
| `MAX_PIECES_PER_PLAYER` | 32 | Including promotions (theoretical max) |
| `ELIMINATED_KING_SENTINEL` | 255 | King square value when eliminated |
| `CASTLING_RIGHTS_BITS` | 8 | 2 per player × 4 players |
| `CHECKMATE_POINTS` | 20 | FFA points for checkmating a king |
| `STALEMATE_POINTS` | 20 | FFA points for self-stalemate |
| `CLAIM_WIN_THRESHOLD` | 21 | Point lead needed to claim win |
| `PAWN_EVAL` | 100 | Centipawn value |
| `KNIGHT_EVAL` | 300 | Centipawn value |
| `BISHOP_EVAL` | 350 | Centipawn value |
| `ROOK_EVAL` | 500 | Centipawn value |
| `QUEEN_EVAL` | 900 | Centipawn value |
| `MAX_GAME_LENGTH` | 1024 | Maximum half-moves before forced draw |
| `MAX_QSEARCH_DEPTH` | 8 | Quiescence depth cap |

---

*End of 4PC Rules Reference v1.0*
