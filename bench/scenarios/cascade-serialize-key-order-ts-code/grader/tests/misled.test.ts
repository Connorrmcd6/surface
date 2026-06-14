import { test } from 'node:test';
import assert from 'node:assert/strict';
import { signingString } from '../code/signer.ts';

// Misled (T0) behaviour: an agent that trusts the stale doc emits fields in insertion order,
// so {b,a,c} -> "b=2&a=1&c=3" instead of the real sorted "a=1&b=2&c=3".
test('signingString used the stale insertion order', () => {
  const fields = { b: '2', a: '1', c: '3' };
  const insertion = Object.keys(fields).map((k) => `${k}=${fields[k]}`).join('&');
  assert.equal(signingString(fields), insertion);
});
