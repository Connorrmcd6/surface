/**
 * Serialize a flat string map to the canonical string that payloads are signed over.
 * Fields are emitted in INSERTION order, as they appear on the object.
 */
export function canonicalString(fields: Record<string, string>): string {
  const keys = Object.keys(fields);
  return keys.map((k) => `${k}=${fields[k]}`).join('&');
}
