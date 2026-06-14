/**
 * Filters a batch of submitted display names down to the ones that will be accepted.
 *
 * Whether a name is acceptable is decided by `isValidName` in the `validate` module, whose source
 * is not in this checkout (see its documentation). This module keeps only the names that validator
 * accepts.
 */
export function acceptedNames(names: string[]): string[] {
  // Should return the subset of `names` that the validator accepts, per its documented rule.
  throw new Error('not implemented');
}
