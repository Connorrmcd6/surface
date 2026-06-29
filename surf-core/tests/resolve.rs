use surf_core::{parse_anchor, resolve, Lang, ResolveError, Span};

const TS: &str = include_str!("fixtures/auth.ts");
const RS: &str = include_str!("fixtures/auth.rs");
const PY: &str = include_str!("fixtures/auth.py");
const GO: &str = include_str!("fixtures/auth.go");
const JS: &str = include_str!("fixtures/auth.js");

fn span(src: &str, lang: Lang, anchor: &str) -> Span {
    let a = parse_anchor(anchor).unwrap();
    resolve(src, lang, &a).unwrap_or_else(|e| panic!("resolve `{anchor}` failed: {e}"))
}

fn snippet(src: &str, s: Span) -> &str {
    &src[s.start_byte..s.end_byte]
}

fn err(src: &str, lang: Lang, anchor: &str) -> ResolveError {
    let a = parse_anchor(anchor).unwrap();
    resolve(src, lang, &a).expect_err("expected resolution to fail")
}

// --- TypeScript -----------------------------------------------------------

#[test]
fn ts_method_disambiguated_by_class() {
    // Both classes declare `rotate`; the qualified path picks the right one.
    let a = span(TS, Lang::TypeScript, "auth.ts > TokenService > rotate");
    assert!(snippet(TS, a).contains("token + \"!\""));

    let b = span(TS, Lang::TypeScript, "auth.ts > OtherService > rotate");
    assert!(snippet(TS, b).contains("token + \"?\""));

    assert_ne!((a.start_byte, a.end_byte), (b.start_byte, b.end_byte));
}

#[test]
fn ts_overloads_are_ambiguous_then_positional() {
    // Two signatures + one implementation, all named `rotate`, at top level.
    match err(TS, Lang::TypeScript, "auth.ts > rotate") {
        ResolveError::Ambiguous { count, .. } => assert_eq!(count, 3),
        other => panic!("expected Ambiguous, got {other:?}"),
    }
    // @3 selects the implementation (the one with a body).
    let impl_fn = span(TS, Lang::TypeScript, "auth.ts > rotate @3");
    assert!(snippet(TS, impl_fn).contains("force ? token.toUpperCase()"));
}

#[test]
fn ts_nested_function_in_arrow_const() {
    let s = span(TS, Lang::TypeScript, "auth.ts > refresh > inner");
    assert!(snippet(TS, s).contains("t.trim()"));
}

#[test]
fn ts_const_bound_wrapped_function_resolves() {
    // `export const getResults = cache(unstable_cache(async () => ...))` — a const whose
    // initializer is a call expression must resolve like a function declaration.
    let s = span(TS, Lang::TypeScript, "auth.ts > getResults");
    assert!(snippet(TS, s).contains("id.trim()"));
}

#[test]
fn ts_const_bound_call_initializer_resolves() {
    // The same path covers `export const X = z.object(...)` (Zod schemas, server actions).
    let s = span(TS, Lang::TypeScript, "auth.ts > loginSchema");
    assert!(snippet(TS, s).contains("z.object"));
}

#[test]
fn ts_not_found_is_distinct() {
    match err(TS, Lang::TypeScript, "auth.ts > doesNotExist") {
        ResolveError::NotFound { segment } => assert_eq!(segment, "doesNotExist"),
        other => panic!("expected NotFound, got {other:?}"),
    }
}

// --- Rust -----------------------------------------------------------------

#[test]
fn rust_method_via_impl_scope() {
    // `TokenService` alone matches both the struct and its impl, but the path
    // `TokenService > rotate` resolves uniquely to the method.
    let s = span(RS, Lang::Rust, "auth.rs > TokenService > rotate");
    assert!(snippet(RS, s).contains("helper(token)"));
}

