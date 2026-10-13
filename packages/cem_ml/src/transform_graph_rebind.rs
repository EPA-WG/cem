//! Source-preserving candidate edits for bounded virtual transform graphs.

use crate::events::cem::CemEventNormalizer;
use crate::parser::{builder::CemAstBuilder, CemAstNode};
use crate::source::{BytesSource, SourceId};
use crate::source_map::FrameSpan;
use crate::tokenizer::{cem::CemTokenizer, AttributeValueSyntax, SchemaTokenKind, SchemaTokenizer};
use crate::transform_config::{
    parse_transform_graph_config, TransformGraphConfigError, TransformGraphParseRequest,
    TRANSFORM_CONFIG_SCHEMA_ELEMENTS,
};
use crate::transform_graph_dependencies::{
    discover_transform_graph_dependencies, GraphReferenceFollowUp, GraphReferenceRole,
    TransformGraphDependencies,
};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TransformGraphRebindPlan {
    /// Original inventory; transitive follow-ups are never cleared by this planner.
    pub dependencies: TransformGraphDependencies,
    pub blockers: Vec<String>,
    pub rewritten_bytes: Option<Vec<u8>>,
}

/// Rewrite exact opaque-input references using native literal token ranges.
/// Relative matches become absolute URLs; all bytes outside edited values remain
/// exact. The graph's acquisition URI must stay fixed. This is a candidate for a
/// host's atomic, revision-checked project edit, not mutation authorization.
/// Resource existence, project-wide dependencies and output ownership remain host
/// obligations. No resolver, module map, template or document is fetched/executed.
pub fn plan_transform_graph_rebind(
    request: TransformGraphParseRequest,
    from_uri: &str,
    to_uri: &str,
) -> Result<TransformGraphRebindPlan, TransformGraphConfigError> {
    plan_transform_graph_edit(request, from_uri, to_uri, false)
}

