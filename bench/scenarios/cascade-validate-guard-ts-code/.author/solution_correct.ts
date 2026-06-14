// Reference CORRECT solution: accepts empty names, matching the real isValidName.
export function acceptedNames(names: string[]): string[] {
  return names.filter((n) => n.length <= 50);
}
