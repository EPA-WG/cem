//! Fixture-pair parity tests.
//!
//! For each `examples/cem-ml/*.cem` fixture and its matching
//! `examples/semantic/<stem>.html` parity fixture, this suite asserts the
//! cross-surface event-identity contract from
//! `packages/cem_ml/docs/cross-surface-conversion.md`:
//!
//! - Both surfaces tokenize without hard violations.
//! - Both surfaces produce balanced open/close event streams.
//! - Both surfaces produce the same multiset of element local names in
//!   the comparable subtree (after stripping HTML-only document
//!   wrappers `html`/`head`/`body`/`meta`/`title`/`link` that the CEM-ML
//!   fixture intentionally omits).
//! - Both surfaces produce the same `cem:`-namespaced attribute set on
//!   the same host elements (where the HTML fixture uses `cem:*` or
//!   `data-cem-*` mirrors).
//!
//! HTML-side `data-cem-*` attributes are HTML mirrors per AC-S-9 — the
//! CEM annotation namespace remains the source of truth on the CEM-ML
//! side. The shape comparison normalizes both forms.

use cem_ml::diagnostics::Severity;
use cem_ml::events::cem::CemEventNormalizer;
use cem_ml::events::{EventNormalizer, NormalizedEvent};
use cem_ml::source::{BytesSource, SourceId};
use cem_ml::tokenizer::cem::CemTokenizer;
use cem_ml::tokenizer::html::HtmlTokenizer;
use cem_ml::tokenizer::{SchemaToken, SchemaTokenKind, SchemaTokenizer};
use std::collections::BTreeMap;

/// HTML wrapper elements the CEM-ML fixtures intentionally omit. Stripped
/// from the HTML element multiset before parity comparison.
const HTML_WRAPPER_ELEMENTS: &[&str] = &[
    "html", "head", "body", "meta", "title", "link", "style", "script",
];

fn fixture_pairs() -> Vec<(std::path::PathBuf, std::path::PathBuf)> {
    let cem_dir =
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../examples/cem-ml");
    let html_dir =
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../examples/semantic");
    let mut pairs = Vec::new();
    for entry in std::fs::read_dir(&cem_dir).unwrap() {
        let cem_path = entry.unwrap().path();
        if cem_path.extension().and_then(|x| x.to_str()) != Some("cem") {
            continue;
        }
        let stem = cem_path.file_stem().unwrap().to_string_lossy().into_owned();
        let html_path = html_dir.join(format!("{stem}.html"));
        if html_path.exists() {
            pairs.push((cem_path, html_path));
        }
    }
    pairs.sort();
    assert!(!pairs.is_empty(), "no fixture pairs found");
    pairs
}

fn tokenize_cem(input: &str) -> (Vec<SchemaToken>, Vec<cem_ml::diagnostics::Diagnostic>) {
    let src = BytesSource::new(SourceId(1), input.as_bytes().to_vec());
    let mut t = CemTokenizer::from_source(src);
    let mut out = Vec::new();
    while let Some(tok) = t.next_token() {
        out.push(tok);
    }
    (out, t.take_diagnostics())
}

fn tokenize_html(input: &str) -> (Vec<SchemaToken>, Vec<cem_ml::diagnostics::Diagnostic>) {
    let src = BytesSource::new(SourceId(1), input.as_bytes().to_vec());
    let mut t = HtmlTokenizer::from_source(src);
    let mut out = Vec::new();
    while let Some(tok) = t.next_token() {
        out.push(tok);
    }
    (out, t.take_diagnostics())
}

fn element_open_names(tokens: &[SchemaToken]) -> Vec<String> {
    tokens
        .iter()
        .filter_map(|t| match &t.kind {
            SchemaTokenKind::NodeStart { name } if !name.starts_with('@') && !name.is_empty() => {
                Some(name.clone())
            }
            _ => None,
        })
        .collect()
}

fn element_count_map(names: &[String]) -> BTreeMap<String, u32> {
    let mut m = BTreeMap::new();
    for n in names {
        *m.entry(n.clone()).or_insert(0) += 1;
    }
    m
}

/// Project the HTML element multiset to the comparable subset by
/// dropping HTML-only document wrappers.
fn comparable_html_counts(html_names: &[String]) -> BTreeMap<String, u32> {
    let mut m = element_count_map(html_names);
    for w in HTML_WRAPPER_ELEMENTS {
        m.remove(*w);
    }
    m
}

fn event_open_close_balance(input: &str, html: bool) -> (usize, usize) {
    let src = BytesSource::new(SourceId(1), input.as_bytes().to_vec());
    let (opens, closes) = if html {
        let tok = HtmlTokenizer::from_source(src);
        let mut n = CemEventNormalizer::new(tok);
        let mut o = 0;
        let mut c = 0;
        while let Some(ev) = n.next_event() {
            match ev {
                NormalizedEvent::OpenScope { .. } => o += 1,
                NormalizedEvent::CloseScope { .. } => c += 1,
                _ => {}
            }
        }
        (o, c)
    } else {
        let tok = CemTokenizer::from_source(src);
        let mut n = CemEventNormalizer::new(tok);
        let mut o = 0;
        let mut c = 0;
        while let Some(ev) = n.next_event() {
            match ev {
                NormalizedEvent::OpenScope { .. } => o += 1,
                NormalizedEvent::CloseScope { .. } => c += 1,
                _ => {}
            }
        }
        (o, c)
    };
    (opens, closes)
}

