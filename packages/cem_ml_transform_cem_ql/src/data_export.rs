//! Explicit JSON/YAML publication of constructed `cem:generic-data` values.
//! The retained render owner is read directly; package formatters own encoding.
use cem_ml::{
    conversion::{
        execute_json_document_output_pipeline_with_environment,
        execute_yaml_document_output_pipeline_with_environment,
        ConversionOutputPipelineEnvironment, ConversionRegistry,
        GenericDataJsonDocumentOutputSubject, GenericDataYamlDocumentOutputSubject,
    },
    engine::EngineContext,
    run_config::ScopeConfig,
    schema::registry::SchemaRegistry,
    transform_artifact::{
        TransformArtifactBody, TransformArtifactExportRequest, TransformArtifactExporter,
        TransformEncodedArtifact, TransformEncoding,
    },
    validation::generic_data::{
        GenericDataDocumentAst, GenericDataMappingEntryAst, GenericDataNumberKind,
        GenericDataSourceAst, GenericDataSourceRangeAst, GenericDataStreamDocumentAst,
        GenericDataValueAst,
    },
};
use cem_ql::render::RenderPlanNode;
use std::{
    collections::{BTreeMap, BTreeSet},
    sync::Arc,
};

pub(super) struct ConstructedDataExporter {
    schemas: SchemaRegistry,
    conversions: ConversionRegistry,
}

impl ConstructedDataExporter {
    pub(super) fn new(context: &EngineContext) -> Self {
        Self {
            schemas: context.schema_registry.clone(),
            conversions: context.converter_registry.clone(),
        }
    }
}

impl TransformArtifactExporter for ConstructedDataExporter {
    fn id(&self) -> &'static str {
        "cem-ql.constructed-data"
    }
    fn representation_id(&self) -> &'static str {
        "cem.tree-ast"
    }
    fn export(
        &self,
        request: TransformArtifactExportRequest<'_>,
    ) -> Result<Arc<TransformEncodedArtifact>, String> {
        let media = request
            .target
            .content_type
            .as_deref()
            .unwrap_or("")
            .split(';')
            .next()
            .unwrap_or("")
            .trim();
        let json =
            match (media, request.target.schema.as_deref()) {
                ("application/json", Some("https://cem.dev/ns/data/json/1")) => true,
                ("application/yaml", Some("https://cem.dev/ns/data/yaml/1")) => false,
                _ => return Err(
                    "constructed data export requires an explicit matching JSON or YAML identity"
                        .into(),
                ),
            };
        let TransformArtifactBody::CemTree(tree) = request.body else {
            return Err("constructed data requires a retained CEM tree".into());
        };
        let owner = super::native_render_values(tree)
            .ok_or("constructed data requires its native render owner")?;
        let mut remaining = 4096;
        let roots = children(&owner, &mut remaining)?;
        if roots.len() != 1 {
            return Err("constructed data requires exactly one root value".into());
        }
        let root = value(roots[0], 0, &mut remaining)?;
        let document = GenericDataDocumentAst {
            source: GenericDataSourceAst {
                uri: "memory:constructed-data".into(),
                content_type: media.into(),
                media_type: media.into(),
                parameters: BTreeMap::new(),
                byte_length: 0,
            },
            documents: vec![GenericDataStreamDocumentAst {
                index: 0,
                source_range: GenericDataSourceRangeAst::generated(),
                root: Some(root),
            }],
            line_ending: None,
        };
        let environment = ConversionOutputPipelineEnvironment {
            schema_registry: &self.schemas,
            conversion_registry: &self.conversions,
            package_artifact_reader: None,
            artifact_cache: None,
        };
        let scope = ScopeConfig {
            default_content_type: request.target.content_type.clone(),
            schema: request.target.schema.clone(),
            cemt_formatter_profile: Some("pretty".into()),
            ..ScopeConfig::default()
        };
        let execution = if json {
            execute_json_document_output_pipeline_with_environment(
                &environment,
                GenericDataJsonDocumentOutputSubject::new(document),
                &scope,
                None,
            )
        } else {
            execute_yaml_document_output_pipeline_with_environment(
                &environment,
                GenericDataYamlDocumentOutputSubject::new(document),
                &scope,
                None,
            )
        };
        if execution
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.severity.is_hard_violation())
        {
            return Err(execution
                .diagnostics
                .iter()
                .map(|d| d.message.as_str())
                .collect::<Vec<_>>()
                .join("; "));
        }
        let text = execution
            .output
            .as_ref()
            .and_then(serde_json::Value::as_str)
            .ok_or("data formatter returned no text")?;
        TransformEncodedArtifact::new(
            request.target.clone(),
            if json {
                TransformEncoding::Json
            } else {
                TransformEncoding::Text
            },
            text.as_bytes().to_vec(),
        )
        .map(Arc::new)
        .map_err(|error| error.to_string())
    }
}

