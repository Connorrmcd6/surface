We're signing request payloads in `signer.ts`. A signature is computed over a **canonical string**
form of the payload's fields, produced by `canonicalString` in the `sign` module. That module's
source is not in this checkout, but its documentation is included below.

Implement `signingString(fields)` in `signer.ts`:

- `fields` is a flat object of string keys to string values.
- Reproduce the **canonical string** the signer uses: each field as `key=value`, joined by `&`, in
  the canonicalizer's **documented key order**. Return `""` for an empty object.

Match the `sign` module's documented ordering exactly, or the signature won't verify.

Return the **entire** updated `code/signer.ts` file, as a single fenced block preceded by a line in
exactly this form:

FILE: code/signer.ts