fn plan_transform_graph_edit(
    request: TransformGraphParseRequest,
    from_uri: &str,
    to_uri: &str,
    relocation: bool,
) -> Result<TransformGraphRebindPlan, TransformGraphConfigError> {
    if !literal_resource_url(from_uri) || !literal_resource_url(to_uri) {
        return Err(error(
            "rename requires canonical absolute literal URLs without query, fragment or patterns",
        ));
    }
    let dependencies = discover_transform_graph_dependencies(request.clone())?;
    if !relocation && (dependencies.config_uri == from_uri || dependencies.config_uri == to_uri) {
        return Err(error(
            "graph relocation or replacement needs a separate base-rebinding contract",
        ));
    }
    let mut plan = TransformGraphRebindPlan {
        dependencies,
        blockers: Vec::new(),
        rewritten_bytes: None,
    };
    for diagnostic in &plan.dependencies.diagnostics {
        plan.blockers
            .push(format!("{}: {}", diagnostic.code, diagnostic.message));
    }
    for reference in &plan.dependencies.references {
        if relocation
            && reference
                .resolved_uri
                .as_deref()
                .is_none_or(|uri| !literal_resource_url(uri) || uri == from_uri || uri == to_uri)
        {
            plan.blockers.push(format!(
                "{} @{}: relocation needs a literal non-self binding",
                reference.node_id, reference.field
            ));
        }
        if reference.role == GraphReferenceRole::Output {
            if reference.follow_up == GraphReferenceFollowUp::PatternExpansion
                || reference
                    .resolved_uri
                    .as_deref()
                    .is_some_and(|uri| uri == from_uri || uri == to_uri)
            {
                plan.blockers.push(format!(
                    "{} @{}: output collision or pattern needs ownership analysis",
                    reference.node_id, reference.field
                ));
            }
        } else if reference.follow_up != GraphReferenceFollowUp::None {
            plan.blockers.push(format!(
                "{} @{}: {:?} requires transitive analysis",
                reference.node_id, reference.field, reference.follow_up
            ));
        }
    }
    if !plan.blockers.is_empty() {
        return Ok(plan);
    }
    let document = CemAstBuilder::new(CemEventNormalizer::new(CemTokenizer::from_source(
        BytesSource::new(SourceId(1), request.bytes.clone()),
    )))
    .build();
    // Bind lexical ranges to native AST attributes by their source name offset.
    // Never find/replace a URI by searching source text.
    let mut tokenizer =
        CemTokenizer::from_source(BytesSource::new(SourceId(1), request.bytes.clone()));
    let mut literals = BTreeMap::new();
    while let Some(token) = tokenizer.next_token() {
        if let SchemaTokenKind::Attribute {
            name_range,
            value: Some(value),
            value_range: Some(range),
            value_syntax: AttributeValueSyntax::Literal,
            ..
        } = token.kind
        {
            literals.insert(name_range.start, (value, range));
        }
    }
    let mut locations = BTreeMap::new();
    for node in document.iter() {
        let CemAstNode::Element {
            node_id,
            expanded_name,
            attributes,
            ..
        } = node
        else {
            continue;
        };
        let schema = TRANSFORM_CONFIG_SCHEMA_ELEMENTS
            .iter()
            .find(|schema| schema.local_name == expanded_name.local_name);
        if !expanded_name.namespace_uri.is_empty() || schema.is_none() {
            plan.blockers.push(format!(
                "unsupported source element {}",
                expanded_name.local_name
            ));
            continue;
        }
        let schema = schema.unwrap();
        let mut id = format!("{}:{node_id}", expanded_name.local_name);
        let mut slots = BTreeMap::new();
        for attr in attributes {
            let Some(CemAstNode::Attribute {
                expanded_name: name,
                value,
                source,
                ..
            }) = document.get(*attr)
            else {
                continue;
            };
            if !name.namespace_uri.is_empty()
                || !(schema
                    .required_attributes
                    .contains(&name.local_name.as_str())
                    || schema
                        .optional_attributes
                        .contains(&name.local_name.as_str()))
            {
                plan.blockers
                    .push(format!("unknown source attribute {}", name.local_name));
            }
            if name.local_name == "id" {
                if let Some(value) = value {
                    id = value.clone();
                }
            }
            if let Some(frame) = source.current() {
                if let FrameSpan::Single(range) = frame.span {
                    if slots.insert(name.local_name.clone(), range.start).is_some() {
                        plan.blockers
                            .push(format!("duplicate source attribute {}", name.local_name));
                    }
                }
            }
        }
        locations.insert(id, slots);
    }
    if !plan.blockers.is_empty() || from_uri == to_uri {
        return Ok(plan);
    }
    let mut edits = Vec::new();
    let mut expected = parse_transform_graph_config(request.clone())?.graph;
    for reference in &plan.dependencies.references {
        let replacement = if relocation {
            let before = reference
                .resolved_uri
                .as_deref()
                .ok_or_else(|| error("relocation reference is unresolved"))?;
            if crate::transform_graph_request::resolve_transform_graph_reference(
                to_uri,
                &reference.authored_uri,
            ) == before
            {
                continue;
            }
            before
        } else {
            if reference.role != GraphReferenceRole::Input
                || reference.resolved_uri.as_deref() != Some(from_uri)
            {
                continue;
            }
            to_uri
        };
        let offset = locations
            .get(&reference.node_id)
            .and_then(|attrs| attrs.get(&reference.field))
            .ok_or_else(|| error("reference has no unique native attribute location"))?;
        let (value, range) = literals
            .get(offset)
            .ok_or_else(|| error("reference has no literal native value range"))?;
        if value != &reference.authored_uri {
            return Err(error("native source value differs from graph reference"));
        }
        let original = request
            .bytes
            .get(range.start as usize..range.end() as usize)
            .ok_or_else(|| error("native value range exceeds source bytes"))?;
        let quote = if original.first() == Some(&b'\'') {
            '\''
        } else {
            '"'
        };
        edits.push((
            range.start as usize,
            range.end() as usize,
            format!("{quote}{replacement}{quote}").into_bytes(),
        ));
        let node = expected
            .nodes
            .iter_mut()
            .find(|node| node.id == reference.node_id)
            .ok_or_else(|| error("graph node disappeared"))?;
        if reference.role == GraphReferenceRole::Output {
            node.out = Some(replacement.into());
        } else {
            node.src = Some(replacement.into());
        }
    }
    let changed = !edits.is_empty();
    edits.sort_by_key(|edit| edit.0);
    if edits.windows(2).any(|pair| pair[0].1 > pair[1].0) {
        return Err(error("overlapping source edits"));
    }
    let mut bytes = request.bytes.clone();
    for (start, end, replacement) in edits.into_iter().rev() {
        bytes.splice(start..end, replacement);
    }
    let mut rewritten = request;
    rewritten.bytes = bytes.clone();
    if relocation {
        rewritten.base_uri = Some(to_uri.into());
        if rewritten.identity.base_uri.is_some() {
            rewritten.identity.base_uri = Some(to_uri.into());
        }
        let after = discover_transform_graph_dependencies(rewritten.clone())?;
        let bindings = |inventory: &TransformGraphDependencies| {
            inventory
                .references
                .iter()
                .map(|r| {
                    (
                        r.node_id.clone(),
                        r.field.clone(),
                        r.role,
                        r.resolved_uri.clone(),
                        r.follow_up,
                    )
                })
                .collect::<Vec<_>>()
        };
        if !after.diagnostics.is_empty() || bindings(&after) != bindings(&plan.dependencies) {
            return Err(error("relocation changed resolved resource bindings"));
        }
    }
    let verified = parse_transform_graph_config(rewritten)?;
    if !verified.diagnostics.is_empty() || verified.graph != expected {
        return Err(error(
            "rewritten graph changed semantics beyond selected reference slots",
        ));
    }
    plan.rewritten_bytes = changed.then_some(bytes);
    Ok(plan)
}

