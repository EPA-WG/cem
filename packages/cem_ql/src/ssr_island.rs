//! First-connection canonical island preparation over retained CEM owners.
use crate::instance_island::construct::{add_child, attribute, element, PREFIXES};
use crate::{
    eval::{Item, RetainedCemNode},
    render::TemplateData,
    ssr_preparation::PreparedInlineDce,
};
use cem_ml::{diagnostics::Diagnostic, operation_control::OperationControl};
#[derive(Debug)]
pub struct PreparedInlineIsland {
    prepared: PreparedInlineDce,
    template: Item,
    root: Item,
    payload: Vec<RetainedCemNode>,
    envelope: Option<RetainedCemNode>,
    data: TemplateData,
    instance_id: String,
    pub(crate) css: Option<crate::ssr_css::InstanceCss>,
    pub(crate) coordinated_css:
        Option<std::sync::Arc<crate::ssr_stylesheets::CoordinatedStylesheets>>,
}
impl PreparedInlineIsland {
    pub fn prepared(&self) -> &PreparedInlineDce {
        &self.prepared
    }
    pub fn template(&self) -> &Item {
        &self.template
    }
    pub fn context_root(&self) -> &Item {
        &self.root
    }
    pub fn payload(&self) -> &[RetainedCemNode] {
        &self.payload
    }
    pub fn payload_envelope(&self) -> Option<&RetainedCemNode> {
        self.envelope.as_ref()
    }
    pub fn bindings(&self) -> &TemplateData {
        &self.data
    }
    pub fn declaration_stylesheets(
        &self,
    ) -> &[std::sync::Arc<crate::retained_template::RetainedStylesheet>] {
        self.coordinated_css
            .as_ref()
            .map_or(&[], |css| css.declaration_records())
    }
    pub fn instance_stylesheets(
        &self,
    ) -> &[std::sync::Arc<crate::retained_template::RetainedStylesheet>] {
        self.coordinated_css
            .as_ref()
            .map(|css| css.instance_records())
            .unwrap_or_else(|| self.css.as_ref().map_or(&[], |css| css.records()))
    }
    pub fn instance_id(&self) -> &str {
        &self.instance_id
    }
}
pub fn prepare_inline_island(
    prepared: PreparedInlineDce,
    instance_id: &str,
    control: &OperationControl,
) -> Result<PreparedInlineIsland, Vec<Diagnostic>> {
    use crate::ssr_preparation::{check, problem};
    use cem_ml::parser::tree::CemTreeNodeKind;
    let page = prepared.page();
    let tree = &prepared.elements().tree;
    let instance = prepared.instance_node();
    let fail = |code, message| problem(page, Some(tree), Some(instance), code, message);
    check(page, control)?;
    if prepared
        .lifecycle_control
        .as_ref()
        .is_some_and(|original| !original.same_operation(control))
    {
        return Err(fail(
            "cem.ssr.participant_operation_stale",
            "Participant preparation belongs to the original operation",
        ));
    }
    if prepared
        .stylesheet_control
        .as_ref()
        .is_some_and(|c| !c.same_operation(control))
    {
        return Err(fail(
            "cem.ssr.css_closure_stale",
            "Stylesheet preparation belongs to the original operation",
        ));
    }
    if instance_id.is_empty()
        || instance_id.len() > 256
        || instance_id.chars().all(char::is_whitespace)
        || instance_id.chars().any(char::is_control)
    {
        return Err(fail(
            "cem.ssr.instance_identity_invalid",
            "A bounded explicit instance identity is required",
        ));
    }
    if let Some(cpu) = &prepared.lifecycle_cpu {
        cpu.usage_ms(control).map_err(|_| {
            fail(
                "cem.ssr.cpu_budget",
                "Original native CPU budget refused island preparation",
            )
        })?;
    }
    let source = RetainedCemNode::new(tree.clone(), instance).unwrap();
    let child_ids = tree.node(instance).unwrap().children.clone();
    let templates: Vec<_> = child_ids
        .iter()
        .copied()
        .filter(|id| is_html(tree, *id, "template"))
        .collect();
    let meaningful = child_ids
        .iter()
        .filter(|id| {
            let n = tree.node(**id).unwrap();
            !(n.kind == CemTreeNodeKind::Text && n.value.chars().all(payload_space)
                || n.kind == CemTreeNodeKind::Comment
                    && ["cem-render-start", "cem-render-end"].contains(&n.value.as_str()))
        })
        .count();
    if !templates.is_empty() && (templates.len() != 1 || meaningful != 1) {
        return Err(fail(
            "cem-element.instance_payload_mixed",
            "An unmarked payload template must be the sole meaningful direct child",
        ));
    }
    let envelope = templates
        .first()
        .copied()
        .map(|id| RetainedCemNode::new(tree.clone(), id).unwrap());
    let mut cem_source = false;
    let payload_ids = if let Some(envelope) = &envelope {
        let node = tree.node(envelope.node_id()).unwrap();
        let attrs = node
            .attributes
            .iter()
            .filter_map(|id| {
                let a = tree.node(*id).unwrap();
                (a.name.as_ref().unwrap().namespace_uri != "http://www.w3.org/2000/xmlns/")
                    .then_some((&a.name.as_ref().unwrap().local_name, a.value.as_str()))
            })
            .collect::<std::collections::BTreeMap<_, _>>();
        cem_source = attrs.get(&"type".to_owned()).is_some_and(|value| {
            cem_ml::import::element_tree::canonical_cem_payload_type(value).is_some()
        });
        for (name, value) in attrs {
            let accepted = if cem_source {
                name == "type"
                    || name == "data-cem-schema"
                        && value == cem_ml::schema::registry::CEM_ML_SCHEMA_URI
            } else {
                name == "type" && value == cem_ml::schema::registry::HTML_CONTENT_TYPE
            };
            if !accepted {
                return Err(fail(
                    "cem.ssr.payload_profile_unsupported",
                    "Payload envelope requires an admitted canonical HTML or CEM-ML source profile",
                ));
            }
        }
        if cem_source {
            vec![envelope.node_id()]
        } else {
            node.children.clone()
        }
    } else {
        child_ids
    };
    // Preserve native source IDs and owners, never clone/serialize their subtrees.
    let payload: Vec<_> = payload_ids
        .into_iter()
        .map(|id| RetainedCemNode::new(tree.clone(), id).unwrap())
        .collect();
    let mut pending = payload
        .iter()
        .map(|node| node.node_id())
        .collect::<Vec<_>>();
    let mut work = 0usize;
    while let Some(id) = pending.pop() {
        check(page, control)?;
        work += 1;
        if work > crate::instance_island::LIMITS.max_values {
            return Err(fail(
                "cem.ssr.island_bounds",
                "Payload exceeds the native island work limit",
            ));
        }
        let node = tree.node(id).unwrap();
        pending.extend(node.children.iter().copied());
    }
    let css = if prepared.stylesheet_coordination {
        Ok(None)
    } else {
        crate::ssr_css::InstanceCss::prepare(
            tree,
            &payload,
            envelope.is_some() && !cem_source,
            instance_id,
            &page.route().document_base_url,
            page.policy(),
            control,
        )
    };
    check(page, control)?;
    let css = css.map_err(|_| {
        fail(
            "cem.ssr.css_closure_refused",
            "Native instance stylesheet closure admission refused",
        )
    })?;
    let native_source = if cem_source {
        let envelope = envelope.as_ref().unwrap();
        let admitted = if let Some(pool) = &prepared.payload_sources {
            pool.retain(
                tree.clone(),
                envelope.node_id(),
                control,
                &crate::instance_island::LIMITS,
            )
        } else {
            cem_ml::import::element_tree::retain_inline_cem_payload(
                tree.clone(),
                envelope.node_id(),
                control,
                &crate::instance_island::LIMITS,
            )
        };
        check(page, control)?;
        Some(admitted.map_err(|_| {
            problem(
                page,
                Some(tree),
                Some(envelope.node_id()),
                "cem.ssr.payload_source_invalid",
                "Native CEM-ML source admission refused the payload",
            )
        })?)
    } else {
        None
    };
    let host = tree.node(instance).unwrap();
    if host
        .attributes
        .iter()
        .filter(|id| {
            tree.node(**id)
                .unwrap()
                .name
                .as_ref()
                .unwrap()
                .namespace_uri
                != "http://www.w3.org/2000/xmlns/"
        })
        .count()
        > 64
    {
        return Err(fail(
            "cem.ssr.island_bounds",
            "Host attributes exceed the native island alias ceiling",
        ));
    }
    let origin = tree.node_source_map(instance).unwrap_or_default();
    let mut parts = crate::instance_island::PARTS
        .iter()
        .enumerate()
        .map(|(index, (name, ns))| {
            element(name, ns, Some(PREFIXES[index + 1]), vec![], vec![], &origin)
        })
        .collect::<Vec<_>>();
    for (name, value) in [
        ("version", crate::instance_island::VERSION),
        ("instanceId", instance_id),
        ("outputTarget", "light-dom"),
    ] {
        let text = crate::render::RenderPlanNode::Text {
            text: value.into(),
            source_map: origin.clone(),
        };
        add_child(
            &mut parts[0],
            element(
                "field",
                crate::instance_island::PARTS[0].1,
                Some(PREFIXES[1]),
                vec![
                    attribute("name", name, &origin),
                    attribute("kind", "string", &origin),
                ],
                vec![text],
                &origin,
            ),
        );
    }
    for id in &host.attributes {
        let attr = tree.node(*id).unwrap();
        if attr.name.as_ref().unwrap().namespace_uri == "http://www.w3.org/2000/xmlns/" {
            continue;
        }
        let name = &attr.name.as_ref().unwrap().local_name;
        if name == "data-cem-render-scope" {
            continue;
        }
        let map = tree.node_source_map(*id).unwrap_or_default();
        add_child(
            &mut parts[1],
            element(
                "attribute",
                crate::instance_island::PARTS[1].1,
                Some(PREFIXES[2]),
                vec![
                    attribute("name", name, &map),
                    attribute("value", &attr.value, &map),
                ],
                vec![],
                &map,
            ),
        );
        if let Some(suffix) = name.strip_prefix("data-") {
            add_child(
                &mut parts[2],
                element(
                    "entry",
                    crate::instance_island::PARTS[2].1,
                    Some(PREFIXES[3]),
                    vec![
                        attribute("key", &crate::instance_island::dataset_key(suffix), &map),
                        attribute("value", &attr.value, &map),
                    ],
                    vec![],
                    &map,
                ),
            );
        }
    }
    if let crate::render::RenderPlanNode::Element {
        attributes,
        children,
        ..
    } = &mut parts[3]
    {
        attributes.extend([
            attribute(
                "content-type",
                if cem_source {
                    "text/cem-ml"
                } else {
                    cem_ml::schema::registry::HTML_CONTENT_TYPE
                },
                &origin,
            ),
            attribute(
                "schema",
                if cem_source {
                    cem_ml::schema::registry::CEM_ML_SCHEMA_URI
                } else {
                    cem_ml::schema::registry::HTML_SCHEMA_URI
                },
                &origin,
            ),
        ]);
        if envelope.is_some() {
            attributes.push(attribute("data-cem-explicit-payload", "true", &origin));
        }
        let canonical_payload = native_source.as_ref().map(|source| {
            vec![RetainedCemNode::new(
                source.canonical_tree().clone(),
                source.canonical_template_node(),
            )
            .unwrap()]
        });
        let projected_payload = canonical_payload.as_deref().unwrap_or(&payload);
        children.extend(projected_payload.iter().map(|node| {
            crate::render::RenderPlanNode::Reference {
                reference: cem_ml::value::CemReference::new(vec![node.query_item()]),
                source_map: node
                    .owner()
                    .node_source_map(node.node_id())
                    .unwrap_or_default(),
            }
        }));
    }
    if let Some(version) = &prepared.declaration_version {
        add_child(
            &mut parts[0],
            element(
                "field",
                crate::instance_island::PARTS[0].1,
                Some(PREFIXES[1]),
                vec![
                    attribute("name", "declarationVersion", &origin),
                    attribute("kind", "string", &origin),
                ],
                vec![crate::render::RenderPlanNode::Text {
                    text: version.clone(),
                    source_map: origin.clone(),
                }],
                &origin,
            ),
        );
    }
    let mut root_attrs = vec![attribute(
        "version",
        crate::instance_island::VERSION,
        &origin,
    )];
    for (prefix, ns) in PREFIXES.iter().zip(
        std::iter::once(crate::instance_island::NAMESPACE)
            .chain(crate::instance_island::PARTS.iter().map(|(_, ns)| *ns)),
    ) {
        let mut attr = attribute(prefix, ns, &origin);
        attr.namespace = Some("http://www.w3.org/2000/xmlns/".into());
        attr.qualified_name = Some(format!("xmlns:{prefix}"));
        root_attrs.push(attr);
    }
    let root = element(
        "context-root",
        crate::instance_island::NAMESPACE,
        Some(PREFIXES[0]),
        root_attrs,
        parts,
        &origin,
    );
    let template = element(
        "template",
        HTML,
        None,
        vec![attribute("data-cem-island", "instance", &origin)],
        vec![root],
        &origin,
    );
    let mut values = crate::eval::output::output_nodes_with_source(vec![template], source).items;
    let template = values.remove(0);
    let root = template
        .view()
        .unwrap()
        .field("children")
        .unwrap()
        .remove(0);
    check(page, control)?;
    let bound = crate::instance_island::bind_retained_root(
        &root,
        instance_id,
        control,
        native_source.as_ref(),
    );
    check(page, control)?;
    let data = bound.map_err(|error| {
        if error == crate::instance_island::BOUNDS {
            fail(
                "cem.ssr.island_bounds",
                "Canonical native island exceeds its value bounds",
            )
        } else {
            fail(
                if cem_source {
                    "cem.ssr.payload_source_invalid"
                } else {
                    "cem.ssr.island_admission_refused"
                },
                "Shared native island/domain admission refused the captured state",
            )
        }
    })?;
    Ok(PreparedInlineIsland {
        prepared,
        template,
        root,
        payload,
        envelope,
        data,
        instance_id: instance_id.into(),
        css,
        coordinated_css: None,
    })
}
const HTML: &str = cem_ml::schema::registry::HTML_NAMESPACE_URI;
fn is_html(tree: &cem_ml::parser::tree::RetainedCemTree, id: u32, name: &str) -> bool {
    tree.node(id)
        .and_then(|node| node.name.as_ref())
        .is_some_and(|n| n.namespace_uri == HTML && n.local_name == name)
}
// Match browser String.trim for first-connection shape analysis, including BOM
// and NBSP; U+0085 is meaningful and must not be removed by Rust's broader trim.
fn payload_space(c: char) -> bool {
    matches!(c, '\u{0009}'..='\u{000d}' | '\u{0020}' | '\u{00a0}' | '\u{1680}' | '\u{2000}'..='\u{200a}' | '\u{2028}' | '\u{2029}' | '\u{202f}' | '\u{205f}' | '\u{3000}' | '\u{feff}')
}
