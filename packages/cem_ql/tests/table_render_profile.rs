//! Opt-in stage attribution for the authored table; elapsed time is not a gate.
use cem_ml::import::import_data;
use cem_ql::{
    api::{compile, evaluate, CompileContext, EvaluationContext},
    eval::{AtomValue, Item, ItemStream},
    render::{
        compile_template, render_compiled_template, render_plan_to_html, CompileTemplateOptions,
        TemplateData,
    },
};
use std::{collections::BTreeMap, hint::black_box, time::Instant};

const VIEW: &str = include_str!("../../cem-elements/demo/data-table-view.cemt");
fn text(value: &str) -> Item {
    Item::Atomic(AtomValue::String(value.into()))
}
fn record(fields: Vec<(&str, Item)>) -> Item {
    Item::Record(
        fields
            .into_iter()
            .map(|(key, value)| (key.into(), vec![value]))
            .collect(),
    )
}
fn measure<T>(case: &str, stage: &str, mut operation: impl FnMut() -> T) -> T {
    let start = Instant::now();
    let mut value = black_box(operation());
    let first = start.elapsed().as_secs_f64() * 1000.0;
    let mut times = Vec::new();
    for _ in 0..5 {
        let start = Instant::now();
        let next = black_box(operation());
        times.push(start.elapsed().as_secs_f64() * 1000.0);
        value = next;
    }
    times.sort_by(f64::total_cmp);
    println!("{case}\t{stage}\tfirst_ms={first:.3}\twarm_median_ms={:.3}\twarm_min_ms={:.3}\twarm_max_ms={:.3}", times[2], times[0], times[4]);
    value
}
fn data(source: &str, format: &str, column: &str, mode: &str) -> TemplateData {
    TemplateData::default()
        .with_binding("format", ItemStream::once(text(format)))
        .with_binding(
            "datadom",
            ItemStream::once(record(vec![
                (
                    "payload",
                    record(vec![("nodes", record(vec![("text", text(source))]))]),
                ),
                (
                    "slices",
                    record(vec![("column", text(column)), ("mode", text(mode))]),
                ),
            ])),
        )
}
// Read the exact expressions from this controlled CEMT fixture, never from an
// imported external document. Guard the anchors so template edits cannot silently
// profile another expression.
fn expression(name: &str) -> &str {
    let anchor = format!("@name={name} @select='");
    assert_eq!(VIEW.matches(&anchor).count(), 1);
    VIEW.split_once(&anchor)
        .unwrap()
        .1
        .split_once('\'')
        .unwrap()
        .0
}
fn query(
    case: &str,
    stage: &str,
    source: &str,
    bindings: &BTreeMap<String, ItemStream>,
) -> ItemStream {
    let query = compile(
        source,
        &CompileContext {
            policy_bindings: bindings.clone(),
            ..Default::default()
        },
    )
    .unwrap();
    let context = EvaluationContext {
        policy_bindings: bindings.clone(),
        ..Default::default()
    };
    let output = measure(case, stage, || evaluate(&query, &context));
    assert!(output.error.is_none(), "{stage}: {:?}", output.diagnostics);
    output
}

#[test]
#[ignore = "profiling fixture: run with --release --ignored --nocapture"]
fn profile_authored_table_stages() {
    let full = measure("shared", "compile-view", || {
        compile_template(VIEW, &CompileTemplateOptions::default())
    });
    assert!(full.diagnostics.is_empty(), "{:?}", full.diagnostics);
    let import = "{cem-data @name=document @select=source @type=\"{$format}\"}";
    assert_eq!(VIEW.matches(import).count(), 1);
    let retained = compile_template(
        &VIEW.replace(import, "{cem:variable @name=document @select=retained}"),
        &CompileTemplateOptions {
            host_bindings: vec!["retained".into()],
            ..Default::default()
        },
    );
    assert!(
        retained.diagnostics.is_empty(),
        "{:?}",
        retained.diagnostics
    );
    // Same small cases as data_view_templates.rs; document content remains CEM.
    for (format, source, column) in [
        (
            "xml",
            "<r><row qty=\"10\">🍒</row><row qty=\"2\">🍋</row><row qty=\"3\">🍌</row></r>",
            "@qty",
        ),
        ("csv", "qty,fruit\n10,🍒\n2,🍋\n3,🍌", "qty"),
        (
            "yaml",
            "- qty: 10\n  fruit: 🍒\n- qty: 2\n  fruit: 🍋\n- qty: 3\n  fruit: 🍌",
            "qty",
        ),
        (
            "json",
            r#"[{"qty":10,"fruit":"🍒"},{"qty":2,"fruit":"🍋"},{"qty":3,"fruit":"🍌"}]"#,
            "qty",
        ),
    ] {
        let _owner = measure(format, "cem-import", || {
            import_data(source, format, "cem", "data:read/source").unwrap()
        });
        let mut bindings = BTreeMap::from([
            ("source".into(), ItemStream::once(text(source))),
            ("format".into(), ItemStream::once(text(format))),
        ]);
        let document = query(
            format,
            "reader-import",
            "data:read(source, format)",
            &bindings,
        );
        bindings.insert("document".into(), document.clone());
        let columns = query(format, "discover-columns", expression("columns"), &bindings);
        assert!(columns.items.contains(&text(column)));
        let rows = query(
            format,
            "select-rows",
            "seq:first(document.root.children).children",
            &bindings,
        );
        assert_eq!(rows.items.len(), 3);
        bindings.insert("rows".into(), rows);
        let grouped = query(
            format,
            "group-rows",
            "seq:group_by(rows, fn(row) => row.namespace + \"|\" + row.name)",
            &bindings,
        );
        assert_eq!(grouped.items.len(), 1);
        let records = query(format, "project-records", expression("records"), &bindings);
        assert_eq!(records.items.len(), 3);
        bindings.insert("records".into(), records);
        let headings = query(
            format,
            "discover-headings",
            expression("headings"),
            &bindings,
        );
        assert!(headings.items.contains(&text(column)));
        bindings.insert("sortColumn".into(), ItemStream::once(text(column)));
        bindings.insert("sortDirection".into(), ItemStream::once(text("ascending")));
        for mode in ["text", "number"] {
            bindings.insert("sortMode".into(), ItemStream::once(text(mode)));
            let sorted = query(
                format,
                &format!("sort-{mode}"),
                expression("ordered"),
                &bindings,
            );
            assert_eq!(sorted.items.len(), 3);
        }
        for (state, sort_column, mode) in [
            ("source", "", "text"),
            ("text", column, "text"),
            ("number", column, "number"),
        ] {
            let input = data(source, format, sort_column, mode);
            let plan = measure(format, &format!("authored-render-{state}"), || {
                render_compiled_template(&full, &input)
            });
            assert!(plan.diagnostics.is_empty(), "{:?}", plan.diagnostics);
            let retained_input = input.with_binding("retained", document.clone());
            let reused = measure(format, &format!("retained-render-{state}"), || {
                render_compiled_template(&retained, &retained_input)
            });
            assert!(reused.diagnostics.is_empty(), "{:?}", reused.diagnostics);
            let html = measure(format, &format!("html-export-{state}"), || {
                render_plan_to_html(&plan)
            });
            assert_eq!(html, render_plan_to_html(&reused));
            let table = html.split_once("<tbody>").unwrap().1;
            let order = if state == "number" {
                ['🍋', '🍌', '🍒']
            } else {
                ['🍒', '🍋', '🍌']
            };
            let positions: Vec<_> = order
                .iter()
                .map(|fruit| table.find(*fruit).unwrap())
                .collect();
            assert!(
                positions.windows(2).all(|pair| pair[0] < pair[1]),
                "{format}/{state}"
            );
        }
    }
}

