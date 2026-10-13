//! Native immediate dependency inventory for transform graphs. No fetch or execution.

use crate::diagnostics::Diagnostic;
use crate::transform_config::{
    parse_transform_graph_config, TransformGraphConfigError, TransformGraphNodeKind,
    TransformGraphParseRequest,
};
use crate::transform_graph_request::resolve_transform_graph_reference;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum GraphReferenceRole {
    Input,
    Template,
    ModuleMap,
    Output,
}

/// Work still required before project-level mutation may rely on this reference.
/// Even `None` says only that this particular opaque input has no content scan.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum GraphReferenceFollowUp {
    None,
    Document,
    Template,
    ModuleMap,
    OutputOwnership,
    PatternExpansion,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GraphResourceReference {
    pub node_id: String,
    /// Native graph attribute name, not a JSON pointer or a source edit location.
    pub field: String,
    pub role: GraphReferenceRole,
    pub authored_uri: String,
    /// Exact virtual execution lookup key; absent for patterns requiring expansion.
    pub resolved_uri: Option<String>,
    pub follow_up: GraphReferenceFollowUp,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TransformGraphDependencies {
    pub config_uri: String,
    pub references: Vec<GraphResourceReference>,
    pub diagnostics: Vec<Diagnostic>,
}

/// Discover immediate virtual-graph references through the native CEM parser.
/// Relative references use the same resolver as ManifestTransformGraphResourceProvider.
/// The supplied base must be the acquired graph URI, not the project root or a
/// template's scope base. No fetching, template evaluation or source rewriting occurs.
///
/// This inventory is NOT a complete dependency closure or permission to delete or
/// rename. Hosts must inspect diagnostics and follow-ups, account for resolver and
/// module-map provenance and outputs, and retain the original source for later edits.
/// Graph node links, schema/namespace identities and arbitrary params are not URI
/// slots; params may influence template execution and belong to that later analysis.
pub fn discover_transform_graph_dependencies(
    request: TransformGraphParseRequest,
) -> Result<TransformGraphDependencies, TransformGraphConfigError> {
    let config_uri = request.base_uri.clone().filter(|base| {
        url::Url::parse(base).is_ok_and(|uri| {
            !uri.cannot_be_a_base() && uri.query().is_none() && uri.fragment().is_none()
                && uri.as_str() == base
        })
    }).ok_or_else(|| TransformGraphConfigError {
        code: "cem.transform_config.dependency_base_required",
        message: "dependency discovery requires the canonical absolute acquired graph URL without query or fragment".into(),
    })?;
    let parsed = parse_transform_graph_config(request)?;
    let mut result = TransformGraphDependencies {
        config_uri,
        references: Vec::new(),
        diagnostics: parsed.diagnostics,
    };
    if result
        .diagnostics
        .iter()
        .any(|d| d.severity.is_hard_violation())
    {
        return Ok(result);
    }
    for node in parsed.graph.nodes {
        match node.kind {
            TransformGraphNodeKind::Import => add(
                &mut result,
                &node.id,
                "src",
                node.src.as_deref(),
                GraphReferenceRole::Input,
                if node.opaque {
                    GraphReferenceFollowUp::None
                } else {
                    GraphReferenceFollowUp::Document
                },
            ),
            TransformGraphNodeKind::Transform => add(
                &mut result,
                &node.id,
                "src",
                node.src.as_deref(),
                GraphReferenceRole::Template,
                GraphReferenceFollowUp::Template,
            ),
            TransformGraphNodeKind::ImportMapRewrite => {
                add(
                    &mut result,
                    &node.id,
                    "source-map",
                    node.source_map.as_deref(),
                    GraphReferenceRole::ModuleMap,
                    GraphReferenceFollowUp::ModuleMap,
                );
                add(
                    &mut result,
                    &node.id,
                    "target-map",
                    node.target_map.as_deref(),
                    GraphReferenceRole::ModuleMap,
                    GraphReferenceFollowUp::ModuleMap,
                );
            }
            TransformGraphNodeKind::Export => add(
                &mut result,
                &node.id,
                "out",
                node.out.as_deref(),
                GraphReferenceRole::Output,
                GraphReferenceFollowUp::OutputOwnership,
            ),
            TransformGraphNodeKind::Join | TransformGraphNodeKind::Convert => {}
        }
    }
    Ok(result)
}

fn add(
    result: &mut TransformGraphDependencies,
    node_id: &str,
    field: &str,
    authored: Option<&str>,
    role: GraphReferenceRole,
    follow_up: GraphReferenceFollowUp,
) {
    let Some(authored) = authored else {
        return;
    };
    // Includes captured graph binding patterns as well as file globs. Discovery
    // must not invent one target where execution needs a resource-set expansion.
    let pattern = authored.contains(['*', '{', '}', '[', ']']);
    result.references.push(GraphResourceReference {
        node_id: node_id.into(),
        field: field.into(),
        role,
        authored_uri: authored.into(),
        resolved_uri: (!pattern)
            .then(|| resolve_transform_graph_reference(&result.config_uri, authored)),
        follow_up: if pattern {
            GraphReferenceFollowUp::PatternExpansion
        } else {
            follow_up
        },
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::FormatIdentity;
    use crate::transform_config::TRANSFORM_CONFIG_SCHEMA_URI;

    fn request(source: &str) -> TransformGraphParseRequest {
        TransformGraphParseRequest {
            bytes: source.as_bytes().to_vec(),
            identity: FormatIdentity {
                content_type: Some("text/cem-ml".into()),
                schema: Some(TRANSFORM_CONFIG_SCHEMA_URI.into()),
                ..Default::default()
            },
            base_uri: Some("studio://project/graphs/run.cem".into()),
        }
    }

    #[test]
    fn resolves_relative_reads_and_outputs_using_execution_rules() {
        let source = r#"{run | {import @id=data @src="../data/input.xml" |
            {export @id=out @out="../output/result.xml"}
        }}"#;
        let result = discover_transform_graph_dependencies(request(source)).unwrap();
        assert!(result.diagnostics.is_empty(), "{:?}", result.diagnostics);
        assert_eq!(result.references.len(), 2);
        let input = &result.references[0];
        assert_eq!(input.node_id, "data");
        assert_eq!(input.field, "src");
        assert_eq!(input.authored_uri, "../data/input.xml");
        assert_eq!(
            input.resolved_uri.as_deref(),
            Some("studio://project/data/input.xml")
        );
        assert_eq!(input.role, GraphReferenceRole::Input);
        assert_eq!(input.follow_up, GraphReferenceFollowUp::Document);
        let output = &result.references[1];
        assert_eq!(output.role, GraphReferenceRole::Output);
        assert_eq!(output.follow_up, GraphReferenceFollowUp::OutputOwnership);
        use crate::command_service::VirtualResourceV1;
        use crate::transform_graph_request::{
            ManifestTransformGraphResourceProvider, TransformGraphResourceProvider,
        };
        let resources = std::collections::BTreeMap::from([(
            "studio://project/data/input.xml".into(),
            VirtualResourceV1 {
                bytes: b"<data/>".to_vec(),
                identity: None,
            },
        )]);
        let provider = ManifestTransformGraphResourceProvider::new(&result.config_uri, &resources);
        let acquired = provider
            .import_resources(&input.authored_uri, None)
            .unwrap();
        assert_eq!(acquired[0].uri, input.resolved_uri.as_deref().unwrap());
        assert_eq!(
            provider.output_uri(&output.authored_uri).unwrap(),
            output.resolved_uri.as_deref().unwrap()
        );

        assert_eq!(
            output.resolved_uri.as_deref(),
            Some("studio://project/output/result.xml")
        );
    }

    #[test]
    fn identities_node_links_and_parameters_are_not_resource_slots() {
        let source = r#"{run | {import @id=data @src="../data/input.xml"
            @schema="studio://project/identity" |
            {transform @id=render @src="../templates/card.cem" @input=data
                @template-schema="https://cem.dev/ns/transform/cem-ql/1" |
                {param @name=label @value="../data/input.xml"}
                {export @out="../output/result.xml"}
            }
        }}"#;
        let result = discover_transform_graph_dependencies(request(source)).unwrap();
        assert!(result.diagnostics.is_empty(), "{:?}", result.diagnostics);
        assert_eq!(result.references.len(), 3);
        assert_eq!(result.references[1].role, GraphReferenceRole::Template);
        assert_eq!(
            result.references[1].follow_up,
            GraphReferenceFollowUp::Template
        );
        assert_eq!(
            result.references[1].resolved_uri.as_deref(),
            Some("studio://project/templates/card.cem")
        );
    }

    #[test]
    fn inventories_both_module_maps_and_marks_transitive_inspection() {
        let source = r#"{run | {import @src="../data/index.html" |
            {rewrite-importmap @source-map="../maps/source.json" @target-map="../maps/target.json" |
                {export @out="../output/index.html"}
            }
        }}"#;
        let result = discover_transform_graph_dependencies(request(source)).unwrap();
        assert!(result.diagnostics.is_empty(), "{:?}", result.diagnostics);
        let maps: Vec<_> = result
            .references
            .iter()
            .filter(|r| r.role == GraphReferenceRole::ModuleMap)
            .collect();
        assert_eq!(maps.len(), 2);
        assert_eq!(maps[0].field, "source-map");
        assert_eq!(maps[1].field, "target-map");
        assert!(maps
            .iter()
            .all(|r| r.follow_up == GraphReferenceFollowUp::ModuleMap));
    }

    #[test]
    fn opaque_imports_are_leaves_and_globs_have_no_single_resolved_target() {
        let result = discover_transform_graph_dependencies(request(
            r#"{run | {import @src="../assets/logo.svg" @opaque=true @content-type="image/svg+xml" | {export @out="../out/logo.svg"}}}"#,
        )).unwrap();
        assert!(result.diagnostics.is_empty(), "{:?}", result.diagnostics);
        assert_eq!(result.references[0].follow_up, GraphReferenceFollowUp::None);
        let result = discover_transform_graph_dependencies(request(
            r#"{run | {import @src="../assets/*.svg" @opaque=true @content-type="image/svg+xml" | {export @out="../out/"}}}"#,
        ))
        .unwrap();
        assert_eq!(
            result.references[0].follow_up,
            GraphReferenceFollowUp::PatternExpansion
        );
        assert!(result.references[0].resolved_uri.is_none());
    }

    #[test]
    fn invalid_graphs_return_diagnostics_without_partial_inventories() {
        let result = discover_transform_graph_dependencies(request(
            r#"{run | {import @src="../data.xml" | {export}}}"#,
        ))
        .unwrap();
        assert!(result
            .diagnostics
            .iter()
            .any(|d| d.severity.is_hard_violation()));
        assert!(result.references.is_empty());
        let mut missing_base = request("{run}");
        missing_base.base_uri = None;
        assert!(discover_transform_graph_dependencies(missing_base).is_err());
    }
}
