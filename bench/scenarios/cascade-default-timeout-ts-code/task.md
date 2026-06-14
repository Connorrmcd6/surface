We're showing a "this may take up to…" hint in `caller.ts`. The hint is the default total request
budget, computed by `totalDeadlineMs` in the `net` module. That module's source is not in this
checkout, but its documentation is included below.

Implement `requestBudgetMs()` in `caller.ts`:

- Return the total request deadline in **milliseconds** that the net layer uses **by default** —
  i.e. `totalDeadlineMs()` called with no per-attempt override.

Use the net module's documented default per-attempt timeout so the budget matches what requests
actually use.

Return the **entire** updated `code/caller.ts` file, as a single fenced block preceded by a line in
exactly this form:

FILE: code/caller.ts
