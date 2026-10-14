use super::*;
fn bound_fixture(
    conflict: bool,
    island: bool,
) -> (Fixture, Arc<RecordedBodyStylesheetInstallations>) {
    let hosts = ["cem-card", "cem-inner"].iter().map(|tag| {
        let marker = if conflict { "{result-attribute @name=data-cem-css-context | wrong}" } else { "" };
        format!("{{result-element @name={tag} @namespace='http://www.w3.org/1999/xhtml' | {marker} {{result-element @name=template @namespace='http://www.w3.org/1999/xhtml' | {{result-attribute @name=data-cem-island | instance}}}}}}")
    }).collect::<String>();
    let mut root = ROOT_SOURCE.replace("{body | {call", &format!("{{body | {hosts} {{call"));
    if !island {
        root = root.replace(
            "@name=data-cem-island | instance",
            "@name=data-cem-island | wrong",
        );
    }
    let mut f = fixture(&root, &CHILD_SOURCE.replace(" layer(base)", ""), false);
    let css = Arc::new(capture(&mut f, css_sources(), Default::default()).unwrap());
    let choices = f
        .execution
        .bodies()
        .bodies
        .iter()
        .map(|b| BodyStylesheetInstallation {
            body_uri: b.uri.clone(),
            index: 0,
            scope: cem_ml::css_emission::CssManagedScope::Private {
                tag: if b.uri == ROOT {
                    "cem-card"
                } else {
                    "cem-inner"
                }
                .into(),
                context: None,
            },
        })
        .collect::<Vec<_>>();
    let compiled = css.compile_installations(&choices, 100000).unwrap();
    (f, compiled)
}
fn bindings(f: &Fixture) -> Vec<Vec<Vec<usize>>> {
    f.execution
        .bodies()
        .bodies
        .iter()
        .map(|b| vec![vec![0, if b.uri == ROOT { 1 } else { 2 }]])
        .collect()
}
#[test]
fn context_bindings_are_native_and_preserve_original_source_output() {
    let (f, compiled) = bound_fixture(false, true);
    let before = f.execution.output().rendered.clone();
    let output = compiled
        .compose_output_with_hosts(
            &[element(f.execution.tree(), "head"); 2],
            &bindings(&f),
            10000,
            100000,
        )
        .unwrap_or_else(|e| panic!("{e}; output={before}; bindings={:?}", bindings(&f)));
    assert_eq!(before, f.execution.output().rendered);
    assert!(!before.contains("data-cem-css-context"));
    for tag in ["cem-card", "cem-inner"] {
        let host = output.tree().node(element(output.tree(), tag)).unwrap();
        assert!(host
            .attributes
            .iter()
            .filter_map(|id| output.tree().node(*id))
            .any(|a| a
                .name
                .as_ref()
                .is_some_and(|n| n.local_name == "data-cem-css-context")
                && a.value.starts_with("cem-context-")));
    }
    assert!(!output.output().output_spans.is_empty());
}
#[test]
fn context_bindings_reject_missing_wrong_or_conflicting_hosts() {
    let (f, compiled) = bound_fixture(false, true);
    let head = [element(f.execution.tree(), "head"); 2];
    assert!(compiled
        .compose_output_with_hosts(&head, &[], 10000, 100000)
        .is_err());
    assert!(compiled
        .compose_output_with_hosts(&head, &[vec![], vec![vec![0, 1, 1]]], 10000, 100000)
        .is_err());
    assert!(compiled
        .compose_output_with_hosts(
            &head,
            &bindings(&f).into_iter().rev().collect::<Vec<_>>(),
            10000,
            100000
        )
        .is_err());
    let (f, compiled) = bound_fixture(true, true);
    assert!(compiled
        .compose_output_with_hosts(
            &[element(f.execution.tree(), "head"); 2],
            &bindings(&f),
            10000,
            100000
        )
        .is_err());
}

#[test]
fn contextual_binding_requires_a_produced_instance_island() {
    let (f, compiled) = bound_fixture(false, false);
    assert!(compiled
        .compose_output_with_hosts(
            &[element(f.execution.tree(), "head"); 2],
            &bindings(&f),
            10000,
            100000
        )
        .is_err());
}
