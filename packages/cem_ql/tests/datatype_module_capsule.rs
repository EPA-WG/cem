use cem_ml::{
    content_cache::ContentHash,
    events::cem::CemEventNormalizer,
    operation_control::OperationControl,
    parser::{AstNodeId, CemAstNode},
    schema::{
        machine::{CemSchemaMachine, LexicallyScopedDocument},
        structural_capsule::SourceIdentity,
        vocab::CompiledSchema,
    },
    source::{BytesSource, SourceId},
    tokenizer::cem::CemTokenizer,
};
use cem_ql::datatype_module_capsule::{self as modules, ModuleSource, ModuleTarget};
use std::{
    collections::{BTreeMap, BTreeSet},
    sync::Arc,
};
fn hash(s: &[u8]) -> String {
    ContentHash::from_blake3(s).header_value()
}
fn capture(text: &str) -> Arc<LexicallyScopedDocument> {
    Arc::new(
        CemSchemaMachine::new(
            CompiledSchema::cem_core(),
            CemEventNormalizer::new(CemTokenizer::from_source(BytesSource::new(
                SourceId(1),
                text.as_bytes().to_vec(),
            ))),
        )
        .build_with_lexical_scopes(),
    )
}
fn datatype(c: &LexicallyScopedDocument, name: &str) -> AstNodeId {
    c.document().nodes.iter().find_map(|n|match n {CemAstNode::Element{node_id,expanded_name,attributes,..} if expanded_name.local_name=="type" && attributes.iter().any(|id|matches!(c.document().get(*id),Some(CemAstNode::Attribute{expanded_name,value:Some(n),..}) if expanded_name.local_name=="name" && n==name))=>Some(*node_id),_=>None}).unwrap()
}
fn field(c: &LexicallyScopedDocument, name: &str, attr: &str) -> AstNodeId {
    let CemAstNode::Element { attributes, .. } = c.document().get(datatype(c, name)).unwrap()
    else {
        unreachable!()
    };
    *attributes.iter().find(|id|matches!(c.document().get(**id),Some(CemAstNode::Attribute{expanded_name,..}) if expanded_name.local_name==attr)).unwrap()
}
fn source(uri: &str, text: &str, root: &str) -> ModuleSource {
    let c = capture(text);
    let roots = BTreeSet::from([datatype(&c, root)]);
    ModuleSource {
        identity: SourceIdentity::new(uri, SourceId(1), text.as_bytes()),
        capture: c,
        roots,
        bindings: BTreeMap::new(),
    }
}
fn leaf() -> ModuleSource {
    source("file:///app/types/core.cem","@ns s = https://cem.dev/ns/schema/1\n@default s\n{schema @namespace=urn:core | {types | {type @name=scalar @kind=scalar}}}","scalar")
}
fn derived(id: &str, target: &str, target_root: AstNodeId) -> ModuleSource {
    let text=format!("@ns s = https://cem.dev/ns/schema/1\n@ns v = urn:{target}\n@default s\n{{schema @namespace=urn:{id} | {{types | {{type @name=value @base={{#selected}}}}}}}}");
    let mut m = source(&format!("file:///app/types/{id}.cem"), &text, "value");
    m.bindings.insert(
        field(&m.capture, "value", "base"),
        ModuleTarget::ModuleDatatype {
            module: target.into(),
            root: target_root,
        },
    );
    m
}
fn chain() -> BTreeMap<String, ModuleSource> {
    let leaf = leaf();
    let vendor = derived("vendor", "core", *leaf.roots.first().unwrap());
    let app = derived("app", "vendor", *vendor.roots.first().unwrap());
    BTreeMap::from([
        ("core".into(), leaf),
        ("vendor".into(), vendor),
        ("app".into(), app),
    ])
}
#[test]
fn module_chain_roundtrips_with_original_sources_and_no_runtime_authority() {
    let input = chain();
    let control = OperationControl::default();
    let built = modules::export("app", &input, &[], &hash(b"registry"), &control).unwrap();
    assert!(built.bytes.starts_with(modules::MAGIC));
    assert!(!built
        .bytes
        .starts_with(cem_ql::datatype_dependency_capsule::MAGIC));
    let a = modules::verify(&built.bytes, &built.pin, &[], &control).unwrap();
    let b = modules::verify(&built.bytes, &built.pin, &[], &control).unwrap();
    assert_eq!(
        a.module_order(),
        &["core".to_owned(), "vendor".to_owned(), "app".to_owned()]
    );
    assert_eq!(a.datatype_order().len(), 3);
    assert!(!a.selections_verified());
    assert!(!a.executable_closure_verified());
    for (id, original) in &input {
        let m = a.module(id).unwrap();
        assert_eq!(m.source().source_uri(), original.identity.uri);
        for node in original.capture.occurrences() {
            assert_eq!(
                rmp_serde::to_vec_named(&m.capture().snapshot(m.capture().document(), node))
                    .unwrap(),
                rmp_serde::to_vec_named(
                    &original.capture.snapshot(original.capture.document(), node)
                )
                .unwrap()
            );
        }
        assert!(!Arc::ptr_eq(
            m.capture().document(),
            b.module(id).unwrap().capture().document()
        ));
        assert_eq!(m.roots(), &original.roots);
    }
    assert_eq!(
        built.bytes,
        modules::export("app", &input, &[], &hash(b"registry"), &control)
            .unwrap()
            .bytes
    );
}