#[test]
fn rust_top_level_fn_is_scoped() {
    // A free `rotate`, a method `rotate`, and a module fn `rotate` all exist;
    // the top-level path resolves to exactly the free function.
    let s = span(RS, Lang::Rust, "auth.rs > rotate");
    assert!(snippet(RS, s).starts_with("pub fn rotate(token: &str)"));
}

#[test]
fn rust_module_function() {
    let s = span(RS, Lang::Rust, "auth.rs > refresh > rotate");
    assert!(snippet(RS, s).contains("token.to_string()"));
    // Distinct from the free function and the method of the same name.
    let free = span(RS, Lang::Rust, "auth.rs > rotate");
    assert_ne!((s.start_byte, s.end_byte), (free.start_byte, free.end_byte));
}

#[test]
fn rust_deeply_nested_function() {
    let s = span(RS, Lang::Rust, "auth.rs > refresh > nested > deep");
    assert!(snippet(RS, s).contains("7"));
}

#[test]
fn rust_type_alone_is_ambiguous() {
    // struct + impl share the name.
    match err(RS, Lang::Rust, "auth.rs > TokenService") {
        ResolveError::Ambiguous { count, .. } => assert_eq!(count, 2),
        other => panic!("expected Ambiguous, got {other:?}"),
    }
}

// --- Python ---------------------------------------------------------------

#[test]
fn python_method_disambiguated_by_class() {
    let a = span(PY, Lang::Python, "auth.py > TokenService > rotate");
    assert!(snippet(PY, a).contains("token + \"!\""));
    let b = span(PY, Lang::Python, "auth.py > OtherService > rotate");
    assert!(snippet(PY, b).contains("token + \"?\""));
    assert_ne!((a.start_byte, a.end_byte), (b.start_byte, b.end_byte));
}

#[test]
fn python_top_level_ambiguous_then_positional() {
    match err(PY, Lang::Python, "auth.py > rotate") {
        ResolveError::Ambiguous { count, .. } => assert_eq!(count, 2),
        other => panic!("expected Ambiguous, got {other:?}"),
    }
    let second = span(PY, Lang::Python, "auth.py > rotate @2");
    assert!(snippet(PY, second).contains("force"));
}

#[test]
fn python_nested_function() {
    let s = span(PY, Lang::Python, "auth.py > refresh > inner");
    assert!(snippet(PY, s).contains("t.strip()"));
}

#[test]
fn python_resolves_through_decorator() {
    let s = span(PY, Lang::Python, "auth.py > cached");
    assert!(snippet(PY, s).contains("return 1"));
}

#[test]
fn python_not_found_is_distinct() {
    assert!(matches!(
        err(PY, Lang::Python, "auth.py > nope"),
        ResolveError::NotFound { .. }
    ));
}

#[test]
fn python_module_constant_resolves() {
    let s = span(PY, Lang::Python, "auth.py > RETRYABLE_STATUS_CODES");
    assert!(snippet(PY, s).contains("frozenset({429, 500, 502, 503, 504})"));
}

#[test]
fn python_type_alias_resolves() {
    let s = span(PY, Lang::Python, "auth.py > Chain");
    assert!(snippet(PY, s).contains("Literal[\"arbitrum\""));
}

#[test]
fn python_class_attribute_resolves() {
    // Annotation-only attribute (no value) and an annotated assignment, both class-level.
    let a = span(PY, Lang::Python, "auth.py > RateLimitError > retry_after");
    assert!(snippet(PY, a).contains("retry_after: float | None"));
    let b = span(PY, Lang::Python, "auth.py > RateLimitError > code");
    assert!(snippet(PY, b).contains("code: int = 429"));
}

// --- Go ---------------------------------------------------------------------

#[test]
fn go_method_resolved_by_receiver() {
    // Both types declare Rotate; the receiver disambiguates.
    let a = span(GO, Lang::Go, "auth.go > TokenService > Rotate");
    assert!(snippet(GO, a).contains("token + \"!\""));
    let b = span(GO, Lang::Go, "auth.go > OtherService > Rotate");
    assert!(snippet(GO, b).contains("token + \"?\""));
    assert_ne!((a.start_byte, a.end_byte), (b.start_byte, b.end_byte));
}

