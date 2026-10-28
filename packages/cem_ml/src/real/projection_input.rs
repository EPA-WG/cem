//! Structural projection-source exports use the retained common tree.
use super::*;
use crate::import::projection::ProjectionDocument;

pub(super) fn convert(
    request: &ConvertRequest,
    document: &ProjectionDocument,
    target: LayerFormat,
) -> Result<PrimaryBytes, String> {
    if matches!(target, LayerFormat::AstBin | LayerFormat::DomBin) {
        let artifact = if target == LayerFormat::AstBin {
            projection::ast_binary_projection_artifact(document.tree.ast())
        } else {
            projection::dom_binary_projection_artifact(document.tree.ast())
        };
        return Ok(primary_bytes_from_binary_artifact(&artifact));
    }
    if matches!(target, LayerFormat::DomJson | LayerFormat::Ast) {
        let (value, mime, schema) = if target == LayerFormat::Ast {
            (
                projection::ast_json(document.tree.ast()),
                crate::schema::registry::CEM_AST_JSON_PROJECTION_CONTENT_TYPE,
                crate::schema::registry::CEM_AST_PROJECTION_SCHEMA_URI,
            )
        } else {
            (
                projection::dom_json(document.tree.ast()),
                CEM_DOM_JSON_PROJECTION_CONTENT_TYPE,
                crate::schema::registry::CEM_DOM_PROJECTION_SCHEMA_URI,
            )
        };
        let bytes = crate::graph_output::structured_document_bytes(
            &value,
            Some(&FormatIdentity {
                content_type: Some(mime.into()),
                ..Default::default()
            }),
        )
        .map_err(|e| e.to_string())?;
        let text = std::str::from_utf8(&bytes).map_err(|e| e.to_string())?;
        return Ok(primary_bytes_from_text_document(text, mime, schema));
    }
    if !matches!(
        target,
        LayerFormat::Cem | LayerFormat::Xml | LayerFormat::Html
    ) {
        return Err("Projection input supports passive common-tree CEM, XML, HTML or named DOM/AST JSON/binary exports; runtime replay requires separately admitted owners".into());
    }
    let stream = writer_stream(document, target)?;
    if target == LayerFormat::Cem {
        return cem_presentation_primary_bytes(
            &request.context,
            stream,
            CEM_ML_SCHEMA_URI,
            &request.target_scope,
            &request.input.uri,
            &request.input.bytes,
            "projection-source",
        )
        .map_err(|e| e.to_string());
    }
    let pipeline = if target == LayerFormat::Html {
        direct_html_output_pipeline()
    } else {
        direct_xml_output_pipeline()
    };
    let mut capture = None;
    convert_markup_tree_stream_with_cemt_pipeline(
        Some(&request.context),
        stream,
        pipeline,
        if target == LayerFormat::Html {
            "html"
        } else {
            "xml"
        },
        "projection-source-markup",
        &request.target_scope,
        Some(&request.input.uri),
        Some((&request.input, &mut capture)),
    )
    .map_err(|d| {
        d.into_iter()
            .map(|d| d.message)
            .collect::<Vec<_>>()
            .join("; ")
    })?;
    capture.ok_or_else(|| "Projection writer produced no owned bytes".into())
}
fn writer_stream(
    document: &ProjectionDocument,
    target: LayerFormat,
) -> Result<projection::CemTreeAstStream, String> {
    use projection::{CemTreeAstAttribute as A, CemTreeAstNode as N};
    let ast = document.tree.ast();
    let mut namespaces = std::collections::BTreeSet::new();
    for node in &ast.nodes {
        if let crate::parser::CemAstNode::Element { expanded_name, .. }
        | crate::parser::CemAstNode::Attribute { expanded_name, .. } = node
        {
            if !expanded_name.namespace_uri.is_empty()
                && expanded_name.namespace_uri != "http://www.w3.org/2000/xmlns/"
            {
                namespaces.insert(expanded_name.namespace_uri.clone());
            }
        }
    }
    let has_references = ast
        .nodes
        .iter()
        .any(|n| matches!(n, crate::parser::CemAstNode::Reference { .. }));
    if has_references {
        namespaces.insert(CEM_ML_SCHEMA_URI.into());
    }
    let bindings: std::collections::BTreeMap<_, _> = namespaces
        .into_iter()
        .enumerate()
        .map(|(i, uri)| {
            let prefix = if uri == "http://www.w3.org/XML/1998/namespace" {
                "xml".into()
            } else {
                format!("n{}", i + 1)
            };
            (uri, prefix)
        })
        .collect();
    let mut names = std::collections::BTreeMap::new();
    for node in &ast.nodes {
        if let crate::parser::CemAstNode::Element { expanded_name, .. }
        | crate::parser::CemAstNode::Attribute { expanded_name, .. } = node
        {
            let n = expanded_name;
            if n.namespace_uri.is_empty() {
                continue;
            }
            if let Some(prefix) = bindings.get(&n.namespace_uri) {
                let source = format!("{}:{}", n.namespace_uri, n.local_name);
                let target = format!("{prefix}:{}", n.local_name);
                if names
                    .insert(source, target.clone())
                    .is_some_and(|v| v != target)
                {
                    return Err("Ambiguous expanded projection name".into());
                }
            }
        }
    }
    if has_references {
        let prefix = bindings
            .get(CEM_ML_SCHEMA_URI)
            .expect("reference namespace");
        names.insert("cem:expr".into(), format!("{prefix}:expr"));
    }
    fn bind(node: &mut N, names: &std::collections::BTreeMap<String, String>) {
        match node {
            N::Element {
                name,
                attributes,
                children,
                ..
            } => {
                if let Some(new) = names.get(name) {
                    *name = new.clone()
                }
                attributes.retain(|a| {
                    !a.name.starts_with("http://www.w3.org/2000/xmlns/:")
                        && a.name != "xmlns"
                        && !a.name.starts_with("xmlns:")
                });
                for a in attributes {
                    if let Some(new) = names.get(&a.name) {
                        a.name = new.clone()
                    }
                    for v in &mut a.value_nodes {
                        bind(v, names)
                    }
                }
                for child in children {
                    bind(child, names)
                }
            }
            N::Document { children, .. } => {
                for child in children {
                    bind(child, names)
                }
            }
            _ => {}
        }
    }
    let mut original = projection::ast_stream(ast, Some(&document.content_type));
    original.retain_non_directive_nodes();
    let mut nodes = original.as_nodes().to_vec();
    for node in &mut nodes {
        bind(node, &names);
    }
    if target == LayerFormat::Cem {
        let mut directives = vec![N::Element {
            name: "@doc".into(),
            attributes: vec![],
            children: vec![N::Text {
                value: "cem-ml 1".into(),
                source: Default::default(),
            }],
            source: Default::default(),
        }];
        for (uri, prefix) in &bindings {
            directives.push(N::Element {
                name: "@ns".into(),
                attributes: vec![],
                children: vec![N::Text {
                    value: format!("{prefix} = {uri:?}"),
                    source: Default::default(),
                }],
                source: Default::default(),
            });
        }
        directives.extend(nodes);
        nodes = directives;
    } else {
        for node in &mut nodes {
            if let N::Element { attributes, .. } = node {
                for (uri, prefix) in &bindings {
                    if prefix != "xml" {
                        attributes.push(A {
                            name: format!("xmlns:{prefix}"),
                            value: Some(uri.clone()),
                            value_nodes: vec![],
                            source: Default::default(),
                        })
                    }
                }
            }
        }
    }
    Ok(projection::CemTreeAstStream::new(nodes))
}
