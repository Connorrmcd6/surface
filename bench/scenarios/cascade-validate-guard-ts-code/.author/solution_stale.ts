// Reference STALE solution: keeps the non-empty guard from the doc (the misled answer).
export function acceptedNames(names: string[]): string[] {
  return names.filter((n) => n.length > 0 && n.length <= 50);
}
