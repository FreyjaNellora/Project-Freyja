#!/bin/bash
cd "$(dirname "$0")"
RESULTS_DIR="reports/experiment_results"
mkdir -p "$RESULTS_DIR"

echo "=== Starting A/B experiments at $(date) ===" > "$RESULTS_DIR/log.txt"

# Experiment 1: opponent ratio 0.25 vs 0.5
echo "[$(date)] Starting opponent ratio experiment..." >> "$RESULTS_DIR/log.txt"
node ab_runner.mjs config_ab_oppratio_d4.json 2>&1 | grep -v "^\[2m" > "$RESULTS_DIR/oppratio_output.txt" 2>&1
EXIT1=$?
echo "[$(date)] Opponent ratio experiment finished with exit code $EXIT1" >> "$RESULTS_DIR/log.txt"

# Experiment 2: beam 30 vs 15
echo "[$(date)] Starting beam width experiment..." >> "$RESULTS_DIR/log.txt"
node ab_runner.mjs config_ab_beam_d4.json 2>&1 | grep -v "^\[2m" > "$RESULTS_DIR/beam_output.txt" 2>&1
EXIT2=$?
echo "[$(date)] Beam width experiment finished with exit code $EXIT2" >> "$RESULTS_DIR/log.txt"

echo "=== All experiments done at $(date) ===" >> "$RESULTS_DIR/log.txt"
echo "Exit codes: oppratio=$EXIT1, beam=$EXIT2" >> "$RESULTS_DIR/log.txt"

# Signal completion
echo "DONE" > "$RESULTS_DIR/COMPLETE"
