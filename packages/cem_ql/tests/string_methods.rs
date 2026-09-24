use cem_ml::scheduler::ScopePolicy;
use cem_ql::api::{compile, evaluate, CompileContext, EvaluationContext};
use cem_ql::eval::{AtomValue, Item, ItemStream};
use cem_ql::render::{render_template, TemplateData};
use cem_ql::stdlib::{ModuleRegistry, StdlibImplKind, Tier};

fn eval(source: &str) -> Vec<Item> {
    let query = compile(source, &CompileContext::default()).expect(source);
    let result = evaluate(
        &query,
        &EvaluationContext {
            scope_policy: ScopePolicy::host_root().with_queue_size(128),
            ..EvaluationContext::default()
        },
    );
    assert!(result.error.is_none(), "{source}: {:?}", result.diagnostics);
    result.items
}

fn strings(values: &[&str]) -> Vec<Item> {
    values
        .iter()
        .map(|value| Item::Atomic(AtomValue::String((*value).into())))
        .collect()
}

#[test]
fn simple_string_methods_are_native_tier_a_with_exact_arities() {
    let registry = ModuleRegistry::tier_a();
    for (name, min, max) in [
        ("split", 2, 2),
        ("trim", 1, 2),
        ("trim_start", 1, 2),
        ("trim_end", 1, 2),
        ("char_at", 2, 2),
        ("at", 2, 2),
        ("index_of", 2, 3),
        ("last_index_of", 2, 3),
    ] {
        for arity in min..=max {
            let function = registry
                .resolve("cem:stdlib/strings", name, arity)
                .expect(name);
            assert_eq!(function.tier, Tier::A);
            assert_eq!(function.implementation, StdlibImplKind::Native);
        }
        assert!(registry
            .resolve("cem:stdlib/strings", name, min - 1)
            .is_none());
        assert!(registry
            .resolve("cem:stdlib/strings", name, max + 1)
            .is_none());
    }
}

