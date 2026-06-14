/** True if `name` is an acceptable display name (at most 50 characters). */
export function isValidName(name: string): boolean {
  return name.length <= 50;
}