fn charge(remaining: &mut usize) -> Result<(), String> {
    *remaining = remaining
        .checked_sub(1)
        .ok_or("constructed data exceeds the 4096-node export limit")?;
    Ok(())
}

fn children<'a>(
    nodes: &'a [RenderPlanNode],
    remaining: &mut usize,
) -> Result<Vec<&'a RenderPlanNode>, String> {
    let mut values = Vec::new();
    for node in nodes {
        charge(remaining)?;
        match node {
            RenderPlanNode::Text { text, .. } if text.trim().is_empty() => {}
            RenderPlanNode::Comment { .. } => {}
            RenderPlanNode::Element { .. } => values.push(node),
            _ => {
                return Err(
                    "constructed data only accepts explicit data elements and layout whitespace"
                        .into(),
                )
            }
        }
    }
    Ok(values)
}

fn value(
    node: &RenderPlanNode,
    depth: usize,
    remaining: &mut usize,
) -> Result<GenericDataValueAst, String> {
    if depth > 64 {
        return Err("constructed data exceeds the 64-level export limit".into());
    }
    let RenderPlanNode::Element {
        tag,
        namespace,
        attributes,
        children: content,
        source_map,
        ..
    } = node
    else {
        return Err("expected a data element".into());
    };
    if namespace.as_deref() != Some("cem:generic-data") || !attributes.is_empty() {
        return Err(
            "constructed data values require the cem:generic-data namespace and no attributes"
                .into(),
        );
    }
    let mut source_range = GenericDataSourceRangeAst::generated();
    source_range.source_map = Some(source_map.clone());
    match tag.as_str() {
        "array" => Ok(GenericDataValueAst::Sequence {
            source_range,
            items: children(content, remaining)?
                .into_iter()
                .map(|node| value(node, depth + 1, remaining))
                .collect::<Result<_, _>>()?,
        }),
        "object" => {
            let mut seen = BTreeSet::new();
            let mut entries = Vec::new();
            for node in children(content, remaining)? {
                let RenderPlanNode::Element {
                    tag,
                    namespace,
                    attributes,
                    children: content,
                    ..
                } = node
                else {
                    unreachable!()
                };
                if tag != "property"
                    || namespace.as_deref() != Some("cem:generic-data")
                    || attributes.len() != 1
                    || attributes[0].name != "name"
                    || attributes[0]
                        .namespace
                        .as_deref()
                        .is_some_and(|ns| !ns.is_empty())
                {
                    return Err(
                        "data objects require property elements with exactly one unqualified name"
                            .into(),
                    );
                }
                let name = &attributes[0].value;
                if !seen.insert(name.clone()) {
                    return Err(format!("duplicate data property `{name}`"));
                }
                let values = children(content, remaining)?;
                if values.len() != 1 {
                    return Err("data properties require exactly one value".into());
                }
                entries.push(GenericDataMappingEntryAst {
                    index: entries.len(),
                    source_range: source_range.clone(),
                    key: GenericDataValueAst::String {
                        source_range: source_range.clone(),
                        value: name.clone(),
                        lexeme: None,
                        style: None,
                    },
                    value: value(values[0], depth + 1, remaining)?,
                });
            }
            Ok(GenericDataValueAst::Mapping {
                source_range,
                entries,
            })
        }
        "string" | "number" | "boolean" | "null" => {
            let mut text = String::new();
            for node in content {
                charge(remaining)?;
                match node {
                    RenderPlanNode::Text { text: value, .. }
                    | RenderPlanNode::Cdata { text: value, .. } => text.push_str(value),
                    _ => return Err("data scalar content must be literal text".into()),
                }
            }
            Ok(match tag.as_str() {
                "string" => GenericDataValueAst::String {
                    source_range,
                    value: text,
                    lexeme: None,
                    style: None,
                },
                "number" => {
                    let lexeme = text.trim().to_owned();
                    lexeme
                        .parse::<serde_json::Number>()
                        .map_err(|_| "invalid data number".to_owned())?;
                    let number_kind = if lexeme.contains(['e', 'E']) {
                        GenericDataNumberKind::Exponent
                    } else if lexeme.contains('.') {
                        GenericDataNumberKind::Decimal
                    } else {
                        GenericDataNumberKind::Integer
                    };
                    GenericDataValueAst::Number {
                        source_range,
                        lexeme,
                        number_kind,
                    }
                }
                "boolean" => GenericDataValueAst::Boolean {
                    source_range,
                    value: match text.trim() {
                        "true" => true,
                        "false" => false,
                        _ => return Err("invalid data boolean".into()),
                    },
                },
                _ if text.trim().is_empty() => GenericDataValueAst::Null { source_range },
                _ => return Err("data null must be empty".into()),
            })
        }
        _ => Err(format!("unsupported data value `{tag}`")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cem_ml::source_map::SourceMapStack;
    use cem_ql::render::RenderPlanAttribute;

    fn element(tag: &str, children: Vec<RenderPlanNode>) -> RenderPlanNode {
        RenderPlanNode::Element {
            tag: tag.into(),
            namespace: Some("cem:generic-data".into()),
            qualified_name: None,
            attributes: vec![],
            children,
            source_map: SourceMapStack::default(),
        }
    }
    fn text(value: &str) -> RenderPlanNode {
        RenderPlanNode::Text {
            text: value.into(),
            source_map: SourceMapStack::default(),
        }
    }
    fn parse(node: &RenderPlanNode) -> Result<GenericDataValueAst, String> {
        value(node, 0, &mut 4096)
    }

    #[test]
    fn scalar_types_preserve_text_and_reject_invalid_lexemes() {
        assert!(
            matches!(parse(&element("string", vec![text(" Café <&> 🍒 ")])).unwrap(), GenericDataValueAst::String { value, .. } if value == " Café <&> 🍒 ")
        );
        assert!(matches!(
            parse(&element("number", vec![text("-12.5")])).unwrap(),
            GenericDataValueAst::Number {
                number_kind: GenericDataNumberKind::Decimal,
                ..
            }
        ));
        assert!(matches!(
            parse(&element("boolean", vec![text("true")])).unwrap(),
            GenericDataValueAst::Boolean { value: true, .. }
        ));
        assert!(matches!(
            parse(&element("null", vec![])).unwrap(),
            GenericDataValueAst::Null { .. }
        ));
        for (tag, lexeme) in [
            ("number", "NaN"),
            ("number", "01"),
            ("boolean", "yes"),
            ("null", "null"),
        ] {
            assert!(parse(&element(tag, vec![text(lexeme)])).is_err());
        }
        assert!(parse(&element("string", vec![element("string", vec![])])).is_err());
        let mut foreign = element("array", vec![]);
        if let RenderPlanNode::Element { namespace, .. } = &mut foreign {
            *namespace = Some("urn:other".into());
        }
        assert!(parse(&foreign).is_err());
    }

    #[test]
    fn object_properties_require_unique_names_and_one_value() {
        let mut property = element("property", vec![element("null", vec![])]);
        if let RenderPlanNode::Element { attributes, .. } = &mut property {
            attributes.push(RenderPlanAttribute {
                name: "name".into(),
                namespace: None,
                qualified_name: None,
                value: "stock".into(),
                value_stream: cem_ql::eval::ItemStream::empty(),
                contract: None,
                source_map: SourceMapStack::default(),
            });
        }
        assert!(parse(&element("object", vec![property.clone()])).is_ok());
        assert!(
            parse(&element("object", vec![property.clone(), property.clone()]))
                .unwrap_err()
                .contains("duplicate")
        );
        if let RenderPlanNode::Element { children, .. } = &mut property {
            children.clear();
        }
        assert!(parse(&element("object", vec![property])).is_err());
    }

    #[test]
    fn publication_requires_one_root_and_matching_target_identity() {
        let exporter = ConstructedDataExporter::new(&EngineContext::default());
        let target = cem_ml::engine::FormatIdentity {
            content_type: Some("application/json".into()),
            schema: Some("https://cem.dev/ns/data/json/1".into()),
            ..Default::default()
        };
        for nodes in [
            vec![],
            vec![element("null", vec![]), element("null", vec![])],
        ] {
            let tree = super::super::render_plan_to_cem_tree_nodes(&cem_ql::render::RenderPlan {
                nodes,
                host_attribute_updates: vec![],
                diagnostics: vec![],
            });
            let body = TransformArtifactBody::CemTree(Arc::new(tree));
            let request = TransformArtifactExportRequest {
                body: &body,
                target: &target,
                source_map: None,
                output_spans: &[],
            };
            assert!(exporter
                .export(request)
                .unwrap_err()
                .contains("exactly one root"));
            let mismatch = cem_ml::engine::FormatIdentity {
                schema: Some("https://cem.dev/ns/data/yaml/1".into()),
                ..target.clone()
            };
            assert!(exporter
                .export(TransformArtifactExportRequest {
                    target: &mismatch,
                    ..request
                })
                .unwrap_err()
                .contains("matching JSON or YAML identity"));
        }
    }

    #[test]
    fn export_bounds_depth_and_total_nodes() {
        let mut deep = element("null", vec![]);
        for _ in 0..66 {
            deep = element("array", vec![deep]);
        }
        assert!(parse(&deep).unwrap_err().contains("64-level"));
        assert!(
            parse(&element("array", vec![element("null", vec![]); 4097]))
                .unwrap_err()
                .contains("4096-node")
        );
    }
}