/// Plan a graph move while preserving its existing input and output lookup keys.
/// Only opaque imports and literal outputs with no transitive dependencies are
/// admitted. Rewrite native literal ranges only when the new acquisition base would
/// change their resolved key; preserve unchanged source bytes. Returned dependencies
/// describe the original graph. Hosts must still prove incoming references, resource
/// existence, project-wide output ownership and revisions before atomic mutation.
pub fn plan_transform_graph_relocation(
    request: TransformGraphParseRequest,
    to_uri: &str,
) -> Result<TransformGraphRebindPlan, TransformGraphConfigError> {
    let from = request
        .base_uri
        .clone()
        .ok_or_else(|| error("configuration base required"))?;
    if request
        .identity
        .base_uri
        .as_deref()
        .is_some_and(|base| base != from)
    {
        return Err(error(
            "identity base conflicts with acquired configuration base",
        ));
    }
    plan_transform_graph_edit(request, &from, to_uri, true)
}

fn literal_resource_url(value: &str) -> bool {
    !value
        .chars()
        .any(|ch| ch.is_whitespace() || ch.is_control() || "\"'{}[]*|\\".contains(ch))
        && url::Url::parse(value).is_ok_and(|url| {
            !url.cannot_be_a_base()
                && url.query().is_none()
                && url.fragment().is_none()
                && url.as_str() == value
        })
}
fn error(message: impl Into<String>) -> TransformGraphConfigError {
    TransformGraphConfigError {
        code: "cem.transform_config.rebind_invalid",
        message: message.into(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::FormatIdentity;
    const OLD: &str = "studio://project/assets/old.svg";
    const NEW: &str = "studio://project/assets/new.svg";
    fn request(source: &str) -> TransformGraphParseRequest {
        TransformGraphParseRequest {
            bytes: source.as_bytes().to_vec(),
            identity: FormatIdentity {
                content_type: Some("text/cem-ml".into()),
                ..Default::default()
            },
            base_uri: Some("studio://project/graphs/run.cem".into()),
        }
    }
    #[test]
    fn configuration_relocation_preserves_inputs_outputs_and_source_bytes() {
        let source = "// café 🙂\r\n{run | {import @id=asset @src='../assets/old.svg' @opaque=true @content-type=image/svg+xml | {export @out=../out/logo.svg}}}\r\n";
        let moved = "studio://project/moved/deep/run.cem";
        let plan = plan_transform_graph_relocation(request(source), moved).unwrap();
        assert!(plan.blockers.is_empty(), "{:?}", plan.blockers);
        let bytes = plan.rewritten_bytes.unwrap();
        let rewritten = String::from_utf8(bytes.clone()).unwrap();
        assert_eq!(
            rewritten,
            source
                .replace("'../assets/old.svg'", "'studio://project/assets/old.svg'")
                .replace(
                    "@out=../out/logo.svg",
                    "@out=\"studio://project/out/logo.svg\""
                )
        );
        let mut relocated = request(&rewritten);
        relocated.base_uri = Some(moved.into());
        let after = discover_transform_graph_dependencies(relocated).unwrap();
        assert_eq!(
            after
                .references
                .iter()
                .map(|r| (&r.node_id, &r.field, &r.resolved_uri))
                .collect::<Vec<_>>(),
            plan.dependencies
                .references
                .iter()
                .map(|r| (&r.node_id, &r.field, &r.resolved_uri))
                .collect::<Vec<_>>()
        );
        use crate::command_service::VirtualResourceV1;
        use crate::transform_graph_request::{
            ManifestTransformGraphResourceProvider, TransformGraphResourceProvider,
        };
        let snapshot = BTreeMap::from([(
            OLD.into(),
            VirtualResourceV1 {
                bytes: vec![0, 255, 10],
                identity: None,
            },
        )]);
        let provider = ManifestTransformGraphResourceProvider::new(moved, &snapshot);
        let input = after
            .references
            .iter()
            .find(|r| r.role == GraphReferenceRole::Input)
            .unwrap();
        let output = after
            .references
            .iter()
            .find(|r| r.role == GraphReferenceRole::Output)
            .unwrap();
        assert_eq!(
            provider
                .import_resources(&input.authored_uri, None)
                .unwrap()[0]
                .bytes,
            vec![0, 255, 10]
        );
        assert_eq!(
            provider.output_uri(&output.authored_uri).unwrap(),
            "studio://project/out/logo.svg"
        );
    }

    #[test]
    fn configuration_relocation_same_directory_and_absolute_references_keep_exact_bytes() {
        let relative = "{run | {import @src='../assets/old.svg' @opaque=true @content-type=image/svg+xml | {export @out='../out/logo.svg'}}}";
        for target in [
            "studio://project/graphs/renamed.cem",
            "studio://project/graphs/run.cem",
        ] {
            let plan = plan_transform_graph_relocation(request(relative), target).unwrap();
            assert!(plan.blockers.is_empty());
            assert!(plan.rewritten_bytes.is_none());
        }
        let absolute = relative
            .replace("../assets/old.svg", OLD)
            .replace("../out/logo.svg", "studio://project/out/logo.svg");
        let plan =
            plan_transform_graph_relocation(request(&absolute), "studio://project/deep/run.cem")
                .unwrap();
        assert!(plan.blockers.is_empty());
        assert!(plan.rewritten_bytes.is_none());
    }

    #[test]
    fn configuration_relocation_blocks_unproven_bindings_and_conflicting_bases() {
        let source = "{run | {import @src='../assets/old.svg' @opaque=true @content-type=image/svg+xml | {export @out='../out/logo.svg'}}}";
        for blocked in [
            source.replace("@opaque=true", "@opaque=false"),
            source.replace("old.svg", "*.svg"),
            source.replace("logo.svg", "*.svg"),
            source.replace("../assets/old.svg", "run.cem"),
            source.replace("../out/logo.svg", "studio://project/moved/run.cem"),
            source.replace("../out/logo.svg", "../out/logo.svg?version=1"),
            source.replace("@opaque=true", "@opaque=true @unknown=x"),
        ] {
            let plan = plan_transform_graph_relocation(
                request(&blocked),
                "studio://project/moved/run.cem",
            )
            .unwrap();
            assert!(!plan.blockers.is_empty(), "{blocked}");
            assert!(plan.rewritten_bytes.is_none());
        }
        let mut conflicting = request(source);
        conflicting.identity.base_uri = Some("studio://project/other/run.cem".into());
        assert!(
            plan_transform_graph_relocation(conflicting, "studio://project/moved/run.cem").is_err()
        );
        for target in [
            "relative.cem",
            "studio://project/run.cem#part",
            "studio://project/*.cem",
        ] {
            assert!(plan_transform_graph_relocation(request(source), target).is_err());
        }
    }

    #[test]
    fn rewrites_only_owned_value_ranges_and_preserves_unicode_and_quotes() {
        let source = "// café source\n{run |\n  {import @id=asset @src = '../assets/old.svg' @opaque=true @content-type=\"image/svg+xml\" |\n    {export @out=\"../out/logo.svg\"}\n  }\n}\n";
        let plan = plan_transform_graph_rebind(request(source), OLD, NEW).unwrap();
        assert!(plan.blockers.is_empty(), "{:?}", plan.blockers);
        let output = String::from_utf8(plan.rewritten_bytes.unwrap()).unwrap();
        assert_eq!(
            output,
            source.replace("'../assets/old.svg'", &format!("'{NEW}'"))
        );
        assert_eq!(
            plan.dependencies.references[0].authored_uri,
            "../assets/old.svg"
        );
        use crate::command_service::VirtualResourceV1;
        use crate::transform_graph_request::{
            ManifestTransformGraphResourceProvider, TransformGraphResourceProvider,
        };
        let resources = BTreeMap::from([(
            NEW.to_owned(),
            VirtualResourceV1 {
                bytes: b"<svg/>".to_vec(),
                identity: None,
            },
        )]);
        let rebound = discover_transform_graph_dependencies(request(&output)).unwrap();
        let provider = ManifestTransformGraphResourceProvider::new(&rebound.config_uri, &resources);
        let acquired = provider
            .import_resources(&rebound.references[0].authored_uri, None)
            .unwrap();
        assert_eq!(acquired[0].uri, NEW);
        assert_eq!(acquired[0].bytes, b"<svg/>");
    }
    #[test]
    fn multiple_matches_are_patched_without_offset_drift_and_noops_keep_bytes() {
        let source = "{run | {import @src=../assets/old.svg @opaque=true @content-type=image/svg+xml | {export @out=../out/one.svg}}\n{import @src=\"../assets/old.svg\" @opaque=true @content-type=image/svg+xml | {export @out=../out/two.svg}}}";
        let plan = plan_transform_graph_rebind(request(source), OLD, NEW).unwrap();
        let output = String::from_utf8(plan.rewritten_bytes.unwrap()).unwrap();
        assert_eq!(
            output,
            source
                .replace("@src=../assets/old.svg", &format!("@src=\"{NEW}\""))
                .replace("@src=\"../assets/old.svg\"", &format!("@src=\"{NEW}\""))
        );
        assert!(plan_transform_graph_rebind(request(&output), OLD, NEW)
            .unwrap()
            .rewritten_bytes
            .is_none());
        assert!(plan_transform_graph_rebind(request(source), OLD, OLD)
            .unwrap()
            .rewritten_bytes
            .is_none());
    }
    #[test]
    fn transitive_documents_and_patterns_block_all_changes() {
        for tail in [
            "{import @src=../data.xml | {export @out=../out/data.xml}}",
            "{import @src=../assets/*.svg @opaque=true @content-type=image/svg+xml | {export @out=../out/}}",
        ] {
            let source = format!("{{run | {{import @src=../assets/old.svg @opaque=true @content-type=image/svg+xml | {{export @out=../out/logo.svg}}}} {tail}}}");
            let plan = plan_transform_graph_rebind(request(&source), OLD, NEW).unwrap();
            assert!(!plan.blockers.is_empty());
            assert!(plan.rewritten_bytes.is_none());
        }
    }
    #[test]
    fn output_collisions_and_graph_relocation_are_rejected() {
        for destination in [OLD, NEW] {
            let source = format!("{{run | {{import @src=../assets/old.svg @opaque=true @content-type=image/svg+xml | {{export @out=\"{destination}\"}}}}}}");
            let plan = plan_transform_graph_rebind(request(&source), OLD, NEW).unwrap();
            assert!(!plan.blockers.is_empty());
            assert!(plan.rewritten_bytes.is_none());
        }
        assert!(plan_transform_graph_rebind(
            request("{run}"),
            "studio://project/graphs/run.cem",
            NEW
        )
        .is_err());
        assert!(plan_transform_graph_rebind(request("{run}"), OLD, "../new.svg").is_err());
    }
    #[test]
    fn invalid_graph_and_duplicate_reference_attributes_cannot_produce_edits() {
        for source in [
            "{run | {import @src=../assets/old.svg | {export}}}",
            "{run | {import @src=../assets/old.svg @src=../assets/old.svg @opaque=true @content-type=image/svg+xml | {export @out=../out/logo.svg}}}",
            "{run | {import @src=../assets/old.svg @futureDependency=../other.svg @opaque=true @content-type=image/svg+xml | {export @out=../out/logo.svg}}}",
        ] {
            let result = plan_transform_graph_rebind(request(source), OLD, NEW);
            assert!(result.is_err() || result.unwrap().rewritten_bytes.is_none());
        }
    }
}