use cem_ql::{datatype_dependency_capsule as capsule, datatype_source_capsule as dt};
#[path = "support/datatype_rule_artifacts.rs"]
mod rule_artifacts;
fn identity(text: &str, uri: &str) -> SourceIdentity {
    SourceIdentity::new(uri, SourceId(1), text.as_bytes())
}
#[test]
fn shared_diamond_uses_one_original_source_per_module_id() {
    let core = leaf();
    let left = derived("left", "core", *core.roots.first().unwrap());
    let right = derived("right", "core", *core.roots.first().unwrap());
    let mut app=source("file:///app/diamond.cem","@ns s = https://cem.dev/ns/schema/1\n@default s\n{schema @namespace=urn:app | {types | {type @name=first @base={#left}} {type @name=second @base={#right}}}}","first");
    app.roots.insert(datatype(&app.capture, "second"));
    app.bindings.insert(
        field(&app.capture, "first", "base"),
        ModuleTarget::ModuleDatatype {
            module: "left".into(),
            root: *left.roots.first().unwrap(),
        },
    );
    app.bindings.insert(
        field(&app.capture, "second", "base"),
        ModuleTarget::ModuleDatatype {
            module: "right".into(),
            root: *right.roots.first().unwrap(),
        },
    );
    let input = BTreeMap::from([
        ("app".into(), app),
        ("core".into(), core),
        ("left".into(), left),
        ("right".into(), right),
    ]);
    let b = modules::export(
        "app",
        &input,
        &[],
        &hash(b"registry"),
        &OperationControl::default(),
    )
    .unwrap();
    let v = modules::verify(&b.bytes, &b.pin, &[], &OperationControl::default()).unwrap();
    assert_eq!(v.module_order(), &["core", "left", "right", "app"]);
    assert_eq!(v.datatype_order().len(), 5);
    assert_eq!(v.module_ids().count(), 4);
    assert_eq!(v.module("core").unwrap().sources().len(), 1);
}
#[test]
fn cross_module_and_self_cycles_report_original_source_fields() {
    for self_cycle in [false, true] {
        let mut input = chain();
        if self_cycle {
            let app = input.get_mut("app").unwrap();
            let id = *app.roots.first().unwrap();
            *app.bindings.values_mut().next().unwrap() = ModuleTarget::ModuleDatatype {
                module: "app".into(),
                root: id,
            };
        } else {
            let core = derived("core", "vendor", *input["vendor"].roots.first().unwrap());
            let id = *core.roots.first().unwrap();
            input.insert("core".into(), core);
            *input
                .get_mut("vendor")
                .unwrap()
                .bindings
                .values_mut()
                .next()
                .unwrap() = ModuleTarget::ModuleDatatype {
                module: "core".into(),
                root: id,
            };
        }
        let error = modules::export(
            "app",
            &input,
            &[],
            &hash(b"registry"),
            &OperationControl::default(),
        )
        .err()
        .unwrap();
        assert_eq!(error.code, "datatype-module-cycle");
        assert_eq!(error.cycle_modules.first(), error.cycle_modules.last());
        assert_eq!(error.related.len() + 1, error.cycle_modules.len());
        for (i, node) in error.related.iter().enumerate() {
            let original = &input[&error.cycle_modules[i]];
            assert_eq!(node.node_id(), *original.bindings.keys().next().unwrap());
            assert!(
                matches!(node.node(),CemAstNode::Attribute{expanded_name,..} if expanded_name.local_name=="base")
            );
        }
    }
}
#[test]
fn missing_unexported_unused_and_wrong_role_modules_refuse() {
    for case in 0..7 {
        let mut input = chain();
        match case {
            0 => {
                input.remove("core");
            }
            1 => {
                *input
                    .get_mut("app")
                    .unwrap()
                    .bindings
                    .values_mut()
                    .next()
                    .unwrap() = ModuleTarget::ModuleDatatype {
                    module: "vendor".into(),
                    root: u32::MAX,
                };
            }
            2 => {
                let mut unused = leaf();
                unused.identity.uri = "file:///app/unused.cem".into();
                input.insert("unused".into(), unused);
            }
            3 => {
                input.get_mut("vendor").unwrap().roots.clear();
            }
            4 => {
                let app = input.remove("app").unwrap();
                input.insert("../app".into(), app);
            }
            5 => {
                input.get_mut("app").unwrap().bindings.clear();
            }
            _ => {
                input
                    .get_mut("app")
                    .unwrap()
                    .bindings
                    .insert(u32::MAX, ModuleTarget::LocalDatatype(0));
            }
        }
        assert!(
            modules::export(
                "app",
                &input,
                &[],
                &hash(b"registry"),
                &OperationControl::default()
            )
            .is_err(),
            "case {case}"
        );
    }
    let (target, artifact) = rule_artifacts::rule_artifact(false);
    let capsule::DependencyTarget::Native(pin) = target else {
        unreachable!()
    };
    let mut input = chain();
    *input
        .get_mut("app")
        .unwrap()
        .bindings
        .values_mut()
        .next()
        .unwrap() = ModuleTarget::Native(pin);
    assert!(
        modules::export(
            "app",
            &input,
            &[artifact],
            &hash(b"registry"),
            &OperationControl::default()
        )
        .is_err(),
        "a rule cannot satisfy a base field"
    );
}
#[test]
fn local_cycles_are_rejected_by_shared_original_datatype_inspection() {
    let mut m=source("file:///app/local.cem","@ns s = https://cem.dev/ns/schema/1\n@default s\n{schema @namespace=urn:app | {types | {type @name=a @base=b} {type @name=b @base=a}}}","a");
    m.bindings.insert(
        field(&m.capture, "a", "base"),
        ModuleTarget::LocalDatatype(datatype(&m.capture, "b")),
    );
    m.bindings.insert(
        field(&m.capture, "b", "base"),
        ModuleTarget::LocalDatatype(datatype(&m.capture, "a")),
    );
    let e = modules::export(
        "app",
        &BTreeMap::from([("app".into(), m)]),
        &[],
        &hash(b"registry"),
        &OperationControl::default(),
    )
    .err()
    .unwrap();
    assert_eq!(e.code, "datatype-module-source-refused");
    assert!(!e.related.is_empty());
}
#[test]
fn exact_query_and_native_leaf_pins_remain_inert_inside_dependency_bearing_modules() {
    for query in [false, true] {
        let (target, artifact) = rule_artifacts::rule_artifact(query);
        let mut core=source("file:///app/core.cem","@ns s = https://cem.dev/ns/schema/1\n@default s\n{schema @namespace=urn:core | {types | {type @name=scalar @kind=scalar @rule={#check}}}}","scalar");
        let target = match target {
            capsule::DependencyTarget::Query(p) => ModuleTarget::Query(p),
            capsule::DependencyTarget::Native(p) => ModuleTarget::Native(p),
            _ => unreachable!(),
        };
        core.bindings
            .insert(field(&core.capture, "scalar", "rule"), target);
        let app = derived("app", "core", *core.roots.first().unwrap());
        let input = BTreeMap::from([("app".into(), app), ("core".into(), core)]);
        let artifacts = [artifact];
        let b = modules::export(
            "app",
            &input,
            &artifacts,
            &hash(b"registry"),
            &OperationControl::default(),
        )
        .unwrap();
        let v =
            modules::verify(&b.bytes, &b.pin, &artifacts, &OperationControl::default()).unwrap();
        assert_eq!(v.leaf_ids().count(), 1);
        assert_eq!(v.module("core").unwrap().sites()[0].role, "validation-rule");
        assert!(!v.selections_verified());
        assert!(!v.executable_closure_verified());
        assert!(modules::verify(&b.bytes, &b.pin, &[], &OperationControl::default()).is_err());
    }
}
#[test]
fn complete_source_agreement_applies_to_modules_and_supplied_leaf_artifacts() {
    let mut input = chain();
    input.get_mut("core").unwrap().identity.uri = input["vendor"].identity.uri.clone();
    assert!(modules::export(
        "app",
        &input,
        &[],
        &hash(b"registry"),
        &OperationControl::default()
    )
    .is_err());
    let core = leaf();
    let root = *core.roots.first().unwrap();
    let leaf = dt::export(
        &core.capture,
        core.identity.clone(),
        &hash(b"registry"),
        core.roots.clone(),
        &OperationControl::default(),
    )
    .unwrap();
    let mut app = derived("app", "unused", root);
    *app.bindings.values_mut().next().unwrap() = ModuleTarget::Datatype {
        pin: leaf.pin.clone(),
        root,
    };
    app.identity.uri = core.identity.uri;
    assert!(modules::export(
        "app",
        &BTreeMap::from([("app".into(), app)]),
        &[capsule::DependencyArtifact::datatype(leaf)],
        &hash(b"registry"),
        &OperationControl::default()
    )
    .is_err());
}
#[test]
fn valid_dt1_leaves_use_shared_limits_and_cannot_be_replaced_by_repinned_trailing_bytes() {
    for corrupt in [false, true] {
        let core = leaf();
        let root = *core.roots.first().unwrap();
        let mut leaf = dt::export(
            &core.capture,
            core.identity.clone(),
            &hash(b"registry"),
            core.roots.clone(),
            &OperationControl::default(),
        )
        .unwrap();
        if corrupt {
            leaf.bytes.push(0);
            leaf.pin.hash = hash(&leaf.bytes);
        }
        let mut app = derived("app", "unused", root);
        *app.bindings.values_mut().next().unwrap() = ModuleTarget::Datatype {
            pin: leaf.pin.clone(),
            root,
        };
        let artifacts = [capsule::DependencyArtifact::datatype(leaf)];
        let result = modules::export(
            "app",
            &BTreeMap::from([("app".into(), app)]),
            &artifacts,
            &hash(b"registry"),
            &OperationControl::default(),
        );
        assert_eq!(result.is_ok(), !corrupt);
    }
}
#[test]
fn depth_is_bounded_across_modules_and_cancelled_work_never_returns_a_verified_graph() {
    for (count, valid) in [(5, true), (6, false)] {
        let mut input = BTreeMap::new();
        let mut previous = None;
        for i in 0..count {
            let name = format!("m{i}");
            let defs = (0..6)
                .map(|n| {
                    if n == 0 {
                        if i == 0 {
                            "{type @name=t0 @kind=scalar}".into()
                        } else {
                            "{type @name=t0 @base={#previous}}".into()
                        }
                    } else {
                        format!("{{type @name=t{n} @base=t{}}}", n - 1)
                    }
                })
                .collect::<Vec<_>>()
                .join(" ");
            let text=format!("@ns s = https://cem.dev/ns/schema/1\n@default s\n{{schema @namespace=urn:{name} | {{types | {defs}}}}}");
            let mut m = source(&format!("file:///app/{name}.cem"), &text, "t5");
            for n in 1..6 {
                m.bindings.insert(
                    field(&m.capture, &format!("t{n}"), "base"),
                    ModuleTarget::LocalDatatype(datatype(&m.capture, &format!("t{}", n - 1))),
                );
            }
            if let Some((module, root)) = previous {
                m.bindings.insert(
                    field(&m.capture, "t0", "base"),
                    ModuleTarget::ModuleDatatype { module, root },
                );
            }
            previous = Some((name.clone(), *m.roots.first().unwrap()));
            input.insert(name, m);
        }
        let result = modules::export(
            &format!("m{}", count - 1),
            &input,
            &[],
            &hash(b"registry"),
            &OperationControl::default(),
        );
        assert_eq!(result.is_ok(), valid);
        if !valid {
            assert_eq!(result.err().unwrap().code, "datatype-module-graph-depth");
        }
    }
    let input = chain();
    let control = OperationControl::default();
    let b = modules::export("app", &input, &[], &hash(b"registry"), &control).unwrap();
    control
        .cancel_root(Some("module fixture".into()), None)
        .unwrap();
    assert!(modules::verify(&b.bytes, &b.pin, &[], &control).is_err());
    assert!(modules::export("app", &input, &[], &hash(b"registry"), &control).is_err());
}

