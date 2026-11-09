//! Fresh native admission of a captured canonical SSR host. Serialized readiness
//! flags and hashes confer no authority: compatible output is recomputed locally.
use crate::{
    eval::Item,
    instance_island as island,
    render::{
        compile_template, project_render_plan_with_control, render_compiled_template_with_control,
        CompileTemplateOptions, RenderPlanAttribute, RenderPlanNode,
    },
};
use cem_ml::operation_control::{OperationControl, ROOT_EXECUTION_SCOPE_ID};
use cem_ml::schema::registry::HTML_NAMESPACE_URI;
use std::collections::{BTreeMap, BTreeSet};
#[path = "resume_stylesheets.rs"]
mod stylesheets;

pub const PROFILE: &str = "cem-native-inline/1";
pub const PROFILE_ATTRIBUTE: &str = "data-cem-ssr-profile";
#[derive(Debug, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum ResumeDecision {
    Adopt,
    RenderOnLoad,
}
#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NativeSsrResume {
    pub decision: ResumeDecision,
    pub instance_id: String,
    pub declaration_version: Option<String>,
    pub source_key: String,
    pub instance_stylesheets: bool,
}

#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NestedDeclaration {
    pub tag: String,
    pub version: String,
    pub source: String,
}

#[derive(serde::Serialize)]
pub struct NativeSsrGroup {
    pub participants: Vec<NativeSsrResume>,
}

struct Group<'a> {
    declarations: BTreeMap<&'a str, (&'a str, &'a str)>,
    declaration_nodes: BTreeMap<String, &'a Item>,
    registry: BTreeSet<String>,
    ids: BTreeSet<String>,
    participants: Vec<NativeSsrResume>,
    remaining_source_bytes: usize,
}

struct ParentInput<'a> {
    attributes: BTreeMap<String, String>,
    payload: &'a [RenderPlanNode],
}
pub fn versions_compatible(a: &str, b: &str) -> bool {
    if !cem_ml::schema::document_model::is_canonical_semver(a)
        || !cem_ml::schema::document_model::is_canonical_semver(b)
    {
        return false;
    }
    fn parts(value: &str) -> (&str, Option<&str>) {
        let value = value.split('+').next().unwrap();
        value
            .split_once('-')
            .map_or((value, None), |(a, b)| (a, Some(b)))
    }
    let (a, ap) = parts(a);
    let (b, bp) = parts(b);
    if ap.is_some() || bp.is_some() {
        return a == b && ap == bp;
    }
    let a = a.split('.').collect::<Vec<_>>();
    let b = b.split('.').collect::<Vec<_>>();
    a[0] == b[0] && (a[0] != "0" || a[1] == b[1] && (a[1] != "0" || a[2] == b[2]))
}
fn fail() -> String {
    "cem.ssr.resume_refused: canonical island, host, output or passive profile is invalid".into()
}
fn text(i: &Item) -> Result<String, String> {
    island::field(i, "value").ok_or_else(fail)
}
fn kind(i: &Item) -> Option<String> {
    island::field(i, "kind")
}
fn whitespace(i: &Item) -> bool {
    matches!(kind(i).as_deref(), Some("text" | "whitespace"))
        && text(i).is_ok_and(|s| s.trim().is_empty())
}
fn marker(i: &Item, name: &str) -> bool {
    kind(i).as_deref() == Some("comment") && text(i).as_deref() == Ok(name)
}
pub fn admit(
    host: &Item,
    tag: &str,
    current_version: &str,
    source: &str,
    c: &OperationControl,
) -> Result<NativeSsrResume, String> {
    admit_group(host, tag, current_version, source, &[], c)
        .map(|mut group| group.participants.remove(0))
}

pub fn admit_group(
    host: &Item,
    tag: &str,
    current_version: &str,
    source: &str,
    nested: &[NestedDeclaration],
    c: &OperationControl,
) -> Result<NativeSsrGroup, String> {
    admit_group_with_styles(host, tag, current_version, source, nested, &[], c)
}

