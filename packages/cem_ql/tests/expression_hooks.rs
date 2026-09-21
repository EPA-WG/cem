//! CEMT-HOOK-FOCUS: expression hooks retain sequence, lexical and focus boundaries.
use cem_ql::render::{render_template, TemplateData};

fn render(source: &str) -> String {
    let source = source.lines().map(str::trim).collect::<String>();
    let result = render_template(&source, &TemplateData::default());
    assert!(result.diagnostics.is_empty(), "{:?}", result.diagnostics);
    result.rendered
}

#[test]
fn attribute_hooks_receive_each_expression_sequence_and_leave_literals_alone() {
    assert_eq!(
        render(
            r#"
        {template @on=expression @into=attribute |
            {$context.attribute.name + ":" + dom:text(seq:count(value)) + ":" + dom:text()}}
        {p @title='pre-{(1, 2)}-mid-{()}-post-{3}' @lang=en}
    "#
        ),
        "<p title=\"pre-title:2:12-mid-title:0:-post-title:1:3\" lang=\"en\"></p>"
    );
}

#[test]
fn hook_scope_captures_each_iteration_and_does_not_escape_the_loop() {
    assert_eq!(
        render(
            r#"
        {cem:variable @name=label @select='"outside"'}
        {cem:for-each @select='("one", "two")' @as=label |
            {template @on=expression @into=content | {$label + ":" + dom:text()}}
            {cem:variable @name=label @select='"changed"'}
            {p | {$label}}}
        {p | {$label}}
    "#
        ),
        "<p>one:changed</p><p>two:changed</p><p>outside</p>"
    );
}

#[test]
fn nested_query_focus_restores_the_complete_hook_input() {
    assert_eq!(
        render(
            r#"
        {template @on=expression @into=content |
            {$dom:text(value.where(dom:text() == "ivy")) + ":" + dom:text()}}
        {p | {$("ivy", "saur")}}
    "#
        ),
        "<p>ivy:ivysaur</p>"
    );
}

#[test]
fn active_hook_is_skipped_for_nested_content_and_outer_behavior_can_apply() {
    assert_eq!(
        render(
            r#"
        {template @on=expression @into=content | {$"outer(" + dom:text() + ")"}}
        {section |
            {template @on=expression @into=content | {b | {$value}}}
            {$"first"}{$"second"}}
        {aside | {$"last"}}
    "#
        ),
        "<section><b>outer(first)</b><b>outer(second)</b></section><aside>outer(last)</aside>"
    );
}

#[test]
fn nested_attribute_hook_restores_content_context_and_reserved_bindings() {
    assert_eq!(render(r#"
        {cem:variable @name=value @select='"caller-value"'}
        {cem:variable @name=context @select='"caller-context"'}
        {section |
            {template @on=expression @into=attribute | {$context.into + ":" + context.attribute.name + ":" + dom:text()}}
            {template @on=expression @into=content |
                {b @title="{$value}" | {$value}}
                {$context.into + ":" + dom:text()}}
            {$"ivy"}}
        {aside | {$value}|{$context}}
    "#), "<section><b title=\"attribute:title:ivy\">ivy</b>content:ivy</section><aside>caller-value|caller-context</aside>");
}
