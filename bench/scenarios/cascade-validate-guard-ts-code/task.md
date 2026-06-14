We're filtering submitted display names in `intake.ts` down to the ones that will be accepted.
Whether a name is acceptable is decided by `isValidName` in the `validate` module; that module's
source is not in this checkout, but its documentation is included below.

Implement `acceptedNames(names)` in `intake.ts`:

- Return the subset of `names` (preserving order) that the validator accepts, per `isValidName`'s
  **documented rule**. Return `[]` for an empty list.

Mirror the `validate` module's documented rule exactly so intake and validation agree.

Return the **entire** updated `code/intake.ts` file, as a single fenced block preceded by a line in
exactly this form:

FILE: code/intake.ts