pub fn admit_group_with_styles(
    host: &Item,
    tag: &str,
    current_version: &str,
    source: &str,
    nested: &[NestedDeclaration],
    declaration_nodes: &[Item],
    c: &OperationControl,
) -> Result<NativeSsrGroup, String> {
    let mut source_bytes = source.len();
    if nested.len() >= 64
        || declaration_nodes.len() > 64
        || tag.len() > 256
        || current_version.len() > 1024
    {
        return Err(fail());
    }
    let mut group = Group {
        declarations: BTreeMap::new(),
        declaration_nodes: BTreeMap::new(),
        registry: BTreeSet::new(),
        ids: BTreeSet::new(),
        participants: vec![],
        remaining_source_bytes: 1024 * 1024,
    };
    for declaration in nested {
        source_bytes = source_bytes.saturating_add(declaration.source.len());
        if !declaration.tag.contains('-')
            || declaration.tag == "cem-element"
            || declaration.tag.len() > 256
            || declaration.version.len() > 1024
            || group
                .declarations
                .insert(
                    &declaration.tag,
                    (&declaration.version, &declaration.source),
                )
                .is_some()
        {
            return Err(fail());
        }
        group.registry.insert(declaration.tag.clone());
    }
    if source_bytes > 1024 * 1024 {
        return Err(fail());
    }
    for declaration in declaration_nodes {
        island::bounded_root(declaration, c)?;
        let declared = island::attrs(declaration)?.remove("tag").ok_or_else(fail)?;
        if !island::is_element(declaration, "cem-element", HTML_NAMESPACE_URI)
            || (declared != tag && !group.registry.contains(&declared))
            || group
                .declaration_nodes
                .insert(declared, declaration)
                .is_some()
        {
            return Err(fail());
        }
    }
    island::bounded_root(host, c)?;
    group.admit_member(host, tag, current_version, source, None, 0, c)?;
    Ok(NativeSsrGroup {
        participants: group.participants,
    })
}

