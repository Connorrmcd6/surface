import { test } from 'node:test';
import assert from 'node:assert/strict';
import { signingString } from '../code/signer.ts';
import { canonicalString } from '../code/sign/canonical.ts';

// Correct (T1) behaviour: recover the canonical ordering from the real (hidden) canonicalizer and
// assert signingString reproduces it. canonicalString now sorts keys, so {b,a,c} -> "a=1&b=2&c=3".
test('signingString reproduces the real canonical (sorted) order', () => {
  const fields = { b: '2', a: '1', c: '3' };
  const expected = canonicalString(fields);

  // fixture sanity: the ordering must actually differ from insertion order
  const insertion = Object.keys(fields).map((k) => `${k}=${fields[k]}`).join('&');
  assert.notEqual(expected, insertion);

  assert.equal(signingString({}), '');
  assert.equal(signingString(fields), expected);
});
