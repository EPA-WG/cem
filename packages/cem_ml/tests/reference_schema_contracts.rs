use cem_ml::{
    events::cem::CemEventNormalizer,
    parser::builder::CemAstBuilder,
    schema::{
        document_model::{compile_schema_document_model, validate_document_model},
        registry::{CEM_AST_PROJECTION_SCHEMA_URI, CEM_ML_SCHEMA_URI, CEM_QL_SCHEMA_URI},
    },
    source::{BytesSource, SourceId},
    tokenizer::cem::CemTokenizer,
};

const ML: &str = include_str!("../schema-packages/cem-ml/v1/schema/cem-ml-generic.cem");
const QL: &str = include_str!("../schema-packages/cem-ql/v1/schema/cem-ql.cem");
const AST: &str =
    include_str!("../schema-packages/cem-ast-projection/v1/schema/cem-ast-projection.cem");

#[test]
fn schema_packages_declare_reference_source_and_consumption_boundaries() {
    for (uri, source, contracts) in [
        (
            CEM_ML_SCHEMA_URI,
            ML,
            vec![
                ("reference-source-retention", "reference"),
                ("reference-scope-defaults", "reference"),
                ("reference-runtime-context", "reference"),
            ],
        ),
        (
            CEM_QL_SCHEMA_URI,
            QL,
            vec![
                ("reference-node-operand", "expression"),
                ("reference-target-edges", "expression"),
                ("reference-chain-consumer-resolution", "expression"),
            ],
        ),
        (
            CEM_AST_PROJECTION_SCHEMA_URI,
            AST,
            vec![
                ("reference-context-provenance", "node"),
                ("reference-target-edges", "node"),
                ("reference-source-state", "node"),
            ],
        ),
    ] {
        let model = compile_schema_document_model(uri, source);
        for (kind, target) in contracts {
            let contract = model
                .constraint(kind)
                .unwrap_or_else(|| panic!("{uri} must own {kind}"));
            assert_eq!(contract.target.as_deref(), Some(target));
            assert!(contract
                .policy
                .as_deref()
                .is_some_and(|policy| !policy.is_empty()));
            // These declare the shared boundary; runtime execution is explicit.
            assert!(contract.engine_behavior.is_none());
            assert!(!contract.source_map.frames.is_empty());
        }
    }
}

#[test]
fn projection_accepts_source_references_without_saved_context_or_targets() {
    let model = compile_schema_document_model(CEM_AST_PROJECTION_SCHEMA_URI, AST);
    for source in [
        r##"{node @id="1" @parent-id="0" @kind="reference" @expression="#nodes"}"##,
        r##"{node @id="1" @kind="reference" @expression="#nodes" @evaluation-state="unevaluated"}"##,
        r##"{node @id="1" @kind="reference" @expression="#nodes" @evaluation-state="resolved" @target-ids=""}"##,
    ] {
        let tokenizer =
            CemTokenizer::from_source(BytesSource::new(SourceId(1), source.as_bytes().to_vec()));
        let document = CemAstBuilder::new(CemEventNormalizer::new(tokenizer)).build();
        assert!(
            document.diagnostics.is_empty(),
            "{:?}",
            document.diagnostics
        );
        let diagnostics = validate_document_model(&document, &model);
        assert!(diagnostics.is_empty(), "{source}: {diagnostics:?}");
    }
    let node = model.element("node").unwrap();
    for field in ["expression", "context-id", "target-ids", "evaluation-state"] {
        assert!(node.optional_attributes.contains(field));
        assert!(!node.required_attributes.contains(field));
    }
}