impl Group<'_> {
    fn admit_member(
        &mut self,
        host: &Item,
        tag: &str,
        current_version: &str,
        source: &str,
        parent: Option<ParentInput<'_>>,
        depth: usize,
        c: &OperationControl,
    ) -> Result<(), String> {
        c.check_scope(ROOT_EXECUTION_SCOPE_ID).map_err(|_| fail())?;
        self.remaining_source_bytes = self
            .remaining_source_bytes
            .checked_sub(source.len())
            .ok_or_else(fail)?;
        if self.participants.len() >= 64
            || depth > 64
            || !island::is_element(host, tag, HTML_NAMESPACE_URI)
        {
            return Err(fail());
        }
        let source_key = host
            .view()
            .and_then(|view| view.provenance())
            .and_then(|source| source.source_key)
            .ok_or_else(fail)?;
        let children = island::children(host)?;
        let templates = children
            .iter()
            .enumerate()
            .filter(|(_, i)| island::is_element(i, "template", HTML_NAMESPACE_URI))
            .collect::<Vec<_>>();
        let [(ti, template)] = templates.as_slice() else {
            return Err(fail());
        };
        if !children[..*ti].iter().all(whitespace)
            || island::attrs(template)?
                != BTreeMap::from([
                    ("data-cem-island".into(), "instance".into()),
                    (PROFILE_ATTRIBUTE.into(), PROFILE.into()),
                ])
        {
            return Err(fail());
        }
        let roots = island::elements(template)?;
        let [root] = roots.as_slice() else {
            return Err(fail());
        };
        let parts = island::elements(root)?;
        let hydration = parts.first().ok_or_else(fail)?;
        let fields = island::hydration_fields(hydration)?;
        let instance_id = fields
            .get("instanceId")
            .filter(|s| !s.trim().is_empty() && s.len() <= 256 && !s.chars().any(char::is_control))
            .ok_or_else(fail)?
            .clone();
        if !self.ids.insert(instance_id.clone()) {
            return Err(fail());
        }
        let data = island::bind_root(root, &instance_id, c, None, true)?;
        let payload_attrs = island::attrs(&parts[3])?;
        if payload_attrs.get("content-type").map(String::as_str) != Some("text/html")
            || parts[4..]
                .iter()
                .any(|p| !island::elements(p).is_ok_and(|v| v.is_empty()))
        {
            return Err(fail());
        }
        let mut saved = BTreeMap::new();
        for a in island::elements(&parts[1])? {
            let a = island::attrs(&a)?;
            saved.insert(
                a.get("name").ok_or_else(fail)?.clone(),
                a.get("value").ok_or_else(fail)?.clone(),
            );
        }
        let mut actual = island::attrs(host)?;
        let host_attrs = actual
            .iter()
            .map(|(k, v)| (k.as_str(), v.as_str()))
            .collect::<Vec<_>>();
        cem_ml::validation::html::validate_passive_html_attributes(tag, &host_attrs)
            .map_err(|_| fail())?;
        actual.remove("data-cem-render-scope");
        if actual != saved {
            return Err(fail());
        }
        let payload = island::children(&parts[3])?;
        let explicit = payload_attrs
            .get("data-cem-explicit-payload")
            .is_some_and(|value| value == "true");
        let mut payload_styles = stylesheets::PayloadStyles::default();
        let declaration = self.declaration_nodes.get(tag).copied();
        let shared_scope = declaration
            .map(stylesheets::declaration_scope)
            .transpose()?
            .flatten();
        if let Some(scope) = &shared_scope {
            if actual.get("scope") != Some(scope) {
                return Err(fail());
            }
        }
        if actual.contains_key("data-cem-css-context") {
            return Err(fail());
        }
        passive_nodes(
            &payload,
            c,
            (explicit && declaration.is_some()).then_some(&mut payload_styles),
        )?;
        if let Some(mut parent) = parent {
            if !parent.attributes.contains_key("scope") {
                if let Some(scope) = &shared_scope {
                    parent.attributes.insert("scope".into(), scope.clone());
                }
            }
            if parent.attributes != actual {
                return Err(fail());
            }
            let mut expected_payload = parent.payload;
            if explicit {
                let [RenderPlanNode::Element {
                    tag,
                    attributes,
                    children,
                    ..
                }] = expected_payload
                else {
                    return Err(fail());
                };
                let attributes = plan_attributes(attributes)?;
                if tag != "template"
                    || !(attributes.is_empty()
                        || attributes == BTreeMap::from([("type".into(), "text/html".into())]))
                {
                    return Err(fail());
                }
                expected_payload = children;
            }
            compare(expected_payload, &payload, depth, c, None, explicit)?;
        }
        let starts = children
            .iter()
            .enumerate()
            .filter(|(_, i)| marker(i, "cem-render-start"))
            .map(|(i, _)| i)
            .collect::<Vec<_>>();
        let ends = children
            .iter()
            .enumerate()
            .filter(|(_, i)| marker(i, "cem-render-end"))
            .map(|(i, _)| i)
            .collect::<Vec<_>>();
        let ([start], [end]) = (starts.as_slice(), ends.as_slice()) else {
            return Err(fail());
        };
        if start <= ti || end <= start || !children[*ti + 1..*start].iter().all(whitespace) {
            return Err(fail());
        }
        let output = &children[*start + 1..*end];
        let instance_stylesheets = payload_styles.verify(&instance_id, &children[*end + 1..], c)?;
        let artifact = compile_template(
            source,
            &CompileTemplateOptions {
                host_bindings: data.bindings.keys().cloned().collect(),
                source_uri: Some("cem-resume:declaration".into()),
                ..Default::default()
            },
        );
        if artifact.module_map.is_some()
            || artifact.requires_legacy_runtime_resources()
            || artifact
                .diagnostics
                .iter()
                .any(|d| d.severity.is_hard_violation())
        {
            return Err(fail());
        }
        stylesheets::verify_declaration(declaration, tag, shared_scope.as_deref(), &artifact, c)?;
        let styled = instance_stylesheets || !artifact.stylesheets.is_empty();
        let version = fields.get("declarationVersion").cloned();
        let plan =
            render_compiled_template_with_control(&artifact, &data, c, ROOT_EXECUTION_SCOPE_ID);
        if plan
            .diagnostics
            .iter()
            .any(|d| d.severity.is_hard_violation())
        {
            return Err(fail());
        }
        let mut plan = project_render_plan_with_control(
            &plan,
            crate::eval::QueryContextScope(0),
            &island::LIMITS,
            c,
            ROOT_EXECUTION_SCOPE_ID,
        )
        .map_err(|_| fail())?;
        crate::ssr_css::strip_sources(&mut plan.nodes, &payload_styles.maps);
        crate::ssr_render::admit_output_mode(&mut plan.nodes, false, Some(&self.registry))
            .map_err(|_| fail())?;
        let mut pending: Vec<_> = plan.nodes.iter().collect();
        while let Some(node) = pending.pop() {
            if let RenderPlanNode::Element {
                tag,
                namespace,
                children,
                ..
            } = node
            {
                if (!profile_tag(tag) && !self.registry.contains(tag))
                    || namespace.as_deref() != Some(HTML_NAMESPACE_URI)
                {
                    return Err(fail());
                }
                if !self.registry.contains(tag) {
                    pending.extend(children);
                }
            }
        }
        if !version
            .as_deref()
            .is_some_and(|v| versions_compatible(v, current_version))
        {
            if styled || !self.registry.is_empty() {
                return Err(fail());
            }
            passive_nodes(output, c, None)?;
            self.participants.push(NativeSsrResume {
                decision: ResumeDecision::RenderOnLoad,
                instance_id,
                declaration_version: version,
                source_key,
                instance_stylesheets,
            });
            return Ok(());
        }
        for a in &plan.host_attribute_updates {
            if actual.get(&a.name) != Some(&a.value) {
                return Err(fail());
            }
        }
        if !plan
            .nodes
            .iter()
            .any(|n| matches!(n, RenderPlanNode::Element { .. }))
        {
            return Err(fail());
        }
        self.participants.push(NativeSsrResume {
            decision: ResumeDecision::Adopt,
            instance_id,
            declaration_version: version,
            source_key,
            instance_stylesheets,
        });
        compare(&plan.nodes, output, depth, c, Some(self), false)
    }
}

