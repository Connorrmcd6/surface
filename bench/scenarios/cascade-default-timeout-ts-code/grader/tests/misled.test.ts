import { test } from 'node:test';
import assert from 'node:assert/strict';
import { requestBudgetMs } from '../code/caller.ts';

// Misled (T0) behaviour: an agent that trusts the stale doc assumes a 30000 ms default per-attempt
// timeout, so it reports 90000 ms instead of the real 15000 ms.
test('requestBudgetMs used the stale default timeout', () => {
  assert.equal(requestBudgetMs(), 30000 * 3);
});
