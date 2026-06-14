import { test } from 'node:test';
import assert from 'node:assert/strict';
import { requestBudgetMs } from '../code/caller.ts';
import { totalDeadlineMs } from '../code/net/timeout.ts';

// Correct (T1) behaviour: recover the default budget from the real (hidden) net helper and assert
// requestBudgetMs matches it. The default per-attempt timeout is now 5000 ms -> total 15000 ms.
test('requestBudgetMs uses the real default per-attempt timeout', () => {
  const expected = totalDeadlineMs(); // real default 5000 -> 15000

  // fixture sanity: the default drift must move the budget
  assert.notEqual(expected, 30000 * 3);

  assert.equal(requestBudgetMs(), expected);
});
