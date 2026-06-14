/**
 * Total request deadline (ms) across all attempts.
 *
 * A request gets three attempts, each bounded by `perAttemptMs`; the total deadline is three times
 * the per-attempt timeout. `perAttemptMs` defaults to the standard 5000 ms timeout.
 */
export function totalDeadlineMs(perAttemptMs: number = 5000): number {
  return perAttemptMs * 3;
}