#[test]
fn go_top_level_func_excludes_methods() {
    // A free `Rotate` and two methods named Rotate exist; the single-segment path
    // resolves only the free function.
    let s = span(GO, Lang::Go, "auth.go > Rotate");
    assert!(snippet(GO, s).starts_with("func Rotate(token string)"));
}

#[test]
fn go_type_resolves_uniquely() {
    let s = span(GO, Lang::Go, "auth.go > TokenService");
    assert!(snippet(GO, s).contains("secret string"));
}

#[test]
fn go_missing_method_is_not_found() {
    assert!(matches!(
        err(GO, Lang::Go, "auth.go > TokenService > Missing"),
        ResolveError::NotFound { .. }
    ));
}

// --- JavaScript (reuses the TS family via the TSX grammar) ------------------

#[test]
fn js_function_method_and_nested_arrow() {
    assert!(snippet(JS, span(JS, Lang::JavaScript, "auth.js > add")).contains("a + b"));
    assert!(
        snippet(JS, span(JS, Lang::JavaScript, "auth.js > Service > rotate"))
            .contains("token + \"!\"")
    );
    assert!(snippet(JS, span(JS, Lang::JavaScript, "auth.js > make > inner")).contains("return x"));
}

#[test]
fn js_jsx_parses_and_resolves() {
    // Proves the TSX grammar handles JSX inside a .js file.
    let s = span(JS, Lang::JavaScript, "auth.js > Badge");
    assert!(snippet(JS, s).contains("className"));
}

#[test]
fn span_lines_are_one_based() {
    let s = span(RS, Lang::Rust, "auth.rs > TokenService > validate");
    assert!(s.start_line >= 1 && s.end_line >= s.start_line);
}

// --- Unicode identifiers (#45) ----------------------------------------------

#[test]
fn unicode_identifier_resolves_across_families() {
    // Non-ASCII symbol names must resolve to the Unicode-named body, with no locale/encoding
    // sensitivity. Pairs with the pinned hashes in golden_hash.rs.
    let rs = "pub fn café(δ: i64) -> i64 {\n    δ\n}\n";
    assert!(snippet(rs, span(rs, Lang::Rust, "x.rs > café")).contains("δ"));

    let ts = "export function café(δ: string): string {\n  return δ;\n}\n";
    assert!(snippet(ts, span(ts, Lang::TypeScript, "x.ts > café")).contains("return δ"));

    let py = "def café(δ):\n    return δ\n";
    assert!(snippet(py, span(py, Lang::Python, "x.py > café")).contains("return δ"));

    let go = "func Café(δ int) int {\n\treturn δ\n}\n";
    assert!(snippet(go, span(go, Lang::Go, "x.go > Café")).contains("return δ"));
}

// --- Python module-level if/try blocks (#81) --------------------------------

const GUARDED_PY: &str = r#"
import sys
from typing import TYPE_CHECKING

if TYPE_CHECKING:
    from collections import OrderedDict
    GuardedAlias = OrderedDict

if sys.version_info >= (3, 10):
    def gated() -> bool:
        return True
elif sys.version_info >= (3, 9):
    def gated() -> bool:
        return False
else:
    LEGACY = 1

try:
    from fast_json import loads

    def parse(data):
        return loads(data)
except ImportError:
    def parse(data):
        return None
finally:
    CLEANUP = 1
"#;

#[test]
fn python_type_checking_guarded_alias_resolves() {
    let s = span(GUARDED_PY, Lang::Python, "auth.py > GuardedAlias");
    assert!(snippet(GUARDED_PY, s).contains("OrderedDict"));
}

