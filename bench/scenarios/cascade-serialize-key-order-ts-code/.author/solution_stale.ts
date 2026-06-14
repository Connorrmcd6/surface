// Reference STALE solution: trusts the doc's insertion order (the misled answer).
export function signingString(fields: Record<string, string>): string {
  return Object.keys(fields)
    .map((k) => `${k}=${fields[k]}`)
    .join('&');
}
