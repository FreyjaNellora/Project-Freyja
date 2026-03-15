#!/bin/bash
# Automated game observer for eval tuning
# Plays a full game by piping protocol commands to the engine binary
# Usage: bash tools/observer.sh [movetime_ms] [plies] [binary]
#   movetime_ms: time per move in milliseconds (default 5000 = 5s)
#   plies: number of half-moves to play (default 40)
#   binary: path to engine binary (default target/release/freyja.exe)

MOVETIME=${1:-5000}
PLIES=${2:-40}
ENGINE=${3:-"target/release/freyja.exe"}
# Per-move timeout: movetime + generous buffer for position replay
TIMEOUT=$(( (MOVETIME / 1000) + 30 ))

PLAYERS=("Red" "Blue" "Yellow" "Green")
MOVES=""
MOVE_LIST=()

echo "=== Freyja Observer ==="
echo "Engine: $ENGINE"
echo "Movetime: ${MOVETIME}ms | Plies: $PLIES | Timeout: ${TIMEOUT}s"
echo "========================"
echo ""

for ((ply=0; ply<PLIES; ply++)); do
    PLAYER=${PLAYERS[$((ply % 4))]}

    # Build protocol input
    if [ -z "$MOVES" ]; then
        POS_CMD="position startpos"
    else
        POS_CMD="position startpos moves $MOVES"
    fi

    INPUT="freyja\n${POS_CMD}\ngo movetime ${MOVETIME}\nquit\n"

    # Run engine and capture output
    OUTPUT=$(printf "$INPUT" | timeout "$TIMEOUT" "$ENGINE" 2>/dev/null)
    EXIT_CODE=$?

    # Extract bestmove
    BESTMOVE=$(echo "$OUTPUT" | grep "^bestmove" | awk '{print $2}')

    if [ $EXIT_CODE -eq 124 ]; then
        echo "Ply $ply: $PLAYER TIMEOUT (>${TIMEOUT}s)"
        break
    fi

    if [ -z "$BESTMOVE" ] || [ "$BESTMOVE" = "(none)" ]; then
        echo "Ply $ply: $PLAYER has no move (game over)"
        break
    fi

    # Extract info line for scores and depth
    INFO=$(echo "$OUTPUT" | grep "^info depth" | tail -1)
    DEPTH_HIT=$(echo "$INFO" | sed -n 's/^info depth \([0-9]*\).*/\1/p')
    SCORES=$(echo "$INFO" | sed -n 's/.*score red \([^ ]*\) blue \([^ ]*\) yellow \([^ ]*\) green \([^ ]*\).*/R=\1 B=\2 Y=\3 G=\4/p')
    NPS=$(echo "$INFO" | sed -n 's/.*nps \([0-9]*\).*/\1/p')

    # Accumulate moves
    if [ -z "$MOVES" ]; then
        MOVES="$BESTMOVE"
    else
        MOVES="$MOVES $BESTMOVE"
    fi
    MOVE_LIST+=("$BESTMOVE")

    echo "Ply $ply: $PLAYER plays $BESTMOVE  (d=$DEPTH_HIT $SCORES nps=$NPS)"
done

echo ""
echo "=== Game Summary ==="
echo "Total plies: ${#MOVE_LIST[@]}"
echo ""
echo "=== Round-by-Round ==="
TOTAL=${#MOVE_LIST[@]}
ROUND=1
for ((i=0; i<TOTAL; i+=4)); do
    R_MOVE="${MOVE_LIST[$i]:-—}"
    B_MOVE="${MOVE_LIST[$((i+1))]:-—}"
    Y_MOVE="${MOVE_LIST[$((i+2))]:-—}"
    G_MOVE="${MOVE_LIST[$((i+3))]:-—}"
    printf "  Round %2d: Red=%-8s Blue=%-8s Yellow=%-8s Green=%-8s\n" "$ROUND" "$R_MOVE" "$B_MOVE" "$Y_MOVE" "$G_MOVE"
    ((ROUND++))
done