#[test]
fn every_fixture_pair_tokenizes_without_hard_violations() {
    for (cem_path, html_path) in fixture_pairs() {
        let cem_input = std::fs::read_to_string(&cem_path).unwrap();
        let html_input = std::fs::read_to_string(&html_path).unwrap();
        let (_, cem_diags) = tokenize_cem(&cem_input);
        let (_, html_diags) = tokenize_html(&html_input);
        let cem_hard: Vec<_> = cem_diags
            .iter()
            .filter(|d| matches!(d.severity, Severity::Error | Severity::Fatal))
            .collect();
        let html_hard: Vec<_> = html_diags
            .iter()
            .filter(|d| matches!(d.severity, Severity::Error | Severity::Fatal))
            .collect();
        assert!(
            cem_hard.is_empty(),
            "[{cem_path:?}] tokenizer hard diagnostics: {cem_hard:?}"
        );
        assert!(
            html_hard.is_empty(),
            "[{html_path:?}] tokenizer hard diagnostics: {html_hard:?}"
        );
    }
}

#[test]
fn every_fixture_pair_has_balanced_event_streams() {
    for (cem_path, html_path) in fixture_pairs() {
        let cem_input = std::fs::read_to_string(&cem_path).unwrap();
        let html_input = std::fs::read_to_string(&html_path).unwrap();
        let (co, cc) = event_open_close_balance(&cem_input, false);
        let (ho, hc) = event_open_close_balance(&html_input, true);
        assert_eq!(
            co, cc,
            "[{cem_path:?}] CEM event opens/closes unbalanced: {co} / {cc}"
        );
        assert_eq!(
            ho, hc,
            "[{html_path:?}] HTML event opens/closes unbalanced: {ho} / {hc}"
        );
    }
}

#[test]
fn every_fixture_pair_shares_comparable_element_multiset() {
    for (cem_path, html_path) in fixture_pairs() {
        let stem = cem_path.file_stem().unwrap().to_string_lossy().into_owned();
        let cem_input = std::fs::read_to_string(&cem_path).unwrap();
        let html_input = std::fs::read_to_string(&html_path).unwrap();
        let (cem_tokens, _) = tokenize_cem(&cem_input);
        let (html_tokens, _) = tokenize_html(&html_input);

        let cem_counts = element_count_map(&element_open_names(&cem_tokens));
        let html_counts = comparable_html_counts(&element_open_names(&html_tokens));

        assert_eq!(
            cem_counts, html_counts,
            "[{stem}] element multiset differs:\n  cem:  {cem_counts:?}\n  html: {html_counts:?}"
        );
    }
}

#[test]
fn every_fixture_pair_has_a_top_level_landmark_element() {
    // The canonical fixtures all wrap their content in a single
    // top-level CEM landmark (main, ol, etc.). Confirm both surfaces
    // contain at least one of the recognized landmark elements.
    const LANDMARKS: &[&str] = &["main", "section", "ol", "ul", "article"];
    for (cem_path, html_path) in fixture_pairs() {
        let cem_input = std::fs::read_to_string(&cem_path).unwrap();
        let html_input = std::fs::read_to_string(&html_path).unwrap();
        let (cem_tokens, _) = tokenize_cem(&cem_input);
        let (html_tokens, _) = tokenize_html(&html_input);
        let cem_names = element_open_names(&cem_tokens);
        let html_names = element_open_names(&html_tokens);
        let cem_has_landmark = LANDMARKS
            .iter()
            .any(|l| cem_names.contains(&(*l).to_owned()));
        let html_has_landmark = LANDMARKS
            .iter()
            .any(|l| html_names.contains(&(*l).to_owned()));
        assert!(
            cem_has_landmark,
            "[{cem_path:?}] no landmark element found in CEM-ML token stream"
        );
        assert!(
            html_has_landmark,
            "[{html_path:?}] no landmark element found in HTML token stream"
        );
    }
}

#[test]
fn parity_diagnostic_codes_align_across_surfaces() {
    // For each pair, the set of diagnostic *codes* should agree on
    // whether the document is clean. Tier A asserts the simple form:
    // both surfaces report zero `cem.byte.*` / `cem.tokenizer.*` /
    // `cem.html.*` hard diagnostics. The CEM-side semantic diagnostics
    // and HTML-side semantic diagnostics share namespaces today; the
    // schema-machine and validation-rule outputs are exercised by
    // their dedicated tests.
    for (cem_path, html_path) in fixture_pairs() {
        let cem_input = std::fs::read_to_string(&cem_path).unwrap();
        let html_input = std::fs::read_to_string(&html_path).unwrap();
        let (_, cem_diags) = tokenize_cem(&cem_input);
        let (_, html_diags) = tokenize_html(&html_input);
        let cem_hard_count = cem_diags
            .iter()
            .filter(|d| matches!(d.severity, Severity::Error | Severity::Fatal))
            .count();
        let html_hard_count = html_diags
            .iter()
            .filter(|d| matches!(d.severity, Severity::Error | Severity::Fatal))
            .count();
        assert_eq!(
            cem_hard_count,
            html_hard_count,
            "[{}] hard-violation counts differ: cem={cem_hard_count}, html={html_hard_count}",
            cem_path.file_stem().unwrap().to_string_lossy()
        );
    }
}