#[test]
#[ignore = "profiling fixture: run with --release --ignored --nocapture"]
fn profile_unread_control_bindings() {
    // Synthetic host-control metadata, not an external-format document or AST.
    let island = Item::Record(
        (0..256)
            .map(|i| {
                (
                    format!("control-{i}"),
                    vec![record(vec![
                        ("name", text(&format!("control-{i}"))),
                        (
                            "attributes",
                            record(vec![
                                ("role", text("option")),
                                ("scope", text("fixture")),
                                ("value", text("number")),
                            ]),
                        ),
                        (
                            "state",
                            record(vec![("revision", text("3")), ("selected", text("false"))]),
                        ),
                    ])],
                )
            })
            .collect(),
    );
    let bindings = BTreeMap::from([
        ("island".into(), ItemStream::once(island.clone())),
        (
            "delta".into(),
            ItemStream::once(Item::Atomic(AtomValue::Integer(1))),
        ),
    ]);
    let context = EvaluationContext {
        policy_bindings: bindings.clone(),
        ..Default::default()
    };
    let query = compile(
        "seq:map((1, 2, 3), fn(n) => n + delta)",
        &CompileContext {
            policy_bindings: bindings,
            ..Default::default()
        },
    )
    .unwrap();
    assert_eq!(
        query.policy_bindings.len(),
        2,
        "current IR retains unused declarations"
    );
    let full = measure("control", "query-with-unread-binding", || {
        evaluate(&query, &context)
    });
    // Diagnostic candidate for this pure query only. Opaque/native callbacks
    // require an explicit dependency contract or a full-context fallback.
    let mut referenced = query.clone();
    let used: std::collections::HashSet<_> = query
        .tree
        .nodes
        .iter()
        .filter_map(|node| {
            if let cem_ql::ir::IrNode::LocalVar(id) = node {
                Some(*id)
            } else {
                None
            }
        })
        .collect();
    referenced.policy_bindings.retain(|id, _| used.contains(id));
    assert_eq!(referenced.policy_bindings.len(), 1);
    let narrowed = measure("control", "query-referenced-binding-only", || {
        evaluate(&referenced, &context)
    });
    assert!(full.error.is_none(), "{:?}", full.diagnostics);
    assert_eq!(full, narrowed);
    assert_eq!(
        full.items,
        [2, 3, 4].map(|n| Item::Atomic(AtomValue::Integer(n)))
    );
    let _copied = measure("control", "clone-evaluation-context", || context.clone());

    let compiled = compile_template(
        VIEW,
        &CompileTemplateOptions {
            host_bindings: vec!["island".into()],
            ..Default::default()
        },
    );
    assert!(
        compiled.diagnostics.is_empty(),
        "{:?}",
        compiled.diagnostics
    );
    let baseline = data(
        "<r><row qty='10'>🍒</row><row qty='2'>🍋</row><row qty='3'>🍌</row></r>",
        "xml",
        "@qty",
        "number",
    );
    let busy = baseline
        .clone()
        .with_binding("island", ItemStream::once(island));
    let ordinary = measure("control", "render-without-unread-control", || {
        render_compiled_template(&compiled, &baseline)
    });
    let loaded = measure("control", "render-with-unread-control", || {
        render_compiled_template(&compiled, &busy)
    });
    assert!(
        ordinary.diagnostics.is_empty(),
        "{:?}",
        ordinary.diagnostics
    );
    assert!(loaded.diagnostics.is_empty(), "{:?}", loaded.diagnostics);
    assert_eq!(render_plan_to_html(&ordinary), render_plan_to_html(&loaded));
}
