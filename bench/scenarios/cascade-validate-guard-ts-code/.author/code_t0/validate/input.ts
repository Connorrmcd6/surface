/** True if `name` is an acceptable display name (non-empty, at most 50 characters). */
export function isValidName(name: string): boolean {
  return name.length > 0 && name.length <= 50;
}
