//! `surf lint` (§9.1.2): every anchor must resolve to exactly one symbol. Ambiguous or
//! vanished anchors block; a symbol that was merely renamed (detected via stored-hash
//! match, §6.4) only warns and points at `surf verify --follow`. It also emits advisory
//! granularity warnings (§8): anchors that span (nearly) a whole file, hubs with too many
//! anchors, exported symbols in an anchored file that no claim covers, and the symmetric
//! consolidation nudges (#142) — a per-symbol "claim-log" and a thin-prose body.

use crate::format::Format;
use crate::workspace::Workspace;
use anyhow::Result;
use serde::Serialize;
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::process::ExitCode;
use surf_core::{
    find_renamed, parse_anchor, public_symbols, resolve, HashOpts, Lang, ResolveError, Surface,
};

/// Over an anchored span this fraction of its file, the anchor is "whole-file-ish" and any
/// edit re-triggers verification — the over-anchoring tension of §8.
const COARSE_SPAN_FRACTION_PCT: usize = 75;
const COARSE_MIN_FILE_LINES: usize = 15;
/// Past this many anchors a hub invites rubber-stamping during a bulk `verify` (§8).
const MAX_ANCHORS_PER_HUB: usize = 12;
/// At or above this many claims, a hub that never once uses a multi-site `at:` list reads as a
/// per-symbol "claim-log" rather than a system briefing — nudge toward consolidation (#142).
const CLAIM_LOG_MIN_CLAIMS: usize = 4;
/// An onboarding hub should average at least this many words of *body* prose per claim. Below it,
/// the prose lives in the frontmatter and the body is a stub — flag thin-prose (#142).
const MIN_PROSE_WORDS_PER_CLAIM: usize = 15;

#[derive(Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Severity {
    Block,
    Warn,
}

#[derive(Debug, Serialize)]
pub struct Finding {
    pub severity: Severity,
    pub hub: String,
    pub at: String,
    pub message: String,
    pub claim: String,
}

pub fn run(ws: &Workspace, format: Format) -> Result<ExitCode> {
    let findings = lint_workspace(ws)?;
    let blocks = findings
        .iter()
        .filter(|f| f.severity == Severity::Block)
        .count();
    let warns = findings.len() - blocks;

    match format {
        Format::Json => println!("{}", serde_json::to_string_pretty(&findings)?),
        Format::Human => print_human(&findings, blocks, warns),
    }

    Ok(if blocks > 0 {
        ExitCode::FAILURE
    } else {
        ExitCode::SUCCESS
    })
}

fn print_human(findings: &[Finding], blocks: usize, warns: usize) {
    for f in findings {
        let tag = match f.severity {
            Severity::Block => "error",
            Severity::Warn => "warning",
        };
        println!("{tag}: {} :: {}", f.hub, f.at);
        println!("    {}", f.message);
        // Coverage warnings have no claim by definition — a bare `claim:` label reads
        // like truncated output (#83).
        if !f.claim.is_empty() {
            println!("    claim: {}", truncate(&f.claim, 80));
        }
    }

    if findings.is_empty() {
        println!("surf lint: all anchors resolve.");
    } else {
        println!("surf lint: {blocks} error(s), {warns} warning(s).");
    }
}

fn lint_workspace(ws: &Workspace) -> Result<Vec<Finding>> {
    let mut findings = Vec::new();

    // Coverage is a workspace property, not a per-hub one: a public symbol anchored by *any* hub
    // is covered, so a second hub that merely touches the same file must not be nagged about the
    // symbols another hub owns (#54). Accumulate across every hub, then run the under-coverage
    // nudge once at the end. Keyed by file:
    //   - `covered`   — the full segment path of each resolved anchor, so anchoring one method
    //                   doesn't mark its siblings covered (the same exactness `suggest` uses, #29).
    //   - `unhealthy` — files with an unresolved anchor anywhere; the nudge skips them, since
    //                   piling coverage nags onto a broken file would just be noise.
    //   - `owner`     — the lexicographically-first hub anchoring the file, which the file's
    //                   nudge is attributed to, so each uncovered symbol is reported once rather
    //                   than once per hub touching the file.
    let mut covered: HashMap<String, HashSet<Vec<String>>> = HashMap::new();
    let mut unhealthy: HashSet<String> = HashSet::new();
    let mut owner: HashMap<String, String> = HashMap::new();

    // `refs` validation needs every other hub on hand, so load once and index the well-formed
    // hubs by rel. A malformed hub is absent from the index (it gets its own block below), so a
    // ref into it reads as "does not resolve to a hub" — which it effectively doesn't.
    let loaded = ws.iter_hubs()?;
    // Reserved OKF files (index.md/log.md) are not concepts: they never carry claims and a `ref`
    // can't target them, so keep them out of the concept index and skip them below.
    let hub_index: HashMap<&str, &surf_core::Hub> = loaded
        .iter()
        .filter(|l| l.kind == surf_core::DocKind::Concept)
        .filter_map(|l| l.hub.as_ref().ok().map(|h| (l.rel.as_str(), h)))
        .collect();

    for loaded_hub in &loaded {
        if loaded_hub.kind != surf_core::DocKind::Concept {
            continue;
        }
        let rel = loaded_hub.rel.as_str();
        let hub = match &loaded_hub.hub {
            Ok(hub) => hub,
            Err(e) => {
                findings.push(Finding {
                    severity: Severity::Block,
                    hub: rel.to_string(),
                    claim: String::new(),
                    at: String::new(),
                    message: format!("invalid hub: {e}"),
                });
                continue;
            }
        };

        for claim in &hub.frontmatter.anchors {
            for site in claim.at.sites() {
                let outcome = lint_site(
                    ws,
                    rel,
                    &claim.claim,
                    site,
                    claim.hash.as_deref(),
                    HashOpts {
                        ignore_literals: claim.ignore_literals,
                    },
                    &mut findings,
                );
                if let Some(info) = outcome {
                    owner
                        .entry(info.file.clone())
                        .and_modify(|h| {
                            if rel < h.as_str() {
                                *h = rel.to_string();
                            }
                        })
                        .or_insert_with(|| rel.to_string());
                    if info.resolved {
                        covered.entry(info.file).or_default().insert(info.segments);
                    } else {
                        unhealthy.insert(info.file);
                    }
                }
            }
        }

        lint_covers(rel, hub, &mut findings);
        lint_refs(rel, hub, &hub_index, &mut findings);
        lint_claim_log(rel, hub, &mut findings);
        lint_thin_prose(rel, hub, &mut findings);
        lint_okf_frontmatter(rel, hub, &mut findings);
        lint_okf_links(ws, rel, hub, &mut findings);

        if hub.frontmatter.anchors.len() > MAX_ANCHORS_PER_HUB {
            findings.push(Finding {
                severity: Severity::Warn,
                hub: rel.to_string(),
                claim: String::new(),
                at: String::new(),
                message: format!(
                    "{} anchors in one hub (> {MAX_ANCHORS_PER_HUB}) — consider splitting; bulk verify of a long list invites rubber-stamping",
                    hub.frontmatter.anchors.len()
                ),
            });
        }
    }

    // Under-coverage, workspace-wide: for each anchored, healthy file, warn for public symbols no
    // hub covers. Sorted by file for deterministic output.
    let mut files: Vec<(&String, &String)> = owner.iter().collect();
    files.sort();
    let empty = HashSet::new();
    for (file, hub) in files {
        if unhealthy.contains(file) {
            continue;
        }
        let cov = covered.get(file).unwrap_or(&empty);
        lint_under_coverage(ws, hub, file, cov, &mut findings);
    }

    lint_agents_pointer(ws, &mut findings);
    Ok(findings)
}

