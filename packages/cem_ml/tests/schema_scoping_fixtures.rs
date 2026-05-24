//! Schema-scoping fixture coverage.
//!
//! Drives each `examples/cem-ml/schema-scoping/*.cem` fixture through
//! the schema machine and asserts the expected `SchemaSource` state
//! and / or diagnostic codes per AC-F-2 and
//! `packages/cem_ml/docs/cross-surface-conversion.md`.

use cem_ml::diagnostics::{Diagnostic, Severity};
use cem_ml::events::cem::CemEventNormalizer;
use cem_ml::schema::machine::CemSchemaMachine;
use cem_ml::schema::scoping::SchemaSource;
use cem_ml::schema::vocab::CompiledSchema;
use cem_ml::source::{ByteRange, BytesSource, SourceId};
use cem_ml::tokenizer::cem::CemTokenizer;

fn fixture_path(stem: &str) -> std::path::PathBuf {
    std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../examples/cem-ml/schema-scoping")
        .join(format!("{stem}.cem"))
}

fn read(stem: &str) -> String {
    let path = fixture_path(stem);
    std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {path:?}: {e}"))
}

fn run_collecting_active_sources(input: &str) -> (Vec<Diagnostic>, Vec<SchemaSource>) {
    let src = BytesSource::new(SourceId(1), input.as_bytes().to_vec());
    let tok = CemTokenizer::from_source(src);
    let normalizer = CemEventNormalizer::new(tok);
    let machine = CemSchemaMachine::new(CompiledSchema::cem_core(), normalizer);
    let mut seen_actives: Vec<SchemaSource> = Vec::new();
    let outcome = machine.run_with_observer(|m| {
        let active = m.schema_scopes().current().active.clone();
        if seen_actives.last() != Some(&active) {
            seen_actives.push(active);
        }
    });
    (outcome.diagnostics, seen_actives)
}

fn hard_scoping_violations(diags: &[Diagnostic]) -> Vec<&Diagnostic> {
    diags
        .iter()
        .filter(|d| matches!(d.severity, Severity::Error | Severity::Fatal))
        .filter(|d| {
            !d.code.starts_with("cem.byte.")
                && !d.code.starts_with("cem.tokenizer.")
                && !d.code.starts_with("cem.handoff.")
        })
        .collect()
}

#[test]
fn inline_declaration_resolves_in_descendants() {
    let input = read("inline-declaration");
    let src = BytesSource::new(SourceId(1), input.as_bytes().to_vec());
    let tok = CemTokenizer::from_source(src);
    let normalizer = CemEventNormalizer::new(tok);
    let machine = CemSchemaMachine::new(CompiledSchema::cem_core(), normalizer);
    let mut saw_resolvable = false;
    let outcome = machine.run_with_observer(|m| {
        if m.schema_scopes().resolve_name("badge").is_some() {
            saw_resolvable = true;
        }
    });
    assert!(
        saw_resolvable,
        "expected `cem:name=\"badge\"` to be resolvable in descendants"
    );
    let hard = hard_scoping_violations(&outcome.diagnostics);
    assert!(hard.is_empty(), "unexpected hard diagnostics: {hard:?}");
}

#[test]
fn wrapping_switch_activates_then_restores() {
    let input = read("wrapping-switch");
    let (diags, actives) = run_collecting_active_sources(&input);
    assert!(
        actives
            .iter()
            .any(|a| matches!(a, SchemaSource::Uri(u) if u == "schema://wrapping/example/1")),
        "expected SchemaSource::Uri inside the wrapping scope; got {actives:?}"
    );
    // Default → Uri → Default at minimum: count Default occurrences.
    let default_count = actives
        .iter()
        .filter(|a| matches!(a, SchemaSource::Default))
        .count();
    assert!(
        default_count >= 2,
        "expected at least two Default observations (before + after wrapping switch); got {actives:?}"
    );
    let hard = hard_scoping_violations(&diags);
    assert!(hard.is_empty(), "unexpected hard diagnostics: {hard:?}");
}

#[test]
fn select_switch_records_select_source() {
    let input = read("select-switch");
    let (diags, actives) = run_collecting_active_sources(&input);
    assert!(
        actives
            .iter()
            .any(|a| matches!(a, SchemaSource::Select(s) if s == ".schemas.active")),
        "expected SchemaSource::Select; got {actives:?}"
    );
    let hard = hard_scoping_violations(&diags);
    assert!(hard.is_empty(), "unexpected hard diagnostics: {hard:?}");
}

