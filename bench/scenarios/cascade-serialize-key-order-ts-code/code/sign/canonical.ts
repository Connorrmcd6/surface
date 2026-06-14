/**
 * Serialize a flat string map to the canonical string that payloads are signed over.
 * Fields are emitted in SORTED key order so the signer and verifier always agree.
 */
export function canonicalString(fields: Record<string, string>): string {
  const keys = Object.keys(fields).sort();
  return keys.map((k) => `${k}=${fields[k]}`).join('&');
}