#[test]
fn split_is_literal_preserves_empty_fields_and_emits_a_sequence() {
    for (source, expected) in [
        (
            r#"str:split("🍒,🍋,,🍌,", ",")"#,
            vec!["🍒", "🍋", "", "🍌", ""],
        ),
        (
            r#"str:split("::🍒::::🍋::", "::")"#,
            vec!["", "🍒", "", "🍋", ""],
        ),
        (r#"str:split("a.b.*c", ".")"#, vec!["a", "b", "*c"]),
        (r#"str:split("aaa", "aa")"#, vec!["", "a"]),
        (r#"str:split("plain", ",")"#, vec!["plain"]),
        (r#"str:split("", ",")"#, vec![""]),
        (r#"str:split("", "")"#, vec!["", ""]),
        (r#"str:split("a🍒é", "")"#, vec!["", "a", "🍒", "e", "\u{301}", ""]),
        (r#"str:split("🍒🍋🍒", "🍒")"#, vec!["", "🍋", ""]),
    ] {
        assert_eq!(eval(source), strings(&expected), "{source}");
    }
    assert_eq!(
        eval(r#"str:concat(str:split("a,,b,", ","), "|")"#),
        strings(&["a||b|"])
    );
    assert_eq!(
        eval(r#"seq:flat_map(("a,b", "c,d"), fn(s) => str:split(s, ","))"#),
        strings(&["a", "b", "c", "d"])
    );
}

#[test]
fn trim_methods_remove_only_edge_ecmascript_whitespace() {
    for (source, expected) in [
        (r#"str:trim("  🍒  🍋  ")"#, "🍒  🍋"),
        (r#"str:trim_start("  🍒  ")"#, "🍒  "),
        (r#"str:trim_end("  🍒  ")"#, "  🍒"),
        (r#"str:trim("")"#, ""),
        ("str:trim(\"\t\n\r\u{a0}\u{2003}\u{2028}\u{feff}\")", ""),
        ("str:trim(\"\u{feff}🍒\u{a0}\")", "🍒"),
        ("str:trim(\"\u{85}🍒\u{85}\")", "\u{85}🍒\u{85}"),
        ("str:trim(\"\u{200b}🍒\u{200b}\")", "\u{200b}🍒\u{200b}"),
    ] {
        assert_eq!(eval(source), strings(&[expected]), "{source}");
    }
}

#[test]
fn character_access_uses_codepoints_and_handles_extreme_bounds() {
    for (source, expected) in [
        (r#"str:char_at("a🍒b", 1)"#, vec!["🍒"]),
        (r#"str:char_at("a🍒b", -1)"#, vec![""]),
        (r#"str:char_at("a🍒b", 3)"#, vec![""]),
        (r#"str:char_at("", 0)"#, vec![""]),
        (r#"str:char_at("a", 9223372036854775807)"#, vec![""]),
        (r#"str:at("a🍒b", 1)"#, vec!["🍒"]),
        (r#"str:at("a🍒b", -1)"#, vec!["b"]),
        (r#"str:at("a🍒b", -3)"#, vec!["a"]),
        (r#"str:at("a🍒b", -4)"#, vec![]),
        (r#"str:at("a🍒b", 3)"#, vec![]),
        (r#"str:at("", 0)"#, vec![]),
        (r#"str:at("a", -9223372036854775807)"#, vec![]),
        (r#"str:at("a", 9223372036854775807)"#, vec![]),
        (r#"str:at("a", 9) ?? "missing""#, vec!["missing"]),
    ] {
        assert_eq!(eval(source), strings(&expected), "{source}");
    }
}

#[test]
fn search_indices_use_codepoints_clamp_positions_and_find_overlapping_matches() {
    for (source, expected) in [
        (r#"str:index_of("🍒a🍋a", "a")"#, 1),
        (r#"str:index_of("🍒a🍋a", "a", 2)"#, 3),
        (r#"str:index_of("abc", "a", -9)"#, 0),
        (r#"str:index_of("abc", "a", 99)"#, -1),
        (r#"str:index_of("abc", "")"#, 0),
        (r#"str:index_of("🍒a", "", 99)"#, 2),
        (r#"str:index_of("", "")"#, 0),
        (r#"str:index_of("", "a")"#, -1),
        (r#"str:index_of("aaa", "aa", 1)"#, 1),
        (r#"str:index_of("a🍒b", "🍒", 9223372036854775807)"#, -1),
        (r#"str:last_index_of("🍒a🍋a", "a")"#, 3),
        (r#"str:last_index_of("🍒a🍋a", "a", 2)"#, 1),
        (r#"str:last_index_of("🍒a🍋a", "a", 3)"#, 3),
        (r#"str:last_index_of("aaa", "aa", 1)"#, 1),
        (r#"str:last_index_of("aaaa", "aa", 1)"#, 1),
        (r#"str:last_index_of("abcabc", "abc", 4)"#, 3),
        (r#"str:last_index_of("a🍒a", "a", -9)"#, 0),
        (r#"str:last_index_of("a🍒a", "🍒", -9)"#, -1),
        (r#"str:last_index_of("a🍒b", "b", 1)"#, -1),
        (r#"str:last_index_of("a🍒b", "🍒", 9223372036854775807)"#, 1),
        (r#"str:last_index_of("🍒a", "")"#, 2),
        (r#"str:last_index_of("abc", "", 1)"#, 1),
        (r#"str:last_index_of("", "")"#, 0),
        (r#"str:last_index_of("", "a")"#, -1),
    ] {
        assert_eq!(
            eval(source),
            vec![Item::Atomic(AtomValue::Integer(expected))],
            "{source}"
        );
    }
}

#[test]
fn split_composes_with_filter_count_and_native_template_loops() {
    for (text, expected) in [
        ("", 0),
        (" \t\n\u{a0}\u{2003} ", 0),
        (" one  one\ttwo\n🍒 ", 4),
    ] {
        let data =
            TemplateData::default().with_binding("text", ItemStream::from_items(strings(&[text])));
        let result = render_template(
            r#"{output | {$seq:count(seq:where(str:split(str:normalize_space(text), " "), fn(word) => word != ""))}}"#,
            &data,
        );
        assert!(result.diagnostics.is_empty(), "{:?}", result.diagnostics);
        assert_eq!(result.rendered, format!("<output>{expected}</output>"));
    }
    let result = render_template(
        r#"{ul | {cem:for-each @select='str:split("🍒,🍋", ",")' @as=fruit | {li | {$fruit}}}}"#,
        &TemplateData::default(),
    );
    assert!(result.diagnostics.is_empty(), "{:?}", result.diagnostics);
    assert_eq!(result.rendered, "<ul><li>🍒</li><li>🍋</li></ul>");
}

// CEMQL-VIEWER-FUNCTION-AUDIT: opt in to XPath/XML whitespace semantics.
#[test]
fn xml_whitespace_profile_preserves_nonbreaking_space_without_changing_defaults() {
    assert_eq!(eval("str:trim(\" \t 🍒 \n\", \"xml\")"), strings(&[" 🍒 "]));
    assert_eq!(eval("str:trim(\" 🍒 \")"), strings(&["🍒"]));
    assert_eq!(eval("str:normalize_space(\"  A\t  B\n  \" , \"xml\")"), strings(&["A   B"]));
    assert_eq!(eval("str:normalize_space(\"  A\t  B\n  \")"), strings(&["A B"]));
    let query = compile(r#"str:trim("a", "guess")"#, &CompileContext::default()).unwrap();
    assert!(evaluate(&query, &EvaluationContext::default()).error.is_some());
}

#[test]
fn authored_url_split_chain_renders_natively() {
    let page = include_str!("../../cem-elements/demo/functions/str.html");
    let sample = page.split_once("legend=\"URL ID with a string chain\"").unwrap().1;
    let template = sample.split_once("<template type=\"text/cem-ml\">").unwrap().1.split_once("</template>").unwrap().0;
    let output = render_template(template, &TemplateData::default());
    assert!(output.diagnostics.is_empty(), "{:?}", output.diagnostics);
    assert!(output.rendered.contains("<output>1</output>"), "{}", output.rendered);
}