/// §11.6: `AGENTS.md` (imperative agent instructions) must point at the hubs *directory* and
/// tell the agent to search it — not duplicate hub prose, and not enumerate every hub (which
/// would push an agent to read everything). Opt-in: enforced only when the file carries a
/// `<!-- surf:hubs -->` … `<!-- /surf:hubs -->` block. The block must link the configured hubs
/// directory, and that directory must exist.
fn lint_agents_pointer(ws: &Workspace, findings: &mut Vec<Finding>) {
    const OPEN: &str = "<!-- surf:hubs -->";
    const CLOSE: &str = "<!-- /surf:hubs -->";

    let Ok(text) = std::fs::read_to_string(ws.root.join("AGENTS.md")) else {
        return; // no AGENTS.md → nothing to enforce
    };
    let Some(block) = text
        .split_once(OPEN)
        .and_then(|(_, rest)| rest.split_once(CLOSE))
        .map(|(block, _)| block)
    else {
        return; // no pointer block → opt-out
    };

    let dir = crate::new::hub_dir(&ws.config.hubs);
    let dir_str = dir.to_string_lossy();
    let want = dir_str.trim_end_matches('/');

    let links_dir = link_targets(block).any(|t| {
        let t = t.trim_start_matches("./").trim_end_matches('/');
        t == want
    });

    if !links_dir || !ws.root.join(&dir).is_dir() {
        findings.push(Finding {
            severity: Severity::Block,
            hub: "AGENTS.md".to_string(),
            claim: String::new(),
            at: String::new(),
            message: format!(
                "`surf:hubs` block must link the hubs directory `{want}/` and it must exist — agents read it to find context"
            ),
        });
    }
}

/// Validate a hub's advisory `covers` globs (§9.1). The verdict never reads `covers`, so this
/// is the only place a malformed glob can be caught — a bad pattern blocks (silently dropping it
/// would let a typo'd scope go unnoticed, the same reasoning as `--files` in `check`, #38). When
/// the globs are well-formed, warn for any of the hub's own anchored files that none of them
/// match: a hub whose `covers` excludes its own anchors is almost certainly a fat-fingered glob.
fn lint_covers(rel: &str, hub: &surf_core::Hub, findings: &mut Vec<Finding>) {
    if hub.frontmatter.covers.is_empty() {
        return;
    }

    let mut patterns = Vec::new();
    for raw in &hub.frontmatter.covers {
        match glob::Pattern::new(raw) {
            Ok(p) => patterns.push(p),
            Err(e) => findings.push(Finding {
                severity: Severity::Block,
                hub: rel.to_string(),
                claim: String::new(),
                at: raw.clone(),
                message: format!("invalid `covers` glob \"{raw}\": {e}"),
            }),
        }
    }
    if patterns.len() != hub.frontmatter.covers.len() {
        return; // a glob didn't compile — don't run the coverage nudge on a partial pattern set
    }

    // The hub's own anchored files (deduped, sorted for deterministic output).
    let mut anchored: Vec<String> = hub
        .frontmatter
        .anchors
        .iter()
        .flat_map(|c| c.at.sites())
        .filter_map(|s| parse_anchor(s).ok().map(|a| a.file))
        .collect();
    anchored.sort();
    anchored.dedup();

    for file in anchored {
        if !patterns.iter().any(|p| p.matches(&file)) {
            findings.push(Finding {
                severity: Severity::Warn,
                hub: rel.to_string(),
                claim: String::new(),
                at: file.clone(),
                message: format!(
                    "anchored file `{file}` is not matched by any `covers` glob — check the globs cover this hub's own anchors"
                ),
            });
        }
    }
}

/// Validate a hub's `refs` composition (§9.3, #4). Each entry names another hub by a path
/// relative to this one, optionally `> segment` to address a claim within it. A ref that doesn't
/// resolve to a loaded hub, points at its own hub, or names a claim no anchor in the target
/// matches is a structural error and blocks — the same fail-on-typo reasoning as `covers`. The
/// verdict does not read `refs` yet (PR2), so lint is the only thing that acts on them.
fn lint_refs(
    rel: &str,
    hub: &surf_core::Hub,
    hub_index: &HashMap<&str, &surf_core::Hub>,
    findings: &mut Vec<Finding>,
) {
    for raw in &hub.frontmatter.refs {
        let mut block = |message: String| {
            findings.push(Finding {
                severity: Severity::Block,
                hub: rel.to_string(),
                claim: String::new(),
                at: raw.clone(),
                message,
            });
        };

        let parsed = match surf_core::parse_ref(raw) {
            Ok(r) => r,
            Err(e) => {
                block(format!("invalid `refs` entry \"{raw}\": {e}"));
                continue;
            }
        };

        let target_rel = crate::workspace::resolve_ref_path(rel, &parsed.path);
        if target_rel == rel {
            block(format!("ref \"{raw}\" points at its own hub"));
            continue;
        }
        let Some(target) = hub_index.get(target_rel.as_str()) else {
            block(format!(
                "ref \"{raw}\" does not resolve to a hub (looked for `{target_rel}`) — `refs` compose hubs, not arbitrary files"
            ));
            continue;
        };

        if !parsed.segments.is_empty() {
            let names: Vec<&str> = parsed.segments.iter().map(|s| s.name.as_str()).collect();
            let matched = target.frontmatter.anchors.iter().any(|c| {
                c.at.sites().iter().any(|site| {
                    parse_anchor(site).is_ok_and(|a| {
                        let anchor_names: Vec<&str> =
                            a.segments.iter().map(|s| s.name.as_str()).collect();
                        anchor_names.ends_with(&names)
                    })
                })
            });
            if !matched {
                block(format!(
                    "ref \"{raw}\" names a claim `{}` that no anchor in `{target_rel}` matches",
                    names.join(" > ")
                ));
            }
        }
    }
}

