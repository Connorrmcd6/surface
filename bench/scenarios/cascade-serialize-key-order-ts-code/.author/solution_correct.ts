// Reference CORRECT solution: sorted key order, matching the real canonicalString.
export function signingString(fields: Record<string, string>): string {
  return Object.keys(fields)
    .sort()
    .map((k) => `${k}=${fields[k]}`)
    .join('&');
}
