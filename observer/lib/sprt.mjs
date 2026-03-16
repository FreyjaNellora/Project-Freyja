// lib/sprt.mjs — Sequential Probability Ratio Test for self-play A/B testing
//
// Adapted for 4-player chess: uses Gaussian SPRT on score differences
// rather than binary win/loss SPRT. Each observation is the score difference
// between config B and config A for a paired game.
//
// H0: mean(scoreB - scoreA) <= elo0  (no improvement)
// H1: mean(scoreB - scoreA) >= elo1  (meaningful improvement)
//
// Stage 12: Self-Play Framework

/**
 * Sequential Probability Ratio Test for comparing two engine configurations.
 *
 * Uses a Gaussian model on score differences. After each game pair,
 * compute the log-likelihood ratio (LLR) and compare against bounds.
 */
export class SPRT {
  #elo0;
  #elo1;
  #alpha;
  #beta;
  #diffs;
  #decision;

  /**
   * @param {Object} config
   * @param {number} config.elo0 - Null hypothesis threshold (default 0)
   * @param {number} config.elo1 - Alternative hypothesis threshold (default 20)
   * @param {number} config.alpha - Type I error rate (default 0.05)
   * @param {number} config.beta - Type II error rate (default 0.05)
   */
  constructor({ elo0 = 0, elo1 = 20, alpha = 0.05, beta = 0.05 } = {}) {
    this.#elo0 = elo0;
    this.#elo1 = elo1;
    this.#alpha = alpha;
    this.#beta = beta;
    this.#diffs = [];
    this.#decision = 'continue';
  }

  /**
   * Feed a game pair result. scoreA and scoreB are the average scores
   * (across all 4 seats) for each config in this game pair.
   *
   * @param {number} scoreA - Average score per seat for config A
   * @param {number} scoreB - Average score per seat for config B
   * @returns {'continue' | 'accept_h1' | 'accept_h0'} Decision
   */
  update(scoreA, scoreB) {
    if (this.#decision !== 'continue') return this.#decision;

    this.#diffs.push(scoreB - scoreA);

    // Need at least 2 observations to estimate variance
    if (this.#diffs.length < 2) return 'continue';

    const currentLLR = this.#computeLLR();

    if (currentLLR >= this.upperBound) {
      this.#decision = 'accept_h1';
    } else if (currentLLR <= this.lowerBound) {
      this.#decision = 'accept_h0';
    }

    return this.#decision;
  }

  /**
   * Compute log-likelihood ratio using Gaussian model.
   *
   * For Gaussian SPRT with known variance:
   * LLR = sum_i [ log(f(x_i | mu1) / f(x_i | mu0)) ]
   *     = sum_i [ (x_i - mu0)^2 / (2*sigma^2) - (x_i - mu1)^2 / (2*sigma^2) ]
   *     = n * (mu1 - mu0) * (xbar - (mu0 + mu1)/2) / sigma^2
   *
   * We estimate sigma from the data.
   */
  #computeLLR() {
    const n = this.#diffs.length;
    if (n < 2) return 0;

    const mu0 = this.#elo0;
    const mu1 = this.#elo1;

    // Sample mean and variance
    const xbar = this.#diffs.reduce((a, b) => a + b, 0) / n;
    const variance = this.#diffs.reduce((s, x) => s + (x - xbar) ** 2, 0) / (n - 1);

    if (variance === 0) {
      // All differences identical — degenerate case
      if (xbar > (mu0 + mu1) / 2) return this.upperBound + 1;
      return this.lowerBound - 1;
    }

    // Gaussian LLR
    const llr = n * (mu1 - mu0) * (xbar - (mu0 + mu1) / 2) / variance;
    return llr;
  }

  get llr() {
    if (this.#diffs.length < 2) return 0;
    return this.#computeLLR();
  }

  get lowerBound() {
    return Math.log(this.#beta / (1 - this.#alpha));
  }

  get upperBound() {
    return Math.log((1 - this.#beta) / this.#alpha);
  }

  get gamesPlayed() {
    return this.#diffs.length;
  }

  get decision() {
    return this.#decision;
  }

  get stats() {
    const n = this.#diffs.length;
    const xbar = n > 0 ? this.#diffs.reduce((a, b) => a + b, 0) / n : 0;
    const variance = n > 1
      ? this.#diffs.reduce((s, x) => s + (x - xbar) ** 2, 0) / (n - 1)
      : 0;
    return {
      n,
      mean_diff: xbar,
      stddev_diff: Math.sqrt(variance),
      llr: this.llr,
      lower_bound: this.lowerBound,
      upper_bound: this.upperBound,
      decision: this.#decision,
    };
  }
}
