//! Namespace-rebinding fixture coverage.
//!
//! Drives each `examples/cem-ml/namespace-rebinding/*.cem` fixture
//! through the schema machine and asserts the expected
//! `NsContext` resolutions across nested scopes per AC-P-10 / AC-P-V-1
//! and `packages/cem_ml/src/schema/namespace.rs`.

use cem_ml::diagnostics::Severity;
use cem_ml::events::cem::CemEventNormalizer;
use cem_ml::interpreter::light_dom::LightDomInterpreter;
use cem_ml::parser::builder::CemAstBuilder;
use cem_ml::parser::CemAstNode;
use cem_ml::schema::machine::CemSchemaMachine;
use cem_ml::schema::vocab::CompiledSchema;
use cem_ml::source::{BytesSource, SourceId};
use cem_ml::tokenizer::cem::CemTokenizer;
use cem_ml::tokenizer::xml::XmlTokenizer;
use cem_ml::tokenizer::SchemaTokenizer;
use cem_ml::validation::{RuleContext, RuleRegistry};

fn fixture(name: &str) -> String {
    let path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../examples/cem-ml/namespace-rebinding")
        .join(name);
    std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {path:?}: {e}"))
}

#[test]
fn html_svg_html_default_rebinding_round_trip() {
    let input = fixture("default-html-svg-html.cem");
    let src = BytesSource::new(SourceId(1), input.as_bytes().to_vec());
    let tok = CemTokenizer::from_source(src);
    let normalizer = CemEventNormalizer::new(tok);
    let machine = CemSchemaMachine::new(CompiledSchema::cem_core(), normalizer);

    let mut default_history: Vec<String> = Vec::new();
    let outcome = machine.run_with_observer(|m| {
        if let Some(b) = m.current_ns_context().binding("") {
            let uri = b.namespace_uri.clone();
            if default_history.last() != Some(&uri) {
                default_history.push(uri);
            }
        }
    });

    // The history must include HTML → SVG → HTML in that order.
    let html = "http://www.w3.org/1999/xhtml";
    let svg = "http://www.w3.org/2000/svg";
    let first_html = default_history.iter().position(|u| u == html);
    let first_svg = default_history.iter().position(|u| u == svg);
    let second_html = default_history
        .iter()
        .enumerate()
        .filter(|(_, u)| *u == html)
        .nth(1)
        .map(|(i, _)| i);
    assert!(
        first_html.is_some() && first_svg.is_some() && second_html.is_some(),
        "expected default binding history to include HTML, SVG, HTML; got {default_history:?}"
    );
    let (f_html, f_svg, s_html) = (
        first_html.unwrap(),
        first_svg.unwrap(),
        second_html.unwrap(),
    );
    assert!(
        f_html < f_svg && f_svg < s_html,
        "expected default-binding order HTML → SVG → HTML; positions: {f_html}, {f_svg}, {s_html}"
    );
    // No hard violations from upstream layers.
    let hard: Vec<_> = outcome
        .diagnostics
        .iter()
        .filter(|d| {
            matches!(d.severity, Severity::Error | Severity::Fatal)
                && !d.code.starts_with("cem.handoff.")
        })
        .collect();
    assert!(hard.is_empty(), "{hard:?}");
}