/// OKF cross-links are plain markdown links between concepts. OKF **tolerates** broken links (they
/// may be not-yet-written knowledge), so this only ever *warns* — it never blocks. Checks local
/// `.md` links (bundle-relative `/x.md` resolved from the workspace root, or relative `./x.md` from
/// the hub's directory), skipping URLs, bare `#anchors`, and non-markdown assets. Best-effort: for a
/// bundle mounted in a subdirectory, an absolute `/x.md` may resolve against the wrong root, so a
/// spurious warning is possible — never a block.
fn lint_okf_links(ws: &Workspace, rel: &str, hub: &surf_core::Hub, findings: &mut Vec<Finding>) {
    for raw in link_targets(&hub.body) {
        let path = raw.split('#').next().unwrap_or(raw).trim();
        if path.is_empty()
            || path.contains("://")
            || path.starts_with("mailto:")
            || path.starts_with("//")
            || !path.ends_with(".md")
        {
            continue;
        }
        let target = match path.strip_prefix('/') {
            Some(abs) => abs.to_string(),
            None => crate::workspace::resolve_ref_path(rel, path),
        };
        if target.is_empty() || ws.root.join(&target).is_file() {
            continue;
        }
        findings.push(Finding {
            severity: Severity::Warn,
            hub: rel.to_string(),
            claim: String::new(),
            at: raw.to_string(),
            message: format!(
                "OKF cross-link `{raw}` points at `{target}`, which doesn't exist — fine if it's not-yet-written, else check for a typo (advisory; OKF tolerates broken links)"
            ),
        });
    }
}

/// Markdown link targets (`](target)`) in a fragment of text.
fn link_targets(text: &str) -> impl Iterator<Item = &str> {
    text.split("](")
        .skip(1)
        .filter_map(|after| after.split_once(')').map(|(target, _)| target.trim()))
}

/// What `lint_site` learned about one anchor site: which file it names, the full segment path it
/// anchors (e.g. `["Builder", "Set"]`), and whether it resolved cleanly. `None` when the site
/// can't even be attributed to a file (unparseable).
struct SiteInfo {
    file: String,
    segments: Vec<String>,
    resolved: bool,
}

fn lint_site(
    ws: &Workspace,
    hub: &str,
    claim: &str,
    site: &str,
    stored_hash: Option<&str>,
    opts: HashOpts,
    findings: &mut Vec<Finding>,
) -> Option<SiteInfo> {
    let mut block = |message: String| {
        findings.push(Finding {
            severity: Severity::Block,
            hub: hub.to_string(),
            claim: claim.to_string(),
            at: site.to_string(),
            message,
        });
    };

    let anchor = match parse_anchor(site) {
        Ok(a) => a,
        Err(e) => {
            block(format!("invalid anchor: {e}"));
            return None;
        }
    };
    let segments: Vec<String> = anchor.segments.iter().map(|s| s.name.clone()).collect();
    let unresolved = |resolved: bool| {
        Some(SiteInfo {
            file: anchor.file.clone(),
            segments: segments.clone(),
            resolved,
        })
    };

    let Some(lang) = Lang::from_path(&anchor.file) else {
        block(format!("unsupported file type: {}", anchor.file));
        return unresolved(false);
    };
    let path: PathBuf = ws.root.join(&anchor.file);
    let source = match std::fs::read_to_string(&path) {
        Ok(s) => s,
        Err(_) => {
            // A moved file is recoverable: if git recognizes the rename, warn and point at
            // `--follow` rather than hard-blocking (best-effort; the gate itself is unaffected).
            if crate::git::renamed_to(&ws.root, &anchor.file).is_some() {
                findings.push(Finding {
                    severity: Severity::Warn,
                    hub: hub.to_string(),
                    claim: claim.to_string(),
                    at: site.to_string(),
                    message: format!(
                        "`{}` appears to have moved — run `surf verify --follow`",
                        anchor.file
                    ),
                });
            } else {
                block(format!(
                    "cannot read `{}` (file moved or removed?)",
                    anchor.file
                ));
            }
            return unresolved(false);
        }
    };

    match resolve(&source, lang, &anchor) {
        Ok(span) => {
            lint_coarse_span(hub, claim, site, &anchor.file, &source, span, findings);
            unresolved(true)
        }
        Err(ResolveError::Ambiguous { segment, count }) => {
            block(format!(
                "`{segment}` is ambiguous ({count} matches); disambiguate with `@N`"
            ));
            unresolved(false)
        }
        Err(ResolveError::Parse) => {
            block(format!("could not parse `{}`", anchor.file));
            unresolved(false)
        }
        Err(ResolveError::NotFound { segment }) => {
            match stored_hash {
                Some(h) => match find_renamed(&source, lang, h, opts) {
                    Ok(Some(new_name)) => findings.push(Finding {
                        severity: Severity::Warn,
                        hub: hub.to_string(),
                        claim: claim.to_string(),
                        at: site.to_string(),
                        message: format!(
                            "`{segment}` not found, but its code appears to live under `{new_name}` now — run `surf verify --follow`"
                        ),
                    }),
                    Ok(None) => block(format!("`{segment}` not found and no current symbol matches the stored hash — the claim points at nothing")),
                    Err(e) => block(format!("`{segment}` not found; rename check failed: {e}")),
                },
                None => {
                    // First-time authoring: a brand-new anchor that doesn't resolve. If it looks
                    // like the (undiscoverable) `Class > method` chain spelled flat, hint it (#68).
                    let hint = surf_core::suggest_chain(&source, lang, &anchor)
                        .map(|chain| format!(" — did you mean `{chain}`?"))
                        .unwrap_or_default();
                    block(format!(
                        "`{segment}` not found (claim has no stored hash to match against){hint}"
                    ))
                }
            }
            unresolved(false)
        }
    }
}

fn lint_coarse_span(
    hub: &str,
    claim: &str,
    site: &str,
    file: &str,
    source: &str,
    span: surf_core::Span,
    findings: &mut Vec<Finding>,
) {
    let span_lines = span.end_line.saturating_sub(span.start_line) + 1;
    let file_lines = source.lines().count().max(1);
    if file_lines >= COARSE_MIN_FILE_LINES
        && span_lines * 100 >= file_lines * COARSE_SPAN_FRACTION_PCT
    {
        let pct = span_lines * 100 / file_lines;
        findings.push(Finding {
            severity: Severity::Warn,
            hub: hub.to_string(),
            claim: claim.to_string(),
            at: site.to_string(),
            message: format!(
                "anchored span covers {pct}% of {file} ({span_lines}/{file_lines} lines) — a near-whole-file anchor re-triggers verification on any edit; point at a narrower symbol"
            ),
        });
    }
}

