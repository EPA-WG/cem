use cem_ml::{
    engine::{FormatIdentity, TemplateInput, TransformTemplateEntrypoint},
    events::cem::CemEventNormalizer,
    parser::{builder::CemAstBuilder, document::CemDocument},
    run_config::ScopeConfig,
    source::{BytesSource, SourceId},
    tokenizer::cem::CemTokenizer,
    transform_artifact::TransformArtifactBody,
    transform_template::{
        TransformTemplateAdapter, TransformTemplateCompileRequest, TransformTemplateDataArtifact,
        TransformTemplateParameterArena, TransformTemplateRenderRequest,
    },
};
use cem_ml_transform_cem_ql::CemQlTransformTemplateAdapter;
use std::{collections::BTreeMap, sync::Arc};

fn document(source: &str) -> Arc<CemDocument> {
    let document = CemAstBuilder::new(CemEventNormalizer::new(CemTokenizer::from_source(
        BytesSource::new(SourceId(77), source.as_bytes().to_vec()),
    )))
    .build();
    assert!(
        document.diagnostics.is_empty(),
        "{:?}",
        document.diagnostics
    );
    Arc::new(document)
}

#[test]
fn native_document_metadata_and_authored_attributes_keep_separate_owners_and_fields() {
    let adapter = CemQlTransformTemplateAdapter;
    let template = TemplateInput {
        uri: "memory:attribute-view.cem".into(),
        bytes: br#"{p | {$input.kind}:{$input.children.kind}:{$seq:where(input.children.attributes, fn(a) => a.name == "kind").value}:{$seq:count(datadom.attributes.kind)}:{$seq:count(input.attributes)}}{$input.children}"#.to_vec(),
        identity: None,
        root_scope: ScopeConfig::default(),
    };
    let compiled = adapter
        .compile(TransformTemplateCompileRequest {
            template: &template,
            entrypoint: &TransformTemplateEntrypoint::implicit(),
            params: &TransformTemplateParameterArena::default(),
            data_bindings: &["input".into()],
            module_options: Default::default(),
            module_preflight: Default::default(),
            execution_policy: Default::default(),
        })
        .unwrap();
    assert!(
        compiled.diagnostics.is_empty(),
        "{:?}",
        compiled.diagnostics
    );
    for (attribute, value) in [
        ("", ""),
        (" @kind=asset", "asset"),
        (" @kind=document", "document"),
    ] {
        let source = format!("{{article{attribute} | Café 🧬}}");
        let owner = document(&source);
        let original_node_count = owner.nodes.len();
        let attributes = |document: &CemDocument| {
            document
                .nodes
                .iter()
                .filter_map(|node| {
                    let cem_ml::parser::CemAstNode::Attribute {
                        expanded_name,
                        value,
                        source,
                        ..
                    } = node
                    else {
                        return None;
                    };
                    Some((
                        expanded_name.namespace_uri.clone(),
                        expanded_name.local_name.clone(),
                        value.clone(),
                        source.clone(),
                    ))
                })
                .collect::<Vec<_>>()
        };
        let original_attributes = attributes(&owner);
        let artifact = TransformTemplateDataArtifact::new(
            "asset",
            Some("memory:asset.cem".into()),
            Some(FormatIdentity {
                content_type: Some("text/cem-ml".into()),
                ..Default::default()
            }),
            TransformArtifactBody::CemDocument(owner.clone()),
        );
        for _ in 0..2 {
            let response = adapter
                .render(TransformTemplateRenderRequest {
                    compiled: &compiled.artifact,
                    primary_input: &artifact,
                    secondary_inputs: &BTreeMap::new(),
                    target: None,
                    target_scope: &ScopeConfig::default(),
                    execution_policy: Default::default(),
                })
                .unwrap();
            assert!(
                response.diagnostics.is_empty(),
                "{:?}",
                response.diagnostics
            );
            let expected_attribute = if value.is_empty() {
                String::new()
            } else {
                format!(" kind=\"{value}\"")
            };
            assert_eq!(response.output.encoded_text_value().unwrap(),
                format!("<p>document:element:{value}:0:0</p><article{expected_attribute}>Café 🧬</article>"));
            assert!(
                response
                    .output
                    .output_spans
                    .iter()
                    .flat_map(|span| &span.origin.frames)
                    .any(|frame| frame.source_id == SourceId(77)),
                "original input provenance was lost"
            );
            let TransformArtifactBody::CemDocument(retained) = &artifact.body else {
                panic!("native CEM owner")
            };
            assert!(Arc::ptr_eq(retained, &owner));
            assert_eq!(owner.nodes.len(), original_node_count);
            assert_eq!(
                attributes(&owner),
                original_attributes,
                "rendering must not inject metadata into source attributes"
            );
        }
    }
}