#[test]
fn prefix_rebinding_uses_innermost_uri() {
    let input = fixture("prefix-rebind.cem");
    let src = BytesSource::new(SourceId(1), input.as_bytes().to_vec());
    let tok = CemTokenizer::from_source(src);
    let normalizer = CemEventNormalizer::new(tok);
    let machine = CemSchemaMachine::new(CompiledSchema::cem_core(), normalizer);

    let mut prefix_history: Vec<String> = Vec::new();
    let outcome = machine.run_with_observer(|m| {
        if let Some(b) = m.current_ns_context().binding("x") {
            let uri = b.namespace_uri.clone();
            if prefix_history.last() != Some(&uri) {
                prefix_history.push(uri);
            }
        }
    });

    // History must include outer → inner → outer.
    let outer = "https://example.test/ns/outer";
    let inner = "https://example.test/ns/inner";
    let first_outer = prefix_history.iter().position(|u| u == outer);
    let first_inner = prefix_history.iter().position(|u| u == inner);
    let second_outer = prefix_history
        .iter()
        .enumerate()
        .filter(|(_, u)| *u == outer)
        .nth(1)
        .map(|(i, _)| i);
    assert!(
        first_outer.is_some() && first_inner.is_some() && second_outer.is_some(),
        "expected prefix `x` history to include outer → inner → outer; got {prefix_history:?}"
    );
    let (a, b, c) = (
        first_outer.unwrap(),
        first_inner.unwrap(),
        second_outer.unwrap(),
    );
    assert!(
        a < b && b < c,
        "expected order outer → inner → outer; positions: {a}, {b}, {c}"
    );
    // No schema-side hard violations from this fixture.
    let hard: Vec<_> = outcome
        .diagnostics
        .iter()
        .filter(|d| {
            matches!(d.severity, Severity::Error | Severity::Fatal)
                && (d.code.starts_with("cem.schema.")
                    || d.code.starts_with("cem.ns."))
        })
        .collect();
    assert!(hard.is_empty(), "{hard:?}");
}

#[test]
fn xml_parity_fixture_runs_through_shared_pipeline() {
    let path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../examples/cem-ml/namespace-rebinding/default-html-svg-html.xml");
    let input = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("xml parity fixture missing: {path:?}: {e}"));
    let bytes = input.as_bytes().to_vec();

    let mut token_count = 0usize;
    {
        let src = BytesSource::new(SourceId(1), bytes.clone());
        let mut tok = XmlTokenizer::from_source(src);
        while tok.next_token().is_some() {
            token_count += 1;
        }
        let hard: Vec<_> = tok
            .take_diagnostics()
            .into_iter()
            .filter(|d| matches!(d.severity, Severity::Error | Severity::Fatal))
            .collect();
        assert!(hard.is_empty(), "{hard:?}");
    }
    assert!(token_count > 0, "XML tokenizer emitted no tokens");

    {
        let src = BytesSource::new(SourceId(1), bytes.clone());
        let tok = XmlTokenizer::from_source(src);
        let normalizer = CemEventNormalizer::new(tok);
        let machine = CemSchemaMachine::new(CompiledSchema::cem_core(), normalizer);
        let mut default_history: Vec<String> = Vec::new();
        let outcome = machine.run_with_observer(|m| {
            if let Some(b) = m.current_ns_context().binding("") {
                let uri = b.namespace_uri.clone();
                if default_history.last() != Some(&uri) {
                    default_history.push(uri);
                }
            }
        });
        let hard: Vec<_> = outcome
            .diagnostics
            .iter()
            .filter(|d| {
                matches!(d.severity, Severity::Error | Severity::Fatal)
                    && (d.code.starts_with("cem.schema.") || d.code.starts_with("cem.ns."))
            })
            .collect();
        assert!(hard.is_empty(), "{hard:?}");
        assert!(
            default_history.contains(&"http://www.w3.org/1999/xhtml".to_owned()),
            "expected XML default namespace history to include HTML; got {default_history:?}"
        );
        assert!(
            default_history.contains(&"http://www.w3.org/2000/svg".to_owned()),
            "expected XML default namespace history to include SVG; got {default_history:?}"
        );
    }

    let document = {
        let src = BytesSource::new(SourceId(1), bytes);
        let tok = XmlTokenizer::from_source(src);
        let normalizer = CemEventNormalizer::new(tok);
        CemAstBuilder::new(normalizer).build()
    };
    assert!(!document.nodes.is_empty(), "XML AST is empty");
    assert!(matches!(document.nodes[0], CemAstNode::Document { .. }));

    let registry = RuleRegistry::with_tier_a_rules();
    let diagnostics = registry.run(&RuleContext {
        document: &document,
        upstream_diagnostics: &document.diagnostics,
    });
    let hard: Vec<_> = diagnostics
        .iter()
        .filter(|d| matches!(d.severity, Severity::Error | Severity::Fatal))
        .collect();
    assert!(hard.is_empty(), "{hard:?}");

    let output = LightDomInterpreter::new().render(&document);
    assert!(output.rendered.contains("<main"));
    assert!(output.rendered.contains("<svg"));
    assert!(output.rendered.contains("<form"));

    assert_eq!(hard.len(), 0);
}
