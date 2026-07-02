---
summary: The hub document format (an OKF concept superset) and the minimal-diff frontmatter editor used by verify.
anchors:
  - claim: >
      A hub is a `---`-fenced YAML frontmatter block followed by a markdown body, and its
      frontmatter is a superset of an OKF concept: `type` (defaulted to `concept`, so pre-OKF hubs
      stay valid), `title`, `tags`, `timestamp` sit alongside Surface's `anchors`/`refs`/`covers`,
      and every other key (OKF `description`/`resource`, a doc system's `author`/`created`) is
      preserved verbatim in `extra` — unknown *frontmatter* keys are kept, not rejected, per OKF.
      Inside an anchor item `at:` is a scalar or list, `hash` is optional until verified, and
      unknown keys there ARE still rejected (a per-anchor typo fails closed). parse_hub resolves
      neither refs nor covers — acting on them is lint/check's job.
    at:
      - surf-core/src/hub.rs > parse_hub
      - surf-core/src/hub.rs > Frontmatter
      - surf-core/src/hub.rs > Claim
    hash: 2:6f2be9c95177
    id: c_18be38a6388e79780004
    verified_at: 2026-07-01T17:29:13Z
    verified_commit: e9e86af7ce662b0f9b26eb379e952d09d9685c05
  - claim: >
      verify writes fields back surgically: set_anchor_field (which set_anchor_hash wraps) locates
      the Nth anchor item and replaces/inserts only that one key's line, so an unchanged write is
      byte-identical — the same primitive stamps hash, id, and verified_* provenance.
    at:
      - surf-core/src/hub.rs > set_anchor_field
      - surf-core/src/hub.rs > set_anchor_hash
    hash: 2:592b1c643978
    id: c_18be38a639b1f2a80005
    verified_at: 2026-07-01T16:53:09Z
    verified_commit: 7c5aabe74da3b56ff680044aeb3b20747b606479
refs:
  - ./cli-lint.md
  - ./cli-check.md
covers:
  - surf-core/src/hub.rs
---

# Hub format

A hub is the unit every command reads and writes: a `---`-fenced YAML frontmatter block (OKF
concept fields plus Surface's machine-checkable `anchors`) followed by a markdown body (the prose a
human or agent reads). `parse_hub` is the contract everything else binds to.

**A hub is an OKF concept, plus freshness.** The frontmatter is a *superset* of an
[Open Knowledge Format](../docs/guides/okf.md) concept: it carries OKF's `type`/`title`/`tags`/
`timestamp` (and preserves any other key in `extra`, since OKF requires consumers to keep unknown
fields), so a hub is a conformant OKF concept that any OKF reader can consume — while Surface's
`anchors` add the freshness OKF omits. That is why `deny_unknown_fields` is *off* for the
frontmatter (a typo'd key is caught by a `surf lint` warning instead of a hard error) but stays
*on* for each anchor item, where an unknown key is a genuine mistake that should fail closed.
`covers` never gates; a stale `refs` target propagates into the [`check`](./cli-check.md) verdict (#4).

**The distinction that drives the design:** a human reviews every write, so edits must be
*surgical*. Writes go through the line-level editor (`set_anchor_field`, which `set_anchor_hash`
wraps, plus `set_anchor_at`) rather than re-serializing the frontmatter — re-serializing would
reorder keys, reflow scalars, and drop the preserved `extra` ordering, burying the one changed line
in a noisy diff. An unchanged write is therefore byte-identical, which is what keeps a no-op
`surf verify` from churning the file (and what lets it stamp `id`/`verified_*` provenance only when
the hash actually changed).

**Boundary:** this module is pure parsing and text editing — it resolves no anchors and computes no
hashes; it only produces the structure [`lint`](./cli-lint.md)/[`check`](./cli-check.md) act on.