#[test]
fn identical_whole_sources_can_export_distinct_roots_without_merging_their_arenas() {
    let text="@ns s = https://cem.dev/ns/schema/1\n@default s\n{schema @namespace=urn:common | {types | {type @name=one @kind=scalar} {type @name=two @kind=scalar}}}";
    let one = source("file:///app/common.cem", text, "one");
    let two = source("file:///app/common.cem", text, "two");
    let text="@ns s = https://cem.dev/ns/schema/1\n@default s\n{schema @namespace=urn:app | {types | {type @name=a @base={#first}} {type @name=b @base={#second}}}}";
    let mut app = source("file:///app/main.cem", text, "a");
    app.roots.insert(datatype(&app.capture, "b"));
    app.bindings.insert(
        field(&app.capture, "a", "base"),
        ModuleTarget::ModuleDatatype {
            module: "one".into(),
            root: *one.roots.first().unwrap(),
        },
    );
    app.bindings.insert(
        field(&app.capture, "b", "base"),
        ModuleTarget::ModuleDatatype {
            module: "two".into(),
            root: *two.roots.first().unwrap(),
        },
    );
    let input = BTreeMap::from([
        ("app".into(), app),
        ("one".into(), one),
        ("two".into(), two),
    ]);
    let b = modules::export(
        "app",
        &input,
        &[],
        &hash(b"registry"),
        &OperationControl::default(),
    )
    .unwrap();
    let v = modules::verify(&b.bytes, &b.pin, &[], &OperationControl::default()).unwrap();
    let (a, b) = (v.module("one").unwrap(), v.module("two").unwrap());
    assert!(!Arc::ptr_eq(a.capture().document(), b.capture().document()));
    assert_ne!(a.roots(), b.roots());
}
#[test]
fn literal_imports_keep_qualified_names_and_original_alias_snapshots() {
    for (name, accepted) in [("v:scalar", true), ("scalar", false), ("v:other", false)] {
        let core = leaf();
        let mut app = derived("app", "core", *core.roots.first().unwrap());
        let text=format!("@ns s = https://cem.dev/ns/schema/1\n@ns v = urn:core\n@default s\n{{schema @namespace=urn:app | {{types | {{type @name=value @base={name}}}}}}}");
        let mut replacement = source("file:///app/app.cem", &text, "value");
        replacement.bindings.insert(
            field(&replacement.capture, "value", "base"),
            app.bindings.pop_first().unwrap().1,
        );
        let input = BTreeMap::from([("app".into(), replacement), ("core".into(), core)]);
        assert_eq!(
            modules::export(
                "app",
                &input,
                &[],
                &hash(b"registry"),
                &OperationControl::default()
            )
            .is_ok(),
            accepted
        );
    }
}
