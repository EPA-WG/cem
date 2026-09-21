use cem_ql::eval::{AtomValue, Item, ItemStream};
use cem_ql::render::{render_template, TemplateData};

fn source() -> &'static str {
    "{cem-data @name=data @type=xml @select=source}"
}

fn data() -> TemplateData {
    TemplateData::default().with_binding(
        "source",
        ItemStream::once(Item::Atomic(AtomValue::String(
            "<root><name>ivy<em>saur</em></name><id>2</id></root>".into(),
        ))),
    )
}

fn rendered(body: &str) -> String {
    let body = body
        .lines()
        .map(str::trim)
        .map(|line| {
            if line.starts_with('@') {
                format!(" {line}")
            } else {
                line.to_owned()
            }
        })
        .collect::<String>();
    let result = render_template(&format!("{}{}", source(), body), &data());
    assert!(result.diagnostics.is_empty(), "{:?}", result.diagnostics);
    result.rendered
}

#[test]
fn explicit_text_and_implicit_template_focus() {
    assert_eq!(
        rendered(
            r#"{template @match='node.name == "name"' |
        {span | {$dom:text()}}}
        {apply-templates @select=data.root.children.children}"#
        ),
        "<span>ivysaur</span>"
    );
    assert_eq!(rendered("{$dom:text(data.root)}"), "ivysaur2");
}

#[test]
fn body_reuses_nodes_while_attributes_project_text() {
    assert_eq!(
        rendered(
            r#"{div @title="{$data.root.children.children}" |
        {$data.root.children.children}}"#
        ),
        "<div title=\"ivysaur2\"><name>ivy<em>saur</em></name><id>2</id></div>"
    );
}

#[test]
fn short_hook_receives_the_whole_sequence_and_restores_scope() {
    assert_eq!(
        rendered(
            r#"{div |
        {template @on=expression @into=content | {$dom:text()}}
        {$data.root.children.children}}
        {aside | {$data.root.children.children}}"#
        ),
        "<div>ivysaur2</div><aside><name>ivy<em>saur</em></name><id>2</id></aside>"
    );
}

#[test]
fn empty_hook_focus_does_not_fall_back_to_template_node() {
    assert_eq!(
        rendered(
            r#"{template @match='node.name == "name"' |
        {template @on=expression @into=content | {$dom:text()}}
        {span | {$node.missing}}}
        {apply-templates @select=data.root.children.children}"#
        ),
        "<span></span>"
    );
}

#[test]
fn named_calls_inherit_focus_without_rebinding_from_a_parameter_name() {
    assert_eq!(
        rendered(
            r#"{template @name=label | {span | {$dom:text()}}}
        {template @match='node.name == "name"' |
            {call @template=label @with:node="different"}}
        {apply-templates @select=data.root.children.children}"#
        ),
        "<span>ivysaur</span>"
    );
}

#[test]
fn missing_focus_is_an_error() {
    let result = render_template("{$dom:text()}", &TemplateData::default());
    assert!(
        result
            .diagnostics
            .iter()
            .any(|d| d.code == "cem.ql.context_missing"),
        "{:?}",
        result.diagnostics
    );
}

#[test]
fn attribute_hooks_preserve_literals_and_convert_then_validate() {
    assert_eq!(
        rendered(
            r#"{template @on=expression @into=attribute
        @match='context.attribute.name == "count"' @returns=integer | {$value}}
        {output @count='{"002"}' @title="name: {$data.root.children.children}"}"#
        ),
        "<output count=\"2\" title=\"name: ivysaur2\"></output>"
    );
}

#[test]
fn attribute_contracts_cover_number_date_and_regex() {
    assert_eq!(
        rendered(
            r#"{output |
        {attribute @name=count @type=integer @minInclusive=1 @value='{"002"}'}
        {attribute @name=day @type=date @value='{"2024-02-29"}'}
        {attribute @name=code @type=string @pattern="[A-Z]{2}-[0-9]+" @value='{"AB-12"}'}}"#
        ),
        "<output count=\"2\" day=\"2024-02-29\" code=\"AB-12\"></output>"
    );
    for (contract, value) in [
        ("@type=integer @minInclusive=1", "0"),
        ("@type=date", "2023-02-29"),
        ("@type=string @pattern='[A-Z]+'", "abc"),
    ] {
        let result = render_template(
            &format!("{{output | {{attribute @name=value {contract} @value='{value}'}}}}"),
            &TemplateData::default(),
        );
        assert!(
            result
                .diagnostics
                .iter()
                .any(|d| d.severity == cem_ml::diagnostics::Severity::Error),
            "{contract}: {:?}",
            result.diagnostics
        );
        assert!(
            result.rendered.is_empty(),
            "invalid typed output is never published"
        );
    }
}

#[test]
fn explicit_reference_clone_and_element_have_distinct_semantics() {
    assert_eq!(rendered(r#"{cem:variable @name=n @select=seq:first(data.root.children.children)}
        {span | {$dom:reference((n, n))}}
        {$dom:clone(n)}{$dom:element(n)}"#),
        "<span><name>ivy<em>saur</em></name><name>ivy<em>saur</em></name></span><name>ivy<em>saur</em></name><name></name>");
}

#[test]
fn reference_targets_keep_identity_and_clone_gets_independent_identity() {
    assert_eq!(
        rendered(
            r#"{cem:variable @name=n @select=seq:first(data.root.children.children)}
        {cem:variable @name=r @select=dom:reference(n)}
        {cem:variable @name=c @select=dom:clone(n)}
        {$r.targets.id == n.id}|{$c.id == n.id}|{$seq:count(dom:parent(c))}|{$dom:text(c)}"#
        ),
        "true|false|0|ivysaur"
    );
}

#[test]
fn source_text_occurrences_do_not_repeat_semantically_coalesced_neighbours() {
    let data = TemplateData::default().with_binding(
        "source",
        ItemStream::once(Item::Atomic(AtomValue::String(
            "<a>one<![CDATA[<two>]]>&amp;three</a>".into(),
        ))),
    );
    let rendered = render_template(
        &format!("{}{{$data.root.children.children}}", source()),
        &data,
    );
    assert!(
        rendered.diagnostics.is_empty(),
        "{:?}",
        rendered.diagnostics
    );
    assert_eq!(rendered.rendered, "one&lt;two&gt;&amp;three");
}

#[test]
fn clone_preserves_document_and_attribute_kinds() {
    assert_eq!(
        rendered(
            r#"{cem:variable @name=copy @select=dom:clone(data.root)}{$copy.kind}|{$seq:count(dom:parent(copy))}|{$dom:text(copy)}"#
        ),
        "document|0|ivysaur2"
    );
    assert_eq!(
        rendered(
            r#"{cem:variable @name=a @select='data:read("<a id=\"2\"/>", "xml").root.children.attributes'}{cem:variable @name=copy @select=dom:clone(a)}{$copy.kind}|{$seq:count(dom:parent(copy))}|{$dom:text(copy)}"#
        ),
        "attribute|0|2"
    );
}
