/**
 * Builds the string a request payload is signed over.
 *
 * The canonical serialization is defined by `canonicalString` in the `sign` module, whose source is
 * not in this checkout (see its documentation). This module must reproduce that exact string so the
 * signature it produces verifies on the other side.
 */
export function signingString(fields: Record<string, string>): string {
  // Should reproduce the canonical serialization of `fields` (same key ordering as the signer's
  // canonicalString), as `key=value` pairs joined by '&'.
  throw new Error('not implemented');
}
