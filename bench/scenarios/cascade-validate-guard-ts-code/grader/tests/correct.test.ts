import { test } from 'node:test';
import assert from 'node:assert/strict';
import { acceptedNames } from '../code/intake.ts';
import { isValidName } from '../code/validate/input.ts';

// Correct (T1) behaviour: filter through the real (hidden) validator and assert acceptedNames
// matches. The empty-name guard was dropped, so "" is now accepted.
test('acceptedNames matches the real validator (accepts empty names)', () => {
  const names = ['alice', '', 'bob', 'x'.repeat(60)];
  const expected = names.filter(isValidName);

  // fixture sanity: real and stale rules must differ (they diverge on the empty string)
  const stale = names.filter((n) => n.length > 0 && n.length <= 50);
  assert.notDeepEqual(expected, stale);

  assert.deepEqual(acceptedNames([]), []);
  assert.deepEqual(acceptedNames(names), expected);
});
