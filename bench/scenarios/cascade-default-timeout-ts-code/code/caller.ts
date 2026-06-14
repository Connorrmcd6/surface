/**
 * Reports the default request budget for the UI's "this may take up to…" hint.
 *
 * The total deadline is computed by `totalDeadlineMs` in the `net` module, whose source is not in
 * this checkout (see its documentation). This module reports the default budget — the deadline when
 * no per-attempt override is given.
 */
export function requestBudgetMs(): number {
  // Should return the total request deadline the net layer uses by default (no per-attempt
  // override) — i.e. totalDeadlineMs() called with no argument.
  throw new Error('not implemented');
}