/// §8/#142: the counter-pressure to under-coverage. A hub is an onboarding doc — prose-first,
/// with coarse claims that each seal one behavior across the several places it lives (a multi-site
/// `at:` list). When a hub accumulates many claims and *never once* consolidates with a multi-site
/// `at:`, it reads as a per-symbol "claim-log"; nudge toward fewer, multi-anchor claims. Advisory.
fn lint_claim_log(rel: &str, hub: &surf_core::Hub, findings: &mut Vec<Finding>) {
    let claims = &hub.frontmatter.anchors;
    if claims.len() < CLAIM_LOG_MIN_CLAIMS {
        return;
    }
    let multi_site = claims.iter().filter(|c| c.at.sites().len() > 1).count();
    if multi_site == 0 {
        findings.push(Finding {
            severity: Severity::Warn,
            hub: rel.to_string(),
            claim: String::new(),
            at: String::new(),
            message: format!(
                "{} claims, all single-site — this reads as a per-symbol claim-log. A hub documents a system: consolidate related claims into fewer coarse ones, each listing every site it spans under one multi-site `at:`",
                claims.len()
            ),
        });
    }
}

/// §8/#142: a hub is an onboarding doc, not a frontmatter dump. Flag a multi-claim hub whose body
/// prose is too thin to onboard a reader — when the prose lives in the `claim:` fields and the
/// readable body is a stub. Advisory; single-claim hubs (short module notes) are exempt.
fn lint_thin_prose(rel: &str, hub: &surf_core::Hub, findings: &mut Vec<Finding>) {
    let claims = hub.frontmatter.anchors.len();
    if claims < 2 {
        return;
    }
    let words = prose_words(&hub.body);
    if words < MIN_PROSE_WORDS_PER_CLAIM * claims {
        findings.push(Finding {
            severity: Severity::Warn,
            hub: rel.to_string(),
            claim: String::new(),
            at: String::new(),
            message: format!(
                "thin prose: {words} words of body for {claims} claims — a hub is an onboarding doc, not a list of claims. Add prose framing the system (the key distinction, how the pieces fit, what it does *not* cover)"
            ),
        });
    }
}

/// OKF/round-trip advisories on a concept's frontmatter. Relaxing `deny_unknown_fields` (so OKF's
/// "consumers MUST preserve unknown keys" rule holds) means a typo'd top-level key no longer
/// hard-blocks — recover that signal as a warning. Also nudge an anchored hub with no
/// human-readable headline: a hub is an onboarding doc, so a reader needs something to orient on.
fn lint_okf_frontmatter(rel: &str, hub: &surf_core::Hub, findings: &mut Vec<Finding>) {
    const KNOWN_KEYS: [&str; 8] = [
        "anchors",
        "refs",
        "covers",
        "summary",
        "title",
        "tags",
        "timestamp",
        "type",
    ];
    for (k, _) in &hub.frontmatter.extra {
        let Some(key) = k.as_str() else { continue };
        if let Some(hit) = KNOWN_KEYS
            .iter()
            .find(|known| within_edit_distance_1(key, known))
        {
            findings.push(Finding {
                severity: Severity::Warn,
                hub: rel.to_string(),
                claim: String::new(),
                at: String::new(),
                message: format!(
                    "unknown frontmatter key `{key}` — did you mean `{hit}`? (unknown keys are preserved for OKF interop, so a typo no longer hard-blocks the gate)"
                ),
            });
        }
    }

    if !hub.frontmatter.anchors.is_empty()
        && hub.frontmatter.summary.is_none()
        && hub.frontmatter.title.is_none()
        && !hub.frontmatter.extra.contains_key("description")
    {
        findings.push(Finding {
            severity: Severity::Warn,
            hub: rel.to_string(),
            claim: String::new(),
            at: String::new(),
            message:
                "anchored hub has no headline (`summary`, `title`, or `description`) — a hub is an onboarding doc; give readers something to orient on"
                    .to_string(),
        });
    }
}

/// True when `a` and `b` are one edit apart — a single insert, delete, substitution, or adjacent
/// transposition. Transposition matters: `anchros` → `anchors` is the classic fat-fingered key and
/// is only distance 1 once swaps count. Identical strings return `false` (nothing to warn about).
fn within_edit_distance_1(a: &str, b: &str) -> bool {
    a != b && osa_distance(a, b) == 1
}

/// Optimal string alignment distance (Levenshtein plus adjacent transposition). Frontmatter keys
/// are short, so the full DP is negligible.
fn osa_distance(a: &str, b: &str) -> usize {
    let a: Vec<char> = a.chars().collect();
    let b: Vec<char> = b.chars().collect();
    let (n, m) = (a.len(), b.len());
    let mut d = vec![vec![0usize; m + 1]; n + 1];
    for (i, row) in d.iter_mut().enumerate() {
        row[0] = i;
    }
    d[0] = (0..=m).collect();
    for i in 1..=n {
        for j in 1..=m {
            let cost = usize::from(a[i - 1] != b[j - 1]);
            let mut best = (d[i - 1][j] + 1)
                .min(d[i][j - 1] + 1)
                .min(d[i - 1][j - 1] + cost);
            if i > 1 && j > 1 && a[i - 1] == b[j - 2] && a[i - 2] == b[j - 1] {
                best = best.min(d[i - 2][j - 2] + 1);
            }
            d[i][j] = best;
        }
    }
    d[n][m]
}

/// Words of readable body prose, excluding fenced code blocks (``` … ```), which carry no
/// onboarding prose and would otherwise inflate the count.
fn prose_words(body: &str) -> usize {
    let mut count = 0;
    let mut in_fence = false;
    for line in body.lines() {
        if line.trim_start().starts_with("```") {
            in_fence = !in_fence;
            continue;
        }
        if !in_fence {
            count += line.split_whitespace().count();
        }
    }
    count
}