fn plan_attributes(attributes: &[RenderPlanAttribute]) -> Result<BTreeMap<String, String>, String> {
    let mut result = BTreeMap::new();
    for attribute in attributes {
        if attribute.namespace.as_deref() == Some("http://www.w3.org/2000/xmlns/") {
            continue;
        }
        if attribute
            .namespace
            .as_deref()
            .is_some_and(|s| !s.is_empty())
            || result
                .insert(attribute.name.clone(), attribute.value.clone())
                .is_some()
        {
            return Err(fail());
        }
    }
    Ok(result)
}
fn compare(
    expected: &[RenderPlanNode],
    actual: &[Item],
    depth: usize,
    c: &OperationControl,
    mut group: Option<&mut Group<'_>>,
    allow_style: bool,
) -> Result<(), String> {
    c.check_scope(ROOT_EXECUTION_SCOPE_ID).map_err(|_| fail())?;
    if depth > 64 {
        return Err(fail());
    }
    let (mut e, mut a) = (0, 0);
    while e < expected.len() || a < actual.len() {
        let mut et = String::new();
        while let Some(RenderPlanNode::Text { text, .. }) = expected.get(e) {
            et.push_str(text);
            e += 1;
        }
        let mut at = String::new();
        while actual
            .get(a)
            .is_some_and(|i| matches!(kind(i).as_deref(), Some("text" | "whitespace")))
        {
            at.push_str(&text(&actual[a])?);
            a += 1;
        }
        if et != at {
            return Err(fail());
        }
        if e == expected.len() && a == actual.len() {
            break;
        }
        let item = actual.get(a).ok_or_else(fail)?;
        match expected.get(e).ok_or_else(fail)? {
            RenderPlanNode::Element {
                tag,
                namespace,
                attributes,
                children,
                ..
            } => {
                if !island::is_element(
                    item,
                    tag,
                    namespace
                        .as_deref()
                        .filter(|s| !s.is_empty())
                        .unwrap_or(HTML_NAMESPACE_URI),
                ) {
                    return Err(fail());
                }
                let attributes = plan_attributes(attributes)?;
                if tag.contains('-') {
                    let group = group.as_deref_mut().ok_or_else(fail)?;
                    let (version, source) =
                        *group.declarations.get(tag.as_str()).ok_or_else(fail)?;
                    let input = ParentInput {
                        attributes,
                        payload: children,
                    };
                    group.admit_member(item, tag, version, source, Some(input), depth + 1, c)?;
                } else {
                    if !profile_tag(tag)
                        || (tag == "style" && !allow_style)
                        || attributes != island::attrs(item)?
                    {
                        return Err(fail());
                    }
                    compare(
                        children,
                        &island::children(item)?,
                        depth + 1,
                        c,
                        group.as_deref_mut(),
                        allow_style,
                    )?;
                }
            }
            RenderPlanNode::Comment { text: expected, .. }
                if kind(item).as_deref() == Some("comment")
                    && text(item).as_deref() == Ok(expected.as_str()) => {}
            _ => return Err(fail()),
        }
        e += 1;
        a += 1;
    }
    Ok(())
}

fn passive_nodes(
    roots: &[Item],
    c: &OperationControl,
    mut styles: Option<&mut stylesheets::PayloadStyles>,
) -> Result<(), String> {
    let mut pending = roots.iter().rev().cloned().collect::<Vec<_>>();
    while let Some(node) = pending.pop() {
        c.check_scope(ROOT_EXECUTION_SCOPE_ID).map_err(|_| fail())?;
        match kind(&node).as_deref() {
            Some("element") => {
                let tag = island::field(&node, "name").ok_or_else(fail)?;
                if island::is_element(&node, "style", HTML_NAMESPACE_URI) {
                    styles.as_deref_mut().ok_or_else(fail)?.push(&node)?;
                    continue;
                }
                // Inert nested templates have a separate DOM mutation root.
                if !profile_tag(&tag) || !island::is_element(&node, &tag, HTML_NAMESPACE_URI) {
                    return Err(fail());
                }
                let attributes = island::attrs(&node)?;
                let pairs = attributes
                    .iter()
                    .map(|(k, v)| (k.as_str(), v.as_str()))
                    .collect::<Vec<_>>();
                cem_ml::validation::html::validate_passive_html_projection(&tag, &pairs)
                    .map_err(|_| fail())?;
                pending.extend(island::children(&node)?.into_iter().rev());
            }
            Some("text" | "whitespace" | "comment") => {}
            _ => return Err(fail()),
        }
    }
    Ok(())
}

fn profile_tag(tag: &str) -> bool {
    !tag.contains('-')
        && !matches!(
            tag,
            "template"
                | "form"
                | "input"
                | "textarea"
                | "select"
                | "option"
                | "fieldset"
                | "button"
        )
}
