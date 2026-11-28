use cem_ml::{
    events::cem::CemEventNormalizer,
    import::import_json_bytes,
    parser::{builder::CemAstBuilder, CemAstNode},
    resource_command::*,
    source::{BytesSource, SourceId},
    tokenizer::cem::CemTokenizer,
};
use serde_json::json;
use std::{
    cell::{Cell, RefCell},
    sync::Arc,
};

const SAVE: &str = include_str!("../../cem-components/docs/examples/file-save-as.v1.cem");
const COPY: &str = "@doc cem-ml 1\n{manifest @version=1 | {command @id=editor.copy @revision=1 |}}";
fn catalog(source: &str) -> Result<CommandCatalog, &'static str> {
    let bytes = BytesSource::new(SourceId(1), source.as_bytes().to_vec());
    let doc = CemAstBuilder::new(CemEventNormalizer::new(CemTokenizer::from_source(bytes))).build();
    assert!(
        !doc.diagnostics
            .iter()
            .any(|d| d.severity.is_hard_violation()),
        "{:?}",
        doc.diagnostics
    );
    let id = doc
        .iter()
        .find_map(|n| match n {
            CemAstNode::Element {
                node_id,
                expanded_name,
                ..
            } if expanded_name.local_name == "manifest" => Some(*node_id),
            _ => None,
        })
        .unwrap();
    CommandCatalog::from_manifest(&doc, id)
}
fn route() -> Route {
    Route {
        instance: 7,
        generation: 1,
        epoch: 2,
        context: 4,
        document_revision: 13,
        owners: vec![
            Owner {
                identity: 1,
                registration: 5,
                eligible: None,
                guarded: true,
            },
            Owner {
                identity: 2,
                registration: 6,
                eligible: Some(true),
                guarded: true,
            },
            Owner {
                identity: 3,
                registration: 7,
                eligible: Some(true),
                guarded: true,
            },
        ],
    }
}
fn save_parameters() -> Parameters {
    [
        (
            "contentType".into(),
            ParameterValue::Scalar(json!("text/markdown")),
        ),
        (
            "filePath".into(),
            ParameterValue::Scalar(json!("private.md")),
        ),
    ]
    .into()
}
#[test]
fn manifest_identity_and_atomic_revision_changes() {
    assert_eq!(
        catalog(&COPY.replace(
            " @revision=1 |}",
            " @revision=1 |} {command @id=editor.copy @revision=2 |}"
        ))
        .unwrap_err(),
        "duplicate command"
    );
    assert!(catalog(&COPY.replace("@version=1", "@version=9")).is_err());
    assert!(catalog(&SAVE.replace("@name=filePath", "@name=contentType")).is_err());
    let mut commands = CommandCoordinator::new(catalog(SAVE).unwrap());
    let ticket = commands.begin("file.save-as", route()).unwrap();
    let edit = SAVE.replace("@min-length=1", "@min-length=3");
    assert!(commands.publish(catalog(&edit).unwrap()).is_err());
    assert!(commands
        .publish(catalog(&SAVE.replace("Texte brut", "Texte simple")).unwrap())
        .unwrap()
        .is_empty());
    let changed = edit.replace("@kind=action @revision=1", "@kind=action @revision=2");
    assert_eq!(
        commands.publish(catalog(&changed).unwrap()).unwrap(),
        [CommandResult {
            invocation: ticket,
            status: Status::Unavailable
        }]
    );
    assert!(commands
        .dispatch(
            ticket,
            save_parameters(),
            || Some(route()),
            |_| true,
            |_, _| panic!("retired schema")
        )
        .is_none());
    assert!(commands.publish(catalog(SAVE).unwrap()).is_err());
}
#[test]
fn save_as_validates_semantic_values_and_projects_only_public_fields() {
    let mut commands = CommandCoordinator::new(catalog(SAVE).unwrap());
    for (field, bad) in [
        ("contentType", json!("Texte brut")),
        ("contentType", json!(null)),
        ("filePath", json!("")),
        ("path", json!("extra")),
    ] {
        let mut params = save_parameters();
        params.insert(field.into(), ParameterValue::Scalar(bad));
        let id = commands.begin("file.save-as", route()).unwrap();
        assert_eq!(
            commands
                .dispatch(
                    id,
                    params,
                    || Some(route()),
                    |_| true,
                    |_, _| panic!("invalid values")
                )
                .unwrap()
                .status,
            Status::Failed
        );
    }
    let id = commands.begin("file.save-as", route()).unwrap();
    let result=commands.dispatch(id,save_parameters(),||Some(route()), |_|true, |owner,args|{
        assert_eq!(owner.identity,2);
        assert_eq!(args.public_scalars(),[("contentType",&json!("text/markdown"))].into());
        assert!(matches!(&args.endpoint_values()["filePath"],ParameterValue::Scalar(v) if v=="private.md"));
        Ok(())
    });
    assert_eq!(
        result,
        Some(CommandResult {
            invocation: id,
            status: Status::Succeeded
        })
    );
}
#[test]
fn native_and_repeated_scalar_values_preserve_identity_and_types() {
    let source=COPY.replace("@revision=1 |}", "@revision=1 | {parameters | {parameter @name=doc @type=native @required=true} {parameter @name=n @type=number @repeated=true} {parameter @name=b @type=boolean} {parameter @name=z @type=null}}}");
    let mut commands = CommandCoordinator::new(catalog(&source).unwrap());
    let tree = import_json_bytes(br#"{"owned":true}"#, "memory:owned").unwrap();
    let weak = Arc::downgrade(&tree);
    let id = commands.begin("editor.copy", route()).unwrap();
    let params = [
        ("doc".into(), ParameterValue::Native(tree)),
        ("n".into(), ParameterValue::Scalar(json!([1234, 0]))),
        ("b".into(), ParameterValue::Scalar(json!(false))),
        ("z".into(), ParameterValue::Scalar(json!(null))),
    ]
    .into();
    assert_eq!(
        commands
            .dispatch(
                id,
                params,
                || Some(route()),
                |_| true,
                |_, args| {
                    let ParameterValue::Native(doc) = &args.endpoint_values()["doc"] else {
                        panic!()
                    };
                    assert!(Arc::ptr_eq(doc, &weak.upgrade().unwrap()));
                    assert_eq!(
                        args.public_scalars().keys().copied().collect::<Vec<_>>(),
                        ["b", "n", "z"]
                    );
                    Ok(())
                }
            )
            .unwrap()
            .status,
        Status::Succeeded
    );
    assert!(weak.upgrade().is_none());
    for params in [
        Parameters::new(),
        [("doc".into(), ParameterValue::Scalar(json!(1)))].into(),
        [
            (
                "doc".into(),
                ParameterValue::Native(import_json_bytes(b"1", "memory:test").unwrap()),
            ),
            ("n".into(), ParameterValue::Scalar(json!(["1.234"]))),
        ]
        .into(),
    ] {
        let id = commands.begin("editor.copy", route()).unwrap();
        assert_eq!(
            commands
                .dispatch(id, params, || Some(route()), |_| true, |_, _| panic!())
                .unwrap()
                .status,
            Status::Failed
        );
    }
}
#[test]
fn copy_replays_busy_and_cross_coordinator_results() {
    let mut commands = CommandCoordinator::new(catalog(COPY).unwrap());
    let mut other = CommandCoordinator::new(catalog(COPY).unwrap());
    let count = Cell::new(0);
    let mut issued = vec![];
    for _ in 0..2 {
        let id = commands.begin("editor.copy", route()).unwrap();
        issued.push(id);
        let busy = commands.begin("editor.copy", route()).unwrap_err();
        assert_ne!(busy.invocation, id);
        assert_eq!(busy.status, Status::Unavailable);
        assert!(other
            .dispatch(
                id,
                Parameters::new(),
                || Some(route()),
                |_| panic!(),
                |_, _| panic!()
            )
            .is_none());
        let result = commands.dispatch(
            id,
            Parameters::new(),
            || Some(route()),
            |_| true,
            |_, _| {
                count.set(count.get() + 1);
                Ok(())
            },
        );
        assert_eq!(
            result,
            Some(CommandResult {
                invocation: id,
                status: Status::Succeeded
            })
        );
        assert!(commands
            .dispatch(
                id,
                Parameters::new(),
                || Some(route()),
                |_| panic!(),
                |_, _| panic!()
            )
            .is_none());
        assert!(commands.cancel(id).is_none());
    }
    assert_ne!(issued[0], issued[1]);
    assert_eq!(count.get(), 2);
}
#[test]
fn nearest_provider_guard_order_and_cancellation() {
    let mut commands = CommandCoordinator::new(catalog(COPY).unwrap());
    let mut r = route();
    r.owners[1].eligible = Some(false);
    assert_eq!(
        commands.begin("editor.copy", r.clone()).unwrap_err().status,
        Status::Unavailable
    );
    for owner in &mut r.owners {
        owner.eligible = None;
    }
    assert_eq!(
        commands.begin("editor.copy", r).unwrap_err().status,
        Status::Unhandled
    );
    let id = commands.begin("editor.copy", route()).unwrap();
    let guards = RefCell::new(vec![]);
    let result = commands.dispatch(
        id,
        Parameters::new(),
        || Some(route()),
        |g| {
            guards.borrow_mut().push(g.identity);
            g.identity != 2
        },
        |_, _| panic!("vetoed"),
    );
    assert_eq!(*guards.borrow(), [3, 2]);
    assert_eq!(
        result.unwrap(),
        CommandResult {
            invocation: id,
            status: Status::Cancelled
        }
    );
    let id = commands.begin("editor.copy", route()).unwrap();
    assert_eq!(commands.cancel(id).unwrap().status, Status::Cancelled);
    assert!(commands
        .dispatch(
            id,
            Parameters::new(),
            || Some(route()),
            |_| panic!(),
            |_, _| panic!()
        )
        .is_none());
}
#[test]
fn stale_route_before_and_during_guards_never_claims() {
    let mut commands = CommandCoordinator::new(catalog(COPY).unwrap());
    for changed in 0..7 {
        let id = commands.begin("editor.copy", route()).unwrap();
        let mut current = route();
        match changed {
            0 => current.instance += 1,
            1 => current.generation += 1,
            2 => current.epoch += 1,
            3 => current.context += 1,
            4 => current.document_revision += 1,
            5 => current.owners[1].registration += 1,
            _ => current.owners[1].eligible = Some(false),
        }
        let revoked = Cell::new(false);
        assert_eq!(
            commands
                .dispatch(
                    id,
                    Parameters::new(),
                    || Some(if revoked.get() {
                        current.clone()
                    } else {
                        route()
                    }),
                    |_| {
                        revoked.set(true);
                        true
                    },
                    |_, _| panic!("stale pin")
                )
                .unwrap()
                .status,
            Status::Unavailable
        );
    }
    let id = commands.begin("editor.copy", route()).unwrap();
    assert_eq!(
        commands
            .dispatch(
                id,
                Parameters::new(),
                || None,
                |_| panic!("stale before guard"),
                |_, _| panic!()
            )
            .unwrap()
            .status,
        Status::Unavailable
    );
}

#[test]
fn overlapping_contexts_settle_in_reverse_order_and_reject_ambiguous_owners() {
    let mut commands = CommandCoordinator::new(catalog(COPY).unwrap());
    let first = commands.begin("editor.copy", route()).unwrap();
    let mut second_route = route();
    second_route.context += 1;
    let second = commands.begin("editor.copy", second_route.clone()).unwrap();
    assert_eq!(
        commands.dispatch(
            second,
            Parameters::new(),
            || Some(second_route.clone()),
            |_| true,
            |_, _| Err(())
        ),
        Some(CommandResult {
            invocation: second,
            status: Status::Failed
        })
    );
    assert_eq!(
        commands.cancel(first),
        Some(CommandResult {
            invocation: first,
            status: Status::Cancelled
        })
    );
    let mut ambiguous = route();
    ambiguous.owners.push(ambiguous.owners[1].clone());
    assert_eq!(
        commands.begin("editor.copy", ambiguous).unwrap_err().status,
        Status::Unavailable
    );
}
