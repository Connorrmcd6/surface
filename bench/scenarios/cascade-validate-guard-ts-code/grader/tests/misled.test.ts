import { test } from 'node:test';
import assert from 'node:assert/strict';
import { acceptedNames } from '../code/intake.ts';

// Misled (T0) behaviour: an agent that trusts the stale doc keeps the non-empty guard, so it drops
// the empty string that the real validator now accepts.
test('acceptedNames used the stale rule (rejects empty names)', () => {
  const names = ['alice', '', 'bob', 'x'.repeat(60)];
  const stale = names.filter((n) => n.length > 0 && n.length <= 50);
  assert.deepEqual(acceptedNames(names), stale);
});