#[test]
fn python_version_gated_def_is_ambiguous_then_positional() {
    // The same name bound in the if and elif branches: two matches, @N picks one.
    match err(GUARDED_PY, Lang::Python, "auth.py > gated") {
        ResolveError::Ambiguous { count, .. } => assert_eq!(count, 2),
        other => panic!("expected Ambiguous, got {other:?}"),
    }
    let first = span(GUARDED_PY, Lang::Python, "auth.py > gated @1");
    assert!(snippet(GUARDED_PY, first).contains("return True"));
    let second = span(GUARDED_PY, Lang::Python, "auth.py > gated @2");
    assert!(snippet(GUARDED_PY, second).contains("return False"));
}

#[test]
fn python_else_and_finally_bindings_resolve() {
    let legacy = span(GUARDED_PY, Lang::Python, "auth.py > LEGACY");
    assert!(snippet(GUARDED_PY, legacy).contains("LEGACY = 1"));
    let cleanup = span(GUARDED_PY, Lang::Python, "auth.py > CLEANUP");
    assert!(snippet(GUARDED_PY, cleanup).contains("CLEANUP = 1"));
}

#[test]
fn python_import_fallback_branches_resolve_positionally() {
    // try/except ImportError fallback: both `parse` defs are reachable; @N disambiguates.
    match err(GUARDED_PY, Lang::Python, "auth.py > parse") {
        ResolveError::Ambiguous { count, .. } => assert_eq!(count, 2),
        other => panic!("expected Ambiguous, got {other:?}"),
    }
    let fast = span(GUARDED_PY, Lang::Python, "auth.py > parse @1");
    assert!(snippet(GUARDED_PY, fast).contains("loads(data)"));
    let fallback = span(GUARDED_PY, Lang::Python, "auth.py > parse @2");
    assert!(snippet(GUARDED_PY, fallback).contains("return None"));
}

// --- Python @overload groups are one anchorable unit (#82) ------------------

const OVERLOADED_PY: &str = r#"
from typing import overload
import typing

@overload
def probe(x: int) -> int: ...
@overload
def probe(x: str) -> str: ...
def probe(x):
    return x

class Client:
    @typing.overload
    def fetch(self, key: int) -> int: ...
    @typing.overload
    def fetch(self, key: str) -> str: ...
    def fetch(self, key):
        return key

def plain(): ...
def plain(): ...
"#;

#[test]
fn python_overload_group_resolves_without_index() {
    // Stubs + impl are one logical symbol: no Ambiguous, and the span covers the whole
    // group so any stub signature is inside the gated bytes.
    let s = span(OVERLOADED_PY, Lang::Python, "f.py > probe");
    let text = snippet(OVERLOADED_PY, s);
    assert!(
        text.contains("def probe(x: int) -> int"),
        "span starts at the first stub"
    );
    assert!(text.contains("return x"), "span reaches the implementation");
}

#[test]
fn python_typing_dotted_overload_groups_methods_too() {
    let s = span(OVERLOADED_PY, Lang::Python, "f.py > Client > fetch");
    let text = snippet(OVERLOADED_PY, s);
    assert!(text.contains("key: int"));
    assert!(text.contains("return key"));
}

#[test]
fn python_non_overload_duplicates_stay_ambiguous() {
    // Grouping is driven by the @overload decorator, not by name collision alone.
    match err(OVERLOADED_PY, Lang::Python, "f.py > plain") {
        ResolveError::Ambiguous { count, .. } => assert_eq!(count, 2),
        other => panic!("expected Ambiguous, got {other:?}"),
    }
}

#[test]
fn python_nested_symbol_resolves_through_overload_group() {
    let src = r#"
from typing import overload

@overload
def outer(x: int) -> int: ...
def outer(x):
    def inner(y):
        return y * 2
    return inner(x)
"#;
    let s = span(src, Lang::Python, "f.py > outer > inner");
    assert!(snippet(src, s).contains("y * 2"));
}