fn lint_under_coverage(
    ws: &Workspace,
    hub: &str,
    file: &str,
    covered: &HashSet<Vec<String>>,
    findings: &mut Vec<Finding>,
) {
    let Some(lang) = Lang::from_path(file) else {
        return;
    };
    let Ok(source) = std::fs::read_to_string(ws.root.join(file)) else {
        return;
    };
    // `public_symbols` measures the behaviour-bearing surface — top-level functions *and* the
    // methods that make up most of a Python/Go API (#54) — not just top-level fns. The nudge is
    // about behaviour that can drift, so it stays on `Callables`; classes/constants (suggest's
    // `--all`) are deliberately excluded here.
    for sym in public_symbols(&source, lang, Surface::Callables) {
        if !covered.contains(&sym) {
            let path = sym.join(" > ");
            findings.push(Finding {
                severity: Severity::Warn,
                hub: hub.to_string(),
                claim: String::new(),
                at: format!("{file} > {path}"),
                message: format!(
                    "public symbol `{path}` in {file} has no claim in any hub — add an anchor or accept it as intentionally undocumented"
                ),
            });
        }
    }
}

fn truncate(s: &str, max: usize) -> String {
    let one_line = s.split_whitespace().collect::<Vec<_>>().join(" ");
    if one_line.chars().count() <= max {
        one_line
    } else {
        let kept: String = one_line.chars().take(max).collect();
        format!("{kept}…")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use surf_core::hash_anchor;

    fn ws_with(files: &[(&str, &str)]) -> (tempfile::TempDir, Workspace) {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();
        fs::write(root.join("surf.toml"), "").unwrap();
        fs::create_dir_all(root.join("hubs")).unwrap();
        for (rel, content) in files {
            let p = root.join(rel);
            if let Some(parent) = p.parent() {
                fs::create_dir_all(parent).unwrap();
            }
            fs::write(p, content).unwrap();
        }
        let ws = Workspace::discover(root).unwrap();
        (tmp, ws)
    }

    fn rust_hash(src: &str, anchor: &str) -> String {
        hash_anchor(src, Lang::Rust, &parse_anchor(anchor).unwrap()).unwrap()
    }

    #[test]
    fn clean_anchor_has_no_findings() {
        let (_t, ws) = ws_with(&[
            ("src/auth.rs", "pub fn greet() -> &'static str { \"hi\" }\n"),
            ("hubs/a.md", "---\nsummary: x\nanchors:\n  - claim: greeting exists\n    at: src/auth.rs > greet\n---\n"),
        ]);
        assert!(lint_workspace(&ws).unwrap().is_empty());
    }

    #[test]
    fn ambiguous_anchor_blocks() {
        let (_t, ws) = ws_with(&[
            (
                "src/dup.ts",
                "function dup(): void {}\nfunction dup(): void {}\n",
            ),
            (
                "hubs/a.md",
                "---\nsummary: x\nanchors:\n  - claim: dup\n    at: src/dup.ts > dup\n---\n",
            ),
        ]);
        let f = lint_workspace(&ws).unwrap();
        assert_eq!(f.len(), 1);
        assert_eq!(f[0].severity, Severity::Block);
        assert!(
            f[0].message.contains("@N"),
            "message should suggest @N: {}",
            f[0].message
        );
    }

    #[test]
    fn vanished_symbol_blocks() {
        let (_t, ws) = ws_with(&[
            ("src/auth.rs", "pub fn greet() {}\n"),
            (
                "hubs/a.md",
                "---\nsummary: x\nanchors:\n  - claim: ghost\n    at: src/auth.rs > ghost\n---\n",
            ),
        ]);
        let f = lint_workspace(&ws).unwrap();
        assert_eq!(f.len(), 1);
        assert_eq!(f[0].severity, Severity::Block);
    }

    #[test]
    fn findings_serialize_with_expected_keys() {
        let (_t, ws) = ws_with(&[
            ("src/auth.rs", "pub fn greet() {}\n"),
            (
                "hubs/a.md",
                "---\nsummary: x\nanchors:\n  - claim: ghost\n    at: src/auth.rs > ghost\n---\n",
            ),
        ]);
        let findings = lint_workspace(&ws).unwrap();
        let json = serde_json::to_value(&findings).unwrap();
        let obj = json[0].as_object().unwrap();
        for key in ["severity", "hub", "at", "message", "claim"] {
            assert!(obj.contains_key(key), "missing key `{key}` in {obj:?}");
        }
        assert_eq!(obj["severity"], "block");
    }

    #[test]
    fn renamed_symbol_warns_and_suggests_follow() {
        let new_src = "pub fn rotate_token(t: &str) -> String { t.to_string() }\n";
        let stored = rust_hash(new_src, "src/auth.rs > rotate_token");
        let hub = format!(
            "---\nsummary: x\nanchors:\n  - claim: rotation\n    at: src/auth.rs > rotate\n    hash: {stored}\n---\n"
        );
        let (_t, ws) = ws_with(&[("src/auth.rs", new_src), ("hubs/a.md", &hub)]);

        let f = lint_workspace(&ws).unwrap();
        assert_eq!(f.len(), 1);
        assert_eq!(f[0].severity, Severity::Warn);
        assert!(f[0].message.contains("rotate_token"));
        assert!(f[0].message.contains("--follow"));
    }

    #[test]
    fn under_coverage_warns_for_unanchored_export() {
        let (_t, ws) = ws_with(&[
            (
                "src/m.rs",
                "pub fn a() {}\npub fn b() {}\nfn private() {}\n",
            ),
            (
                "hubs/a.md",
                "---\nsummary: x\nanchors:\n  - claim: a does\n    at: src/m.rs > a\n---\n",
            ),
        ]);
        let f = lint_workspace(&ws).unwrap();
        // Only the exported-but-unanchored `b`; the private fn and the covered `a` are silent.
        assert_eq!(f.len(), 1);
        assert_eq!(f[0].severity, Severity::Warn);
        assert!(f[0].message.contains("`b`"), "{}", f[0].message);
    }

    #[test]
    fn coverage_is_workspace_wide_not_per_hub() {
        // One file, its public surface split across two hubs. Neither symbol is uncovered
        // workspace-wide, so neither hub may be nagged about the symbol the other one owns (#54).
        let (_t, ws) = ws_with(&[
            ("src/m.rs", "pub fn a() {}\npub fn b() {}\n"),
            (
                "hubs/a.md",
                "---\nsummary: x\nanchors:\n  - claim: a does\n    at: src/m.rs > a\n---\n",
            ),
            (
                "hubs/b.md",
                "---\nsummary: x\nanchors:\n  - claim: b does\n    at: src/m.rs > b\n---\n",
            ),
        ]);
        let f = lint_workspace(&ws).unwrap();
        assert!(f.is_empty(), "expected no findings, got {f:?}");
    }

    #[test]
    fn under_coverage_includes_methods() {
        // A method-heavy Go type: the top-level fn is anchored, but a public method is not.
        // Pre-#54 the nudge saw only top-level fns and stayed silent; now methods count.
        let go = "package m\n\ntype Builder struct{}\n\nfunc NewBuilder() *Builder { return &Builder{} }\n\nfunc (b *Builder) Set() {}\n";
        let (_t, ws) = ws_with(&[
            ("src/m.go", go),
            (
                "hubs/a.md",
                "---\nsummary: x\nanchors:\n  - claim: ctor\n    at: src/m.go > NewBuilder\n---\n",
            ),
        ]);
        let f = lint_workspace(&ws).unwrap();
        assert_eq!(f.len(), 1, "expected one method nudge, got {f:?}");
        assert_eq!(f[0].severity, Severity::Warn);
        assert!(f[0].message.contains("`Builder > Set`"), "{}", f[0].message);
    }

    #[test]
    fn anchoring_a_method_silences_only_that_method() {
        // Anchoring `Builder > Set` covers exactly it — a sibling method stays flagged (#29 parity).
        let go = "package m\n\ntype Builder struct{}\n\nfunc (b *Builder) Set() {}\n\nfunc (b *Builder) Del() {}\n";
        let (_t, ws) = ws_with(&[
            ("src/m.go", go),
            (
                "hubs/a.md",
                "---\nsummary: x\nanchors:\n  - claim: set\n    at: src/m.go > Builder > Set\n---\n",
            ),
        ]);
        let f = lint_workspace(&ws).unwrap();
        assert_eq!(f.len(), 1, "only the unanchored sibling, got {f:?}");
        assert!(f[0].message.contains("`Builder > Del`"), "{}", f[0].message);
    }

    #[test]
    fn broken_anchor_suppresses_under_coverage() {
        // `ghost` blocks, so the file is unhealthy and `b` is NOT additionally flagged.
        let (_t, ws) = ws_with(&[
            ("src/m.rs", "pub fn a() {}\npub fn b() {}\n"),
            (
                "hubs/a.md",
                "---\nsummary: x\nanchors:\n  - claim: c\n    at: src/m.rs > ghost\n---\n",
            ),
        ]);
        let f = lint_workspace(&ws).unwrap();
        assert_eq!(f.len(), 1);
        assert_eq!(f[0].severity, Severity::Block);
    }

    #[test]
    fn coarse_span_warns_on_whole_file_anchor() {
        let body: String = (0..40).map(|i| format!("    let x{i} = {i};\n")).collect();
        let src = format!("pub fn big() {{\n{body}}}\n");
        let (_t, ws) = ws_with(&[
            ("src/m.rs", src.as_str()),
            (
                "hubs/a.md",
                "---\nsummary: x\nanchors:\n  - claim: big does\n    at: src/m.rs > big\n---\n",
            ),
        ]);
        let f = lint_workspace(&ws).unwrap();
        assert_eq!(f.len(), 1);
        assert_eq!(f[0].severity, Severity::Warn);
        assert!(f[0].message.contains("whole-file"), "{}", f[0].message);
    }

    #[test]
    fn too_many_anchors_warns() {
        let mut src = String::new();
        let mut anchors = String::new();
        for i in 0..=MAX_ANCHORS_PER_HUB {
            src.push_str(&format!("pub fn f{i}() {{}}\n"));
            anchors.push_str(&format!("  - claim: c{i}\n    at: src/m.rs > f{i}\n"));
        }
        let hub = format!("---\nsummary: x\nanchors:\n{anchors}---\n");
        let (_t, ws) = ws_with(&[("src/m.rs", src.as_str()), ("hubs/a.md", hub.as_str())]);

        let f = lint_workspace(&ws).unwrap();
        assert!(
            f.iter()
                .any(|x| x.severity == Severity::Warn && x.message.contains("anchors in one hub")),
            "expected a too-many-anchors warning, got {f:?}"
        );
    }

    #[test]
    fn claim_log_warns_on_many_single_site_claims() {
        // Four claims, each anchoring a single symbol, no multi-site `at:` — the per-symbol
        // claim-log smell. A rich body keeps thin-prose quiet so only the granularity warning fires.
        let mut src = String::new();
        let mut anchors = String::new();
        for i in 0..CLAIM_LOG_MIN_CLAIMS {
            src.push_str(&format!("pub fn f{i}() {{}}\n"));
            anchors.push_str(&format!("  - claim: c{i}\n    at: src/m.rs > f{i}\n"));
        }
        let body: String = "prose ".repeat(200);
        let hub = format!("---\nsummary: x\nanchors:\n{anchors}---\n# H\n\n{body}\n");
        let (_t, ws) = ws_with(&[("src/m.rs", src.as_str()), ("hubs/a.md", hub.as_str())]);

        let f = lint_workspace(&ws).unwrap();
        let warn = f
            .iter()
            .find(|x| x.message.contains("claim-log"))
            .expect("expected a claim-log warning");
        assert_eq!(warn.severity, Severity::Warn);
    }

    #[test]
    fn claim_log_silent_when_a_claim_consolidates() {
        // Same claim count, but one claim uses a multi-site `at:` — the hub consolidates, so the
        // claim-log nudge stays quiet.
        let mut src = String::new();
        for i in 0..CLAIM_LOG_MIN_CLAIMS {
            src.push_str(&format!("pub fn f{i}() {{}}\n"));
        }
        let mut anchors = String::from(
            "  - claim: pair\n    at:\n      - src/m.rs > f0\n      - src/m.rs > f1\n",
        );
        for i in 2..CLAIM_LOG_MIN_CLAIMS {
            anchors.push_str(&format!("  - claim: c{i}\n    at: src/m.rs > f{i}\n"));
        }
        let body: String = "prose ".repeat(200);
        let hub = format!("---\nsummary: x\nanchors:\n{anchors}---\n# H\n\n{body}\n");
        let (_t, ws) = ws_with(&[("src/m.rs", src.as_str()), ("hubs/a.md", hub.as_str())]);

        let f = lint_workspace(&ws).unwrap();
        assert!(
            !f.iter().any(|x| x.message.contains("claim-log")),
            "consolidated hub should not warn: {f:?}"
        );
    }

    #[test]
    fn thin_prose_warns_on_stub_body() {
        // Two claims, near-empty body — an onboarding doc with no onboarding prose.
        let (_t, ws) = ws_with(&[
            ("src/m.rs", "pub fn a() {}\npub fn b() {}\n"),
            (
                "hubs/a.md",
                "---\nsummary: x\nanchors:\n  - claim: a does a\n    at: src/m.rs > a\n  - claim: b does b\n    at: src/m.rs > b\n---\n# H\n",
            ),
        ]);
        let f = lint_workspace(&ws).unwrap();
        let warn = f
            .iter()
            .find(|x| x.message.contains("thin prose"))
            .expect("expected a thin-prose warning");
        assert_eq!(warn.severity, Severity::Warn);
    }

    #[test]
    fn thin_prose_silent_with_real_body() {
        // Two claims plus a real body — the onboarding prose a hub should carry; no warning. A
        // fenced code block doesn't count toward prose, so the words are genuine.
        let body: String = "word ".repeat(40);
        let hub = format!(
            "---\nsummary: x\nanchors:\n  - claim: a\n    at: src/m.rs > a\n  - claim: b\n    at: src/m.rs > b\n---\n# H\n\n{body}\n"
        );
        let (_t, ws) = ws_with(&[
            ("src/m.rs", "pub fn a() {}\npub fn b() {}\n"),
            ("hubs/a.md", hub.as_str()),
        ]);
        let f = lint_workspace(&ws).unwrap();
        assert!(
            !f.iter().any(|x| x.message.contains("thin prose")),
            "a hub with a real body should not warn: {f:?}"
        );
    }

    #[test]
    fn covers_valid_globs_are_silent() {
        let (_t, ws) = ws_with(&[
            ("src/auth.rs", "pub fn greet() {}\n"),
            (
                "hubs/a.md",
                "---\nsummary: x\ncovers:\n  - src/**\nanchors:\n  - claim: greeting\n    at: src/auth.rs > greet\n---\n",
            ),
        ]);
        assert!(lint_workspace(&ws).unwrap().is_empty());
    }

    #[test]
    fn covers_malformed_glob_blocks() {
        let (_t, ws) = ws_with(&[
            ("src/auth.rs", "pub fn greet() {}\n"),
            (
                "hubs/a.md",
                "---\nsummary: x\ncovers:\n  - 'src/[unterminated'\nanchors:\n  - claim: greeting\n    at: src/auth.rs > greet\n---\n",
            ),
        ]);
        let f = lint_workspace(&ws).unwrap();
        let block = f
            .iter()
            .find(|x| x.message.contains("invalid `covers` glob"))
            .expect("expected a covers glob error");
        assert_eq!(block.severity, Severity::Block);
    }

    #[test]
    fn covers_not_matching_own_anchor_warns() {
        // `covers` scopes only `lib/**`, but the hub anchors a file under `src/` — the fat-finger
        // the nudge exists to catch.
        let (_t, ws) = ws_with(&[
            ("src/auth.rs", "pub fn greet() {}\n"),
            (
                "hubs/a.md",
                "---\nsummary: x\ncovers:\n  - lib/**\nanchors:\n  - claim: greeting\n    at: src/auth.rs > greet\n---\n",
            ),
        ]);
        let f = lint_workspace(&ws).unwrap();
        let warn = f
            .iter()
            .find(|x| x.message.contains("not matched by any `covers` glob"))
            .expect("expected an unmatched-anchor warning");
        assert_eq!(warn.severity, Severity::Warn);
        assert_eq!(warn.at, "src/auth.rs");
    }

    #[test]
    fn refs_to_existing_hub_is_silent() {
        let (_t, ws) = ws_with(&[
            ("src/auth.rs", "pub fn greet() {}\n"),
            (
                "hubs/a.md",
                "---\nsummary: x\nrefs:\n  - ./b.md\nanchors:\n  - claim: g\n    at: src/auth.rs > greet\n---\n",
            ),
            ("hubs/b.md", "---\nsummary: y\n---\n# B\n"),
        ]);
        assert!(lint_workspace(&ws).unwrap().is_empty());
    }

    #[test]
    fn refs_to_missing_hub_blocks() {
        let (_t, ws) = ws_with(&[("hubs/a.md", "---\nsummary: x\nrefs:\n  - ./gone.md\n---\n")]);
        let f = lint_workspace(&ws).unwrap();
        let block = f
            .iter()
            .find(|x| x.message.contains("does not resolve to a hub"))
            .expect("expected a dangling-ref error");
        assert_eq!(block.severity, Severity::Block);
        assert_eq!(block.at, "./gone.md");
    }

    #[test]
    fn refs_to_non_hub_file_blocks() {
        // A doc path is not a hub — the reclassification trigger for the two ../docs refs (#4).
        let (_t, ws) = ws_with(&[(
            "hubs/a.md",
            "---\nsummary: x\nrefs:\n  - ../docs/guide.md\n---\n",
        )]);
        let f = lint_workspace(&ws).unwrap();
        assert!(f
            .iter()
            .any(|x| x.severity == Severity::Block && x.message.contains("does not resolve")));
    }

    #[test]
    fn self_ref_blocks() {
        let (_t, ws) = ws_with(&[("hubs/a.md", "---\nsummary: x\nrefs:\n  - ./a.md\n---\n")]);
        let f = lint_workspace(&ws).unwrap();
        let block = f
            .iter()
            .find(|x| x.message.contains("its own hub"))
            .expect("expected a self-ref error");
        assert_eq!(block.severity, Severity::Block);
    }

    #[test]
    fn malformed_ref_blocks() {
        let (_t, ws) = ws_with(&[(
            "hubs/a.md",
            "---\nsummary: x\nrefs:\n  - '> dangling'\n---\n",
        )]);
        let f = lint_workspace(&ws).unwrap();
        assert!(f
            .iter()
            .any(|x| x.severity == Severity::Block && x.message.contains("invalid `refs` entry")));
    }

    #[test]
    fn claim_ref_matches_anchor_suffix() {
        // `./b.md > greet` resolves: b.md has a claim anchored at `src/auth.rs > greet`.
        let (_t, ws) = ws_with(&[
            ("src/auth.rs", "pub fn greet() {}\n"),
            (
                "hubs/a.md",
                "---\nsummary: x\nrefs:\n  - ./b.md > greet\n---\n",
            ),
            (
                "hubs/b.md",
                "---\nsummary: y\nanchors:\n  - claim: g\n    at: src/auth.rs > greet\n---\n",
            ),
        ]);
        assert!(lint_workspace(&ws).unwrap().is_empty());
    }

    #[test]
    fn claim_ref_with_no_matching_anchor_blocks() {
        let (_t, ws) = ws_with(&[
            ("src/auth.rs", "pub fn greet() {}\n"),
            (
                "hubs/a.md",
                "---\nsummary: x\nrefs:\n  - ./b.md > nonexistent\n---\n",
            ),
            (
                "hubs/b.md",
                "---\nsummary: y\nanchors:\n  - claim: g\n    at: src/auth.rs > greet\n---\n",
            ),
        ]);
        let f = lint_workspace(&ws).unwrap();
        let block = f
            .iter()
            .find(|x| x.message.contains("no anchor in"))
            .expect("expected a no-matching-claim error");
        assert_eq!(block.severity, Severity::Block);
    }

    #[test]
    fn typo_frontmatter_key_warns_not_blocks() {
        // `anchros` is one edit from `anchors` → a warning that recovers the fail-closed signal
        // relaxing deny_unknown_fields gave up. Never a block (OKF preserves unknown keys).
        let (_t, ws) = ws_with(&[(
            "hubs/a.md",
            "---\nsummary: x\nanchros:\n  - claim: c\n    at: src/m.rs > add\n---\n",
        )]);
        let f = lint_workspace(&ws).unwrap();
        let warn = f
            .iter()
            .find(|x| x.message.contains("did you mean `anchors`"))
            .expect("expected a typo warning");
        assert_eq!(warn.severity, Severity::Warn);
        assert!(!f.iter().any(|x| x.severity == Severity::Block));
    }

    #[test]
    fn unrelated_okf_key_is_not_flagged_as_typo() {
        // A legitimate OKF/doc-system key (well clear of any known key) must not warn.
        let (_t, ws) = ws_with(&[
            ("src/m.rs", "pub fn a() {}\n"),
            (
                "hubs/a.md",
                "---\ntype: Runbook\ndescription: how to deploy\nauthor: rachel\nanchors:\n  - claim: a\n    at: src/m.rs > a\n---\n",
            ),
        ]);
        let f = lint_workspace(&ws).unwrap();
        assert!(
            !f.iter().any(|x| x.message.contains("did you mean")),
            "no typo warning expected: {f:?}"
        );
    }

    #[test]
    fn anchored_hub_without_headline_warns() {
        // An anchored hub with no summary/title/description reads as a claim dump, not an
        // onboarding doc.
        let (_t, ws) = ws_with(&[
            ("src/m.rs", "pub fn a() {}\n"),
            (
                "hubs/a.md",
                "---\ntype: concept\nanchors:\n  - claim: a does a\n    at: src/m.rs > a\n---\n",
            ),
        ]);
        let f = lint_workspace(&ws).unwrap();
        let warn = f
            .iter()
            .find(|x| x.message.contains("no headline"))
            .expect("expected a headline warning");
        assert_eq!(warn.severity, Severity::Warn);
    }

    #[test]
    fn reserved_index_file_is_not_linted() {
        // A plain OKF index.md (no frontmatter) must not produce a block from lint.
        let (_t, ws) = ws_with(&[("hubs/index.md", "# Sales\n\n- [orders](./orders.md)\n")]);
        let f = lint_workspace(&ws).unwrap();
        assert!(f.is_empty(), "reserved file should not be linted: {f:?}");
    }

    #[test]
    fn edit_distance_1_matches_only_close_keys() {
        assert!(within_edit_distance_1("anchros", "anchors")); // adjacent transposition
        assert!(within_edit_distance_1("anchor", "anchors")); // one deletion
        assert!(within_edit_distance_1("tag", "tags")); // one insertion
        assert!(within_edit_distance_1("titel", "title")); // el↔le transposition
        assert!(!within_edit_distance_1("resource", "anchors")); // unrelated
        assert!(!within_edit_distance_1("anchors", "anchors")); // identical → no warning
        assert!(!within_edit_distance_1("tg", "tags")); // two edits away
    }

    #[test]
    fn okf_dangling_cross_link_warns_never_blocks() {
        // A body link to a non-existent concept warns (advisory) but never blocks — OKF tolerates
        // broken links.
        let (_t, ws) = ws_with(&[(
            "hubs/orders.md",
            "---\ntype: BigQuery Table\ndescription: orders\n---\n# Orders\n\nJoined with [customers](./customers.md).\n",
        )]);
        let f = lint_workspace(&ws).unwrap();
        let warn = f
            .iter()
            .find(|x| x.message.contains("OKF cross-link"))
            .expect("expected a dangling-link warning");
        assert_eq!(warn.severity, Severity::Warn);
        assert!(!f.iter().any(|x| x.severity == Severity::Block));
    }

    #[test]
    fn okf_resolvable_cross_link_is_silent() {
        // The link target exists → no warning. URLs and anchors are ignored too.
        let (_t, ws) = ws_with(&[
            (
                "hubs/orders.md",
                "---\ntype: BigQuery Table\ndescription: orders\n---\n# Orders\n\nSee [customers](./customers.md), the [docs](https://x.io/a.md), and [top](#orders).\n",
            ),
            (
                "hubs/customers.md",
                "---\ntype: BigQuery Table\ndescription: customers\n---\n# Customers\n",
            ),
        ]);
        let f = lint_workspace(&ws).unwrap();
        assert!(
            !f.iter().any(|x| x.message.contains("OKF cross-link")),
            "no dangling-link warning expected: {f:?}"
        );
    }

    fn agents_findings(ws: &Workspace) -> Vec<Finding> {
        lint_workspace(ws)
            .unwrap()
            .into_iter()
            .filter(|f| f.hub == "AGENTS.md")
            .collect()
    }

    #[test]
    fn agents_pointer_valid_is_silent() {
        // ws_with creates the `hubs/` dir; the block links it.
        let (_t, ws) = ws_with(&[(
            "AGENTS.md",
            "# Agents\n<!-- surf:hubs -->\nContext lives in [`hubs/`](./hubs/) — search it.\n<!-- /surf:hubs -->\n",
        )]);
        assert!(agents_findings(&ws).is_empty());
    }

    #[test]
    fn agents_no_markers_is_silent() {
        // A link to hubs but no markers → opt-out, no enforcement.
        let (_t, ws) = ws_with(&[("AGENTS.md", "# Agents\nsee [hubs](./hubs/)\n")]);
        assert!(agents_findings(&ws).is_empty());
    }

    #[test]
    fn agents_no_file_is_silent() {
        let (_t, ws) = ws_with(&[("src/m.rs", "pub fn a() {}\n")]);
        assert!(agents_findings(&ws).is_empty());
    }

    #[test]
    fn agents_pointer_to_wrong_dir_blocks() {
        let (_t, ws) = ws_with(&[(
            "AGENTS.md",
            "<!-- surf:hubs -->\nsee [stuff](./nothubs/)\n<!-- /surf:hubs -->\n",
        )]);
        let f = agents_findings(&ws);
        assert_eq!(f.len(), 1);
        assert_eq!(f[0].severity, Severity::Block);
    }
}