#[test]
fn self_closing_sibling_switch_activates_for_following_siblings() {
    let input = read("self-closing-sibling-switch");
    let (diags, actives) = run_collecting_active_sources(&input);
    assert!(
        actives
            .iter()
            .any(|a| matches!(a, SchemaSource::Uri(u) if u == "schema://sibling/example/1")),
        "expected no-body cem:schema switch to activate Uri for sibling scope; got {actives:?}"
    );
    let hard = hard_scoping_violations(&diags);
    assert!(hard.is_empty(), "unexpected hard diagnostics: {hard:?}");
}

#[test]
fn host_node_switch_activates_only_inside_host() {
    let input = read("host-node-switch");
    let (diags, actives) = run_collecting_active_sources(&input);
    assert!(
        actives
            .iter()
            .any(|a| matches!(a, SchemaSource::Uri(u) if u == "schema://host-node/profile/1")),
        "expected host-node switch to activate Uri on the section; got {actives:?}"
    );
    let hard = hard_scoping_violations(&diags);
    assert!(hard.is_empty(), "unexpected hard diagnostics: {hard:?}");
}

#[test]
fn src_and_select_together_emits_exclusivity_error() {
    let input = read("src-select-exclusivity");
    let (diags, _) = run_collecting_active_sources(&input);
    assert!(
        diags
            .iter()
            .any(|d| d.code == "cem.schema.scoping.exclusive_src_select"),
        "expected exclusive_src_select diagnostic; got {diags:?}"
    );
}

#[test]
fn host_form_src_and_select_together_emits_exclusivity_error() {
    let input = read("host-src-select-exclusivity");
    let (diags, _) = run_collecting_active_sources(&input);
    assert!(
        diags
            .iter()
            .any(|d| d.code == "cem.schema.scoping.exclusive_src_select"),
        "expected exclusive_src_select diagnostic on host-form attributes; got {diags:?}"
    );
}

#[test]
fn inline_declaration_does_not_leak_to_sibling_scope() {
    let input = read("sibling-isolation");
    let src = BytesSource::new(SourceId(1), input.as_bytes().to_vec());
    let tok = CemTokenizer::from_source(src);
    let normalizer = CemEventNormalizer::new(tok);
    let machine = CemSchemaMachine::new(CompiledSchema::cem_core(), normalizer);
    let mut saw_local = false;
    let mut leaked_to_aside = false;
    let outcome = machine.run_with_observer(|m| {
        if m.schema_scopes().resolve_name("Local").is_some() {
            saw_local = true;
        }
        if matches!(
            m.schema_scopes().current().active,
            SchemaSource::Uri(ref u) if u == "schema://sibling-isolation/aside/1"
        ) && m.schema_scopes().resolve_name("Local").is_some()
        {
            leaked_to_aside = true;
        }
    });
    assert!(
        saw_local,
        "Local declaration was never visible in its own scope"
    );
    assert!(
        !leaked_to_aside,
        "Local declaration leaked into sibling aside scope"
    );
    let hard = hard_scoping_violations(&outcome.diagnostics);
    assert!(hard.is_empty(), "unexpected hard diagnostics: {hard:?}");
}

#[test]
fn nested_cem_name_shadows_outer_definition() {
    let input = read("name-shadowing");
    let src = BytesSource::new(SourceId(1), input.as_bytes().to_vec());
    let tok = CemTokenizer::from_source(src);
    let normalizer = CemEventNormalizer::new(tok);
    let machine = CemSchemaMachine::new(CompiledSchema::cem_core(), normalizer);
    let mut outer_body: Option<ByteRange> = None;
    let mut inner_body: Option<ByteRange> = None;
    let mut after_inner_body: Option<ByteRange> = None;
    let outcome = machine.run_with_observer(|m| {
        if let Some(decl) = m.schema_scopes().resolve_name("Item") {
            if outer_body.is_none() {
                outer_body = Some(decl.body_byte_range);
            } else if Some(decl.body_byte_range) != outer_body {
                inner_body = Some(decl.body_byte_range);
            }
            if matches!(
                m.schema_scopes().current().active,
                SchemaSource::Uri(ref u) if u == "schema://name-shadowing/after-inner/1"
            ) {
                after_inner_body = Some(decl.body_byte_range);
            }
        }
    });
    assert!(outer_body.is_some(), "outer Item declaration not observed");
    assert!(
        inner_body.is_some(),
        "inner Item declaration did not shadow outer; outer={outer_body:?}"
    );
    assert_ne!(
        outer_body, inner_body,
        "inner Item should have a distinct body byte range"
    );
    assert_eq!(
        after_inner_body, outer_body,
        "after nested scope closes, Item should resolve back to outer declaration"
    );
    assert!(
        outcome
            .diagnostics
            .iter()
            .all(|d| d.code != "cem.schema.duplicate_cem_name"),
        "duplicate-name diagnostics should not fire on nested cem:name shadowing"
    );
}
