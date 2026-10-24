use base64::{engine::general_purpose::STANDARD, Engine};
use cem_ml::checkpoint_runtime_state::validate;
use serde_json::{json, Value};
fn state() -> Value {
    json!({"version":"cem-checkpoint-runtime/1","capability":{"kind":"form-control","authoredValue":null,"formDisabled":false,"customValidity":"Review","selectionRequest":null},
        "resources":[{"name":"page","kind":"location","disposition":"read-and-subscribe","revision":0,"cursor":0,"live":true,"state":"loaded","url":"https://example.test/docs"}],
        "writers":[{"index":0,"trigger":"save:1","implicit":false},{"index":1,"trigger":null,"implicit":true}],"writeDisposition":"suppress-until-new-input"})
}
#[test]
fn preserves_capabilities_live_resources_and_consumed_writer_state() {
    let input = state();
    assert_eq!(
        serde_json::from_str::<Value>(&validate(&input.to_string()).unwrap()).unwrap(),
        input
    );
    for kind in ["repository-query", "storage-status", "local-storage"] {
        let mut s = state();
        s["resources"][0]["kind"] = kind.into();
        s["resources"][0]["url"] = Value::Null;
        assert!(validate(&s.to_string()).is_ok(), "{kind}");
    }
    let mut mixed = state();
    mixed["writers"][0]["implicit"] = true.into();
    assert!(validate(&mixed.to_string()).is_ok());
    for cap in [
        json!({"kind":"checkable-control","authoredIndeterminate":true}),
        json!({"kind":"plaintext-editor","authored":null,"selectionRequest":null}),
        Value::Null,
    ] {
        let mut s = state();
        s["capability"] = cap;
        assert!(validate(&s.to_string()).is_ok());
    }
}
#[test]
fn retains_original_http_document_bytes_with_native_digest_and_no_handles() {
    let mut s = state();
    s["resources"][0] = json!({"name":"data","kind":"http","disposition":"reimport-and-read-again","revision":3,"cursor":0,"live":false,"state":"loaded","url":"https://example.test/data.xml","method":"GET",
        "document":{"base64":STANDARD.encode("<data>é😀</data>"),"contentType":"application/xml","sourceUri":"https://example.test/data.xml","digest":null}});
    let normalized = validate(&s.to_string()).unwrap();
    let output: Value = serde_json::from_str(&normalized).unwrap();
    assert_eq!(
        STANDARD
            .decode(
                output["resources"][0]["document"]["base64"]
                    .as_str()
                    .unwrap()
            )
            .unwrap(),
        "<data>é😀</data>".as_bytes()
    );
    assert_eq!(
        output["resources"][0]["document"]["digest"]
            .as_str()
            .unwrap()
            .len(),
        64
    );
    assert_eq!(validate(&normalized).unwrap(), normalized);
    // Reimport into an independently owned native document table, never a JS AST.
    let doc = &output["resources"][0]["document"];
    let mut documents = cem_ml::import::documents::CemDocuments::default();
    let bytes = STANDARD.decode(doc["base64"].as_str().unwrap()).unwrap();
    let id = documents
        .retain(
            &bytes,
            doc["contentType"].as_str().unwrap(),
            doc["sourceUri"].as_str().unwrap(),
        )
        .unwrap();
    let node = cem_ml::validation::xpath::XPathNativeNode::cem_document(documents.get(id).unwrap());
    assert_eq!(node.string_value(), "é😀");
    assert!(documents.dispose(id));
    let mut expanded = s.clone();
    expanded["capability"]["customValidity"] = "".into();
    let remaining = 512 * 1024 - expanded.to_string().len();
    expanded["capability"]["customValidity"] = "x".repeat(remaining).into();
    assert_eq!(expanded.to_string().len(), 512 * 1024);
    assert!(validate(&expanded.to_string()).is_err());
    s["resources"][0]["document"]["digest"] = "wrong".into();
    assert!(validate(&s.to_string()).is_err());
    s["resources"][0]["document"]["digest"] = Value::Null;
    s["resources"][0]["document"]["base64"] = "%%%".into();
    assert!(validate(&s.to_string()).is_err());
}
#[test]
fn refuses_pending_credential_unknown_duplicate_and_replayable_state() {
    for value in ["scheduled", "in-progress", "waiting"] {
        let mut s = state();
        s["resources"][0]["state"] = value.into();
        assert!(validate(&s.to_string()).is_err());
    }
    for url in [
        "https://user:secret@example.test/",
        "javascript:alert(1)",
        "relative",
    ] {
        let mut s = state();
        s["resources"][0]["url"] = url.into();
        assert!(validate(&s.to_string()).is_err());
    }
    let mut s = state();
    s["capability"]["lease"] = 7.into();
    assert!(validate(&s.to_string()).is_err());
    let mut s = state();
    s["writeDisposition"] = "retry".into();
    assert!(validate(&s.to_string()).is_err());
    let mut s = state();
    let duplicate = s["resources"][0].clone();
    s["resources"].as_array_mut().unwrap().push(duplicate);
    assert!(validate(&s.to_string()).is_err());
    let mut s = state();
    s["resources"][0]["cursor"] = 9_007_199_254_740_992u64.into();
    assert!(validate(&s.to_string()).is_err());
    let mut s = state();
    s["resources"][0]["disposition"] = "read-again".into();
    assert!(validate(&s.to_string()).is_err());
    let mut s = state();
    s["writers"][0]["trigger"] = Value::Null;
    assert!(validate(&s.to_string()).is_err());
}
#[test]
fn bounds_counts_documents_and_normalized_output() {
    let mut s = state();
    s["resources"] = json!((0..257)
        .map(|n| {
            let mut r = state()["resources"][0].clone();
            r["name"] = format!("resource:{n}").into();
            r
        })
        .collect::<Vec<_>>());
    assert!(validate(&s.to_string()).is_err());
    let mut s = state();
    s["capability"]["customValidity"] = "x".repeat(512 * 1024).into();
    assert!(validate(&s.to_string()).is_err());
    let mut s = state();
    s["resources"][0] = json!({"name":"large","kind":"http","disposition":"reimport-and-read-again","revision":1,"cursor":0,"live":false,"state":"loaded","url":"https://example.test/a","method":"GET","document":{"base64":STANDARD.encode(vec![b'x';256*1024+1]),"contentType":"text/plain","sourceUri":"https://example.test/a","digest":null}});
    assert!(validate(&s.to_string()).is_err());
}

#[test]
fn choice_editors_preserve_selection_preview_defaults_and_clock_state() {
    let select = json!({"kind":"choice-select","selection":["b"],"defaultValues":["a"],"activeIndex":1,"rangeAnchor":0,"expanded":true,"previewSelection":["a"],"mode":"dropdown","authoredValue":null,"valueRequest":"request:1","formDisabled":false,"typeahead":"b","typeaheadAt":1234,"focusedPart":"control"});
    let editable = json!({"kind":"editable-choice","activeIndex":0,"authoredValue":null,"committedOptionValue":"a","defaultCommittedOptionValue":"a","defaultDisplayValue":"Alpha","defaultValue":"a","displayValue":"Draft é","edited":true,"expanded":false,"formDisabled":false,"value":"Draft é"});
    for capability in [select, editable] {
        let mut s = state();
        s["capability"] = capability;
        assert!(validate(&s.to_string()).is_ok());
    }
    let mut s = state();
    s["capability"] = json!({"kind":"choice-select","selection":[],"defaultValues":[],"activeIndex":-2,"rangeAnchor":0,"expanded":false,"previewSelection":[],"mode":"bad","authoredValue":null,"valueRequest":null,"formDisabled":false,"typeahead":"","typeaheadAt":0,"focusedPart":null});
    assert!(validate(&s.to_string()).is_err());
}

#[test]
fn validates_stable_non_form_interaction_and_focus_contracts() {
    for capability in [
        json!({"kind":"navigation-landmark"}),
        json!({"kind":"tree-view","focusValue":"docs","typeahead":"do","typeaheadAt":12}),
        json!({"kind":"tab-panels","focusIndex":1,"tabCount":3}),
        json!({"kind":"composite-menu","activeNode":"menu.1","typeahead":"s","typeaheadAt":4,"open":false}),
        json!({"kind":"popup","open":false}),
        json!({"kind":"native-surface","phase":"closed","defaultsInitialized":true}),
    ] {
        let mut s = state();
        s["capability"] = capability;
        s["focus"] = json!({"target":"node","nodeId":"render.1"});
        let result = validate(&s.to_string()).unwrap();
        assert_eq!(serde_json::from_str::<Value>(&result).unwrap(), s);
    }
    for capability in [
        json!({"kind":"tree-view","focusValue":"docs","typeahead":"x".repeat(4097),"typeaheadAt":12}),
        json!({"kind":"tab-panels","focusIndex":3,"tabCount":3}),
        json!({"kind":"tab-panels","focusIndex":-2,"tabCount":3}),
        json!({"kind":"composite-menu","activeNode":null,"typeahead":"","typeaheadAt":0,"open":true}),
        json!({"kind":"popup","open":true}),
        json!({"kind":"native-surface","phase":"preparing","defaultsInitialized":true}),
        json!({"kind":"suggestions","token":"old-native-authority"}),
    ] {
        let mut s = state();
        s["capability"] = capability;
        assert!(validate(&s.to_string()).is_err(), "{s}");
    }
    for focus in [
        json!({"target":"host","nodeId":"not-host"}),
        json!({"target":"node","nodeId":""}),
        json!({"target":"selector","nodeId":"input"}),
    ] {
        let mut s = state();
        s["focus"] = focus;
        assert!(validate(&s.to_string()).is_err());
    }
}

#[test]
fn active_suggestions_keep_source_order_proof_and_require_reacquisition() {
    let mut s = state();
    s["capability"] = json!({"kind":"suggestions","state":{
        "source":{"version":"cem-native-source/1","revision":"source:1","rowCount":2,"select":"input.children","resolve":false,"requesting":0,
            "sources":[{"codec":"xml","base64":STANDARD.encode("<options><option value='same'>First</option><option value='same'>Second</option></options>"),"sourceUri":"https://example.test/options.xml","primarySourceId":1,"context":true}],"bindings":[],"crossings":[],"labels":{"option":null,"group":null},"disposition":"reacquire"},
        "publication":{"filter":"none","filterBy":null,"expanded":null,"active":null,"committed":null},"providerRevision":2,"queryRevision":3,"preparedRevision":2,"query":"same","intent":true,"feedbackIntent":true,"feedbackState":"ready","activeIndex":1,"committedIndex":0,"committedValue":"same","visible":true,"scrollTop":4,"scrollLeft":0,
        "rows":[{"index":0,"value":"same","eligible":true,"available":true,"nodeId":"row.0"},{"index":1,"value":"same","eligible":true,"available":true,"nodeId":"row.1"}]}});
    let normalized = validate(&s.to_string()).unwrap();
    let portable: Value = serde_json::from_str(&normalized).unwrap();
    assert_eq!(portable["capability"]["state"]["committedIndex"], 0);
    assert_eq!(portable["capability"]["state"]["activeIndex"], 1);
    assert_eq!(
        portable["capability"]["state"]["source"]["sources"][0]["digest"]
            .as_str()
            .unwrap()
            .len(),
        64
    );
    for (name, value) in [
        ("feedbackState", json!("pending")),
        ("activeIndex", json!(2)),
        ("committedIndex", json!(-1)),
        ("preparedRevision", json!(1)),
        ("sessionKey", json!("old-authority")),
    ] {
        let mut bad = s.clone();
        bad["capability"]["state"][name] = value;
        assert!(validate(&bad.to_string()).is_err(), "{name}");
    }
    let mut bad = s.clone();
    bad["capability"]["state"]["source"]["sources"][0]["codec"] = json!("js-ast");
    assert!(validate(&bad.to_string()).is_err());
}

#[test]
fn placement_sources_require_original_bundles_and_unique_inert_routes() {
    use cem_ml::{
        ast::reload::{ReferenceReloadBundle, ReloadLimits, ReloadSource},
        import::import_bytes_with_lexical_scopes,
        schema::vocab::CompiledSchema,
        source::SourceId,
    };
    let text = "{dialog | Target}";
    let imported = import_bytes_with_lexical_scopes(
        text.as_bytes(),
        "text/cem-ml",
        "memory:target.cem",
        CompiledSchema::cem_core(),
    )
    .unwrap();
    let bundle = ReferenceReloadBundle::export(
        &imported.captured,
        vec![ReloadSource::new(
            SourceId(1),
            "memory:target.cem",
            text.as_bytes(),
            true,
        )],
        ReloadLimits::default(),
    )
    .unwrap()
    .encode(ReloadLimits::default())
    .unwrap();
    let source = json!({"version":"cem-native-source/1","revision":"producer:1","rowCount":0,"select":"input.children","resolve":false,"requesting":0,
        "sources":[{"codec":"cemb","base64":STANDARD.encode(bundle),"sourceUri":null,"primarySourceId":1,"context":true}],"bindings":[],"crossings":[],"labels":{"option":null,"group":null},"disposition":"reacquire"});
    let mut references = state();
    references["references"] = source.clone();
    assert!(validate(&references.to_string()).is_ok());
    let route = json!({"requesterNode":"button.1","property":"commandfor","producer":"producer","revision":"producer:1","providerNode":"dialog.1","sourceIndex":0,"path":[0],"select":"input.children","source":source,"disposition":"reacquire"});
    let mut s = state();
    s["placements"] = json!([route.clone()]);
    assert!(validate(&s.to_string()).is_ok());
    s["placements"] = json!([route.clone(), route]);
    assert!(validate(&s.to_string()).is_err());
    s["placements"].as_array_mut().unwrap().pop();
    for (name, value) in [
        ("token", json!("old-grant")),
        ("property", json!("onclick")),
        ("disposition", json!("restore-grant")),
    ] {
        let mut bad = s.clone();
        bad["placements"][0][name] = value;
        assert!(validate(&bad.to_string()).is_err());
    }
    let mut bad = s;
    bad["placements"][0]["source"]["sources"][0]["base64"] = json!(STANDARD.encode("not a bundle"));
    assert!(validate(&bad.to_string()).is_err());
}
#[test]
fn settled_surface_sessions_preserve_return_values_and_refuse_transitions() {
    let mut s = state();
    let session = json!({"returnValue":"done","modal":true,"focusWithin":true,"closeReason":"native","disposition":"reacquire","invocation":{
        "source":{"instanceId":"toolbar","dataRevision":"1","nodeId":"open.1"},"returnDestination":{"instanceId":"toolbar","dataRevision":"1","nodeId":"open.1"},"logicalParent":null,
        "command":"show-modal","contextKey":"record:2","inputKind":"keyboard","pointer":null,"selection":null}});
    s["capability"] =
        json!({"kind":"native-surface","phase":"open","defaultsInitialized":true,"state":session});
    assert!(validate(&s.to_string()).is_ok());
    for phase in ["preparing", "closing"] {
        let mut bad = s.clone();
        bad["capability"]["phase"] = json!(phase);
        assert!(validate(&bad.to_string()).is_err());
    }
    let mut bad = s.clone();
    bad["capability"]["state"]["invocation"]["context"] = json!("native-handle");
    assert!(validate(&bad.to_string()).is_err());
    s["capability"]["phase"] = json!("closed");
    s["capability"]["state"]["modal"] = json!(false);
    s["capability"]["state"]["invocation"] = Value::Null;
    assert!(validate(&s.to_string()).is_ok());
    s["capability"] = json!({"kind":"popup","open":true,"state":{"trigger":"button.1","panel":"aside.1","scrollTop":12,"scrollLeft":0,"disposition":"reacquire"}});
    assert!(validate(&s.to_string()).is_ok());
}

#[test]
fn completed_provider_commits_keep_exact_one_shot_change_suppression() {
    let mut s = state();
    s["capability"]["provider"] = json!({"revision":4,"consumer":"suggestions","validity":""});
    s["capability"]["changeSuppression"] =
        json!({"revision":4,"value":"same","disposition":"suppress-once"});
    assert!(validate(&s.to_string()).is_ok());
    s["capability"]["changeSuppression"]["revision"] = json!(3);
    assert!(validate(&s.to_string()).is_err());
    s["capability"]["changeSuppression"]["revision"] = json!(4);
    s["capability"]["changeSuppression"]["disposition"] = json!("replay");
    assert!(validate(&s.to_string()).is_err());
}

#[test]
fn inactive_suggestions_preserve_source_intent_without_old_proofs() {
    let source = json!({"version":"cem-native-source/1","revision":"source:inactive","rowCount":0,"select":"input.children","resolve":false,"requesting":0,
        "sources":[{"codec":"xml","base64":STANDARD.encode("<options/>"),"sourceUri":"https://example.test/options.xml","primarySourceId":1,"context":true}],"bindings":[],"crossings":[],"labels":{"option":null,"group":null},"disposition":"reacquire"});
    for profile in ["listbox", "native-datalist"] {
        for feedback in ["idle", "failed", "unavailable"] {
            let mut s = state();
            s["capability"] = json!({"kind":"suggestions","state":{"phase":"inactive","profile":profile,"source":source,"providerRevision":4,"queryRevision":5,"query":"edited","feedbackState":feedback,"feedbackIntent":false,"editable":feedback != "unavailable"}});
            assert!(validate(&s.to_string()).is_ok(), "{s}");
            for (key, value) in [
                ("feedbackState", json!("pending")),
                ("activeIndex", json!(0)),
                ("committedValue", json!("old")),
                ("lease", json!("old")),
                ("providerRevision", json!(9_007_199_254_740_992u64)),
            ] {
                let mut bad = s.clone();
                bad["capability"]["state"][key] = value;
                assert!(validate(&bad.to_string()).is_err(), "{key}");
            }
        }
    }
}
#[test]
fn native_datalist_checkpoints_reacquire_claims_and_reset_ua_presentation() {
    let mut s = state();
    s["capability"] = json!({"kind":"suggestions","state":{"phase":"native-datalist","providerRevision":3,"targetNode":"datalist.1","claim":"attached","presentation":"reset-on-resume",
        "source":{"version":"cem-native-source/1","revision":"source:datalist","rowCount":1,"select":"input.children","resolve":false,"requesting":0,
        "sources":[{"codec":"xml","base64":STANDARD.encode("<options><option value='A'/></options>"),"sourceUri":"https://example.test/options.xml","primarySourceId":1,"context":true}],"bindings":[],"crossings":[],"labels":{"option":null,"group":null},"disposition":"reacquire"}}});
    assert!(validate(&s.to_string()).is_ok());
    s["capability"]["state"]["claim"] = json!("unavailable");
    assert!(validate(&s.to_string()).is_ok());
    for (key, value) in [
        ("targetNode", json!("")),
        ("claim", json!("restore-old-lease")),
        ("presentation", json!("restore-popup")),
        ("rows", json!([])),
        ("providerRevision", json!(9_007_199_254_740_992u64)),
    ] {
        let mut bad = s.clone();
        bad["capability"]["state"][key] = value;
        assert!(validate(&bad.to_string()).is_err(), "{key}");
    }
}

#[test]
fn inactive_retained_proof_requires_original_source_ordinal_and_fresh_validation() {
    let source = json!({"version":"cem-native-source/1","revision":"retained:1","rowCount":2,"select":"input.children","resolve":false,"requesting":0,
        "sources":[{"codec":"xml","base64":STANDARD.encode("<options><option value='same'/><option value='same'/></options>"),"sourceUri":"https://example.test/options.xml","primarySourceId":1,"context":true}],"bindings":[],"crossings":[],"labels":{"option":null,"group":null},"disposition":"reacquire"});
    let mut s = state();
    let mut intent = source.clone();
    intent["rowCount"] = json!(0);
    s["capability"] = json!({"kind":"suggestions","state":{"phase":"inactive","profile":"listbox","source":intent,"providerRevision":4,"queryRevision":5,"query":"same","feedbackState":"unavailable","feedbackIntent":false,"editable":false,
        "proof":{"source":source,"index":1,"value":"same","disposition":"revalidate"}}});
    assert!(validate(&s.to_string()).is_ok());
    for (key, value) in [
        ("index", json!(2)),
        ("value", json!("different")),
        ("disposition", json!("restore-handle")),
        ("handle", json!("old")),
    ] {
        let mut bad = s.clone();
        bad["capability"]["state"]["proof"][key] = value;
        assert!(validate(&bad.to_string()).is_err(), "{key}");
    }
    let mut bad = s.clone();
    bad["capability"]["state"]["profile"] = json!("native-datalist");
    assert!(validate(&bad.to_string()).is_err());
    s["capability"]["state"]["proof"]["source"]["sources"][0]["base64"] =
        json!(STANDARD.encode("<invalid"));
    assert!(validate(&s.to_string()).is_err());
}

#[test]
fn settled_submenus_require_bounded_relationship_intent_without_live_authority() {
    let endpoint = json!({"instanceId":"menu-parent","dataRevision":"4","nodeId":"menu.1"});
    let mut s = state();
    s["capability"] = json!({"kind":"composite-menu","activeNode":"item.1","typeahead":"m","typeaheadAt":4,"open":true,
        "state":{"parent":endpoint,"launcher":endpoint,"panel":endpoint,"disposition":"reacquire"}});
    assert!(validate(&s.to_string()).is_ok());
    for (key, value) in [
        ("launcher", Value::Null),
        ("panel", Value::Null),
        ("disposition", json!("restore-grant")),
        ("geometry", json!({})),
    ] {
        let mut bad = s.clone();
        bad["capability"]["state"][key] = value;
        assert!(validate(&bad.to_string()).is_err(), "{key}");
    }
    let mut bad = s.clone();
    bad["capability"]["state"]["parent"]["dataRevision"] = json!("");
    assert!(validate(&bad.to_string()).is_err());
    s["capability"]["open"] = json!(false);
    s["capability"]["state"]["panel"] = Value::Null;
    assert!(validate(&s.to_string()).is_ok());
}

#[test]
fn native_publication_consumers_require_original_sources_and_exact_row_contracts() {
    let mut s = state();
    s["capability"] = Value::Null;
    s["nativePublication"] = json!({"profile":"listbox","source":{"version":"cem-native-source/1","revision":"external:1","rowCount":2,"select":"input.children","resolve":false,"requesting":0,
        "sources":[{"codec":"xml","base64":STANDARD.encode("<options><option value='same'/><option value='same'/></options>"),"sourceUri":"https://example.test/options.xml","primarySourceId":1,"context":true}],"bindings":[],"crossings":[],"labels":{"option":null,"group":null},"disposition":"reacquire"},
        "query":"same","queryRevision":2,"publication":{"filter":"external","filterBy":null,"expanded":null,"active":null,"committed":null},
        "rows":[{"index":0,"value":"same","eligible":true,"available":true,"nodeId":"row.0"},{"index":1,"value":"same","eligible":false,"available":true,"nodeId":"row.1"}],"disposition":"reacquire"});
    let normalized = validate(&s.to_string()).unwrap();
    assert_eq!(validate(&normalized).unwrap(), normalized);
    for (key, value) in [
        ("profile", json!("native-datalist")),
        ("queryRevision", json!(9_007_199_254_740_992u64)),
        ("disposition", json!("restore-binding")),
        ("sessionKey", json!("old")),
    ] {
        let mut bad = s.clone();
        bad["nativePublication"][key] = value;
        assert!(validate(&bad.to_string()).is_err(), "{key}");
    }
    for (key, value) in [
        ("index", json!(0)),
        ("nodeId", json!("row.0")),
        ("index", json!(2)),
        ("available", json!(false)),
    ] {
        let mut bad = s.clone();
        bad["nativePublication"]["rows"][1]["eligible"] = json!(true);
        bad["nativePublication"]["rows"][1][key] = value;
        assert!(validate(&bad.to_string()).is_err(), "{key}");
    }
    let mut bad = s.clone();
    bad["nativePublication"]["publication"]["filter"] = json!("unknown");
    assert!(validate(&bad.to_string()).is_err());
    let mut bad = s.clone();
    bad["nativePublication"]["source"]["sources"][0]["base64"] = json!(STANDARD.encode("<invalid"));
    assert!(validate(&bad.to_string()).is_err());
    let large = format!("<options>{}</options>", "x".repeat(140_000));
    s["nativePublication"]["source"]["sources"][0]["base64"] = json!(STANDARD.encode(large));
    let mut references = s["nativePublication"]["source"].clone();
    references["rowCount"] = json!(0);
    s["references"] = references;
    assert!(validate(&s.to_string()).is_err());
}

#[test]
fn standalone_delegates_and_deferred_bodies_have_inert_exact_contracts() {
    let endpoint = json!({"instanceId":"owner","dataRevision":"1","nodeId":"panel"});
    let delegate = json!({"panel":endpoint,"editor":{"instanceId":"editor","dataRevision":"2","nodeId":"host"},"providerRevision":2,"visible":false,"presentation":"reset-on-resume","disposition":"reacquire"});
    let body = json!({"templateNode":"template","mode":"retain","materialized":true,"roots":["input","text"],"disposition":"rematerialize"});
    let mut s = state();
    s["capability"] = json!({"kind":"native-surface","phase":"closed","defaultsInitialized":true,"state":{"returnValue":"","modal":false,"focusWithin":false,"closeReason":"native","invocation":null,"disposition":"reacquire","body":body}});
    s["manualListbox"] = delegate.clone();
    assert_eq!(
        validate(&validate(&s.to_string()).unwrap()).unwrap(),
        validate(&s.to_string()).unwrap()
    );
    for bad in [
        json!({"panel":endpoint,"editor":null,"providerRevision":2,"visible":false,"presentation":"reset-on-resume","disposition":"reacquire"}),
        {
            let mut v = delegate.clone();
            v["providerRevision"] = json!(9_007_199_254_740_992u64);
            v
        },
        {
            let mut v = delegate.clone();
            v["lease"] = json!("authority");
            v
        },
        {
            let mut v = delegate.clone();
            v["presentation"] = json!("restore-platform-picker");
            v
        },
        {
            let mut v = delegate.clone();
            v["panel"]["nodeId"] = json!("");
            v
        },
    ] {
        let mut bad_state = s.clone();
        bad_state["manualListbox"] = bad;
        assert!(validate(&bad_state.to_string()).is_err());
    }
    for (key, value) in [
        ("mode", json!("eager")),
        ("roots", json!(["same", "same"])),
        ("roots", json!(["template"])),
        ("roots", json!([""])),
        ("disposition", json!("reuse-dom")),
        ("grant", json!("forged")),
        (
            "roots",
            json!((0..257).map(|i| format!("n{i}")).collect::<Vec<_>>()),
        ),
    ] {
        let mut v = s.clone();
        v["capability"]["state"]["body"][key] = value;
        assert!(validate(&v.to_string()).is_err(), "{key}");
    }
    s["capability"]["state"]["body"]["materialized"] = json!(false);
    assert!(validate(&s.to_string()).is_err());
    s["capability"]["state"]["body"]["roots"] = json!([]);
    assert!(validate(&s.to_string()).is_ok());
}

#[test]
fn original_retained_projections_preserve_ranges_and_nested_owner_intent() {
    let entries = json!([
        {"kind":"element","nodeId":"child","tag":"cem-field","namespace":null,"attributes":[{"name":"value","value":"Original"}]},
        {"kind":"end"},
        {"kind":"range-start","nodeId":"text:start","rangeId":"text"},
        {"kind":"text","nodeId":"text","text":"Old output"},
        {"kind":"range-end","nodeId":"text:end","rangeId":"text"}
    ]);
    let body = json!({"templateNode":"template","mode":"retain","materialized":true,"roots":["child","text:start","text","text:end"],"disposition":"rematerialize",
        "reuseOnOpen":true,"projection":{"version":"cem-render-projection/1","origin":{"instanceId":"parent","templateArtifactId":"artifact","dataRevision":"1","scopePolicyStamp":"policy"},"entries":entries},"owners":[{"hostNode":"child","owner":{"instanceId":"nested","dataRevision":"4","nodeId":"host"}}]});
    let mut s = state();
    s["capability"] = json!({"kind":"native-surface","phase":"closed","defaultsInitialized":true,"state":{"returnValue":"","modal":false,"focusWithin":false,"closeReason":"native","invocation":null,"disposition":"reacquire","body":body}});
    let normalized = validate(&s.to_string()).unwrap();
    assert_eq!(validate(&normalized).unwrap(), normalized);
    let mut replace = s.clone();
    replace["capability"]["state"]["body"]["reuseOnOpen"] = json!(false);
    assert!(validate(&replace.to_string()).is_ok());
    for (path, value) in [
        ("/capability/state/body/reuseOnOpen", Value::Null),
        (
            "/capability/state/body/projection/entries/0/attributes",
            json!([{"name":"type","value":"password"}]),
        ),
        (
            "/capability/state/body/projection/entries/0/attributes",
            json!([{"name":"autocomplete","value":"section-login current-password"}]),
        ),
        (
            "/capability/state/body/projection/version",
            json!("unknown"),
        ),
        (
            "/capability/state/body/projection/origin/dataRevision",
            json!(""),
        ),
        (
            "/capability/state/body/projection/entries/0/nativeValue",
            json!("handle"),
        ),
        (
            "/capability/state/body/projection/entries/0/nodeId",
            json!("text"),
        ),
        (
            "/capability/state/body/projection/entries/4/rangeId",
            json!("foreign"),
        ),
        (
            "/capability/state/body/projection/entries/0/attributes",
            json!([{"name":"value","value":"a"},{"name":"value","value":"b"}]),
        ),
        ("/capability/state/body/roots", json!(["child"])),
        ("/capability/state/body/owners/0/hostNode", json!("text")),
        ("/capability/state/body/owners/0/owner/nodeId", json!("")),
        ("/capability/state/body/materialized", json!(false)),
    ] {
        let mut bad = s.clone();
        // Unknown fields are explicit negative transport cases.
        if path.ends_with("nativeValue") {
            bad["capability"]["state"]["body"]["projection"]["entries"][0]["nativeValue"] = value;
        } else {
            *bad.pointer_mut(path).unwrap() = value;
        }
        assert!(validate(&bad.to_string()).is_err(), "{path}");
    }
    let mut bad = s.clone();
    bad["capability"]["state"]["body"]["projection"]["entries"] = json!([{"kind":"end"}]);
    assert!(validate(&bad.to_string()).is_err());
    let mut bad = s.clone();
    bad["capability"]["state"]["body"]["owners"] =
        json!([body["owners"][0].clone(), body["owners"][0].clone()]);
    assert!(validate(&bad.to_string()).is_err());
    let mut bad = s.clone();
    bad["capability"]["state"]["body"]["projection"]["entries"][3]["text"] =
        json!("x".repeat(256 * 1024 + 1));
    assert!(validate(&bad.to_string()).is_err());
    let mut aggregate = s.clone();
    aggregate["capability"]["state"]["body"]["projection"]["entries"][3]["text"] =
        json!("x".repeat(140_000));
    aggregate["resources"] = json!([{ "name":"data", "kind":"http", "disposition":"reimport-and-read-again", "revision":1, "cursor":0, "live":false, "state":"loaded", "url":"https://example.test/source.xml", "method":"GET",
        "document":{"base64":STANDARD.encode(format!("<root>{}</root>","x".repeat(140_000))),"contentType":"application/xml","sourceUri":"https://example.test/source.xml","digest":null} }]);
    assert!(aggregate.to_string().len() < 512 * 1024);
    assert!(validate(&aggregate.to_string()).is_err());
}

#[test]
fn standalone_datalist_ownership_requires_portable_source_and_reset_intent() {
    let mut s = state();
    s["capability"] = Value::Null;
    let delegate = json!({"editor":{"instanceId":"field","dataRevision":"2","nodeId":"host"},
        "target":{"instanceId":"options","dataRevision":"3","nodeId":"datalist"},
        "providerRevision":2,"claim":"attached","presentation":"reset-on-resume","disposition":"reacquire",
        "source":{"version":"cem-native-source/1","revision":"options:1","rowCount":2,"select":"input.children","resolve":false,"requesting":0,
            "sources":[{"codec":"xml","base64":STANDARD.encode("<options><option value='same'/><option value='same'/></options>"),"sourceUri":"https://example.test/options.xml","primarySourceId":1,"context":true}],
            "bindings":[],"crossings":[],"labels":{"option":null,"group":null},"disposition":"reacquire"}});
    s["standaloneDatalist"] = delegate.clone();
    let normalized = validate(&s.to_string()).unwrap();
    assert_eq!(validate(&normalized).unwrap(), normalized);
    s["standaloneDatalist"]["claim"] = json!("unavailable");
    assert!(validate(&s.to_string()).is_ok());
    for (key, value) in [
        ("editor", Value::Null),
        (
            "target",
            json!({"instanceId":"options","dataRevision":"3","nodeId":""}),
        ),
        ("providerRevision", json!(9_007_199_254_740_992u64)),
        ("claim", json!("author-list")),
        ("presentation", json!("restore-picker")),
        ("disposition", json!("restore-lease")),
        ("lease", json!("authority")),
    ] {
        let mut bad = s.clone();
        bad["standaloneDatalist"][key] = value;
        assert!(validate(&bad.to_string()).is_err(), "{key}");
    }
    for (part, key, value) in [
        ("editor", "nodeId", "input"),
        ("target", "nodeId", "host"),
        ("target", "instanceId", "field"),
    ] {
        let mut bad = s.clone();
        bad["standaloneDatalist"][part][key] = json!(value);
        assert!(validate(&bad.to_string()).is_err());
    }
    let mut bad = s.clone();
    bad["standaloneDatalist"]["source"]["sources"][0]["base64"] =
        json!(STANDARD.encode("<invalid"));
    assert!(validate(&bad.to_string()).is_err());
    s["standaloneDatalist"]["source"]["sources"][0]["base64"] =
        json!(STANDARD.encode(format!("<options>{}</options>", "x".repeat(140_000))));
    s["references"] = s["standaloneDatalist"]["source"].clone();
    s["references"]["rowCount"] = json!(0);
    assert!(validate(&s.to_string()).is_err());
}

fn menu_policy_state() -> Value {
    let endpoint = json!({"instanceId":"owner","dataRevision":"2","nodeId":"target"});
    let mut s = state();
    s["capability"] = json!({"kind":"composite-menu","activeNode":"item","typeahead":"","typeaheadAt":0,"open":false,
        "state":{"parent":null,"launcher":null,"panel":null,"disposition":"reacquire","policies":{
            "compositeNode":"composite","anchor":{"mode":"target","target":endpoint},
            "boundary":{"mode":"target","target":endpoint},"boundaryOwner":{"instanceId":"parent","dataRevision":"3","nodeId":"host"},
            "focusTarget":{"mode":"target","target":endpoint},"returnFocus":{"mode":"target","target":endpoint},
            "anchorLost":"freeze","geometry":"recompute"}}});
    s
}
#[test]
fn submenu_geometry_and_focus_policies_round_trip_as_reacquisition_intent() {
    let mut s = menu_policy_state();
    let normalized = validate(&s.to_string()).unwrap();
    assert_eq!(validate(&normalized).unwrap(), normalized);
    for (role, mode) in [
        ("anchor", "invoker"),
        ("boundary", "viewport"),
        ("focusTarget", "auto"),
        ("returnFocus", "none"),
    ] {
        s["capability"]["state"]["policies"][role] = json!({"mode":mode,"target":null});
        assert!(validate(&s.to_string()).is_ok());
    }
    s["capability"]["state"]["policies"]["boundaryOwner"] = Value::Null;
    s["capability"]["state"]["policies"]["anchorLost"] = json!("close");
    assert!(validate(&s.to_string()).is_ok());
}
#[test]
fn submenu_policy_contracts_refuse_incomplete_roles_pixels_and_private_authority() {
    let s = menu_policy_state();
    for (key, value) in [
        ("compositeNode", json!("")),
        ("compositeNode", json!("host")),
        ("anchorLost", json!("restore")),
        ("geometry", json!("restore-pixels")),
        ("rect", json!({"left":1})),
        ("lease", json!("authority")),
    ] {
        let mut bad = s.clone();
        bad["capability"]["state"]["policies"][key] = value;
        assert!(validate(&bad.to_string()).is_err(), "{key}");
    }
    for role in ["anchor", "boundary", "focusTarget", "returnFocus"] {
        for value in [
            json!({"mode":"target","target":null}),
            json!({"mode":"unknown","target":null}),
            json!({"mode":"auto","target":{"instanceId":"x","dataRevision":"1","nodeId":"node"}}),
            json!({"mode":"target","target":{"instanceId":"x","dataRevision":"","nodeId":"node"}}),
            json!({"mode":"target","target":{"instanceId":"x","dataRevision":"1","nodeId":"node","grant":"private"}}),
        ] {
            let mut bad = s.clone();
            bad["capability"]["state"]["policies"][role] = value;
            assert!(validate(&bad.to_string()).is_err(), "{role}");
        }
        let mut bad = s.clone();
        bad["capability"]["state"]["policies"]
            .as_object_mut()
            .unwrap()
            .remove(role);
        assert!(validate(&bad.to_string()).is_err());
    }
    let mut bad = s;
    bad["capability"]["state"]["policies"]["boundaryOwner"]["nodeId"] = json!("raw-node");
    assert!(validate(&bad.to_string()).is_err());
}

fn native_projection_state() -> Value {
    use cem_ml::value::artifact::{
        CemValueArtifactLimits, CemValueGraph, CemValueProvenance, CemValueRecord,
    };
    let graph = CemValueGraph {
        roots: vec![0],
        records: vec![
            CemValueRecord {
                kind: "attribute".into(),
                name: "native-options".into(),
                native_content: true,
                values: vec![1, 2],
                ..Default::default()
            },
            CemValueRecord {
                lexical: "42".into(),
                datatype: "integer".into(),
                provenance: Some(CemValueProvenance {
                    source_uri: Some("https://example.test/options.cem".into()),
                    source_key: Some("original".into()),
                    line_number: Some(3),
                }),
                ..Default::default()
            },
            CemValueRecord {
                lexical: "é😀".into(),
                ..Default::default()
            },
        ],
    };
    let artifact = graph.encode(&CemValueArtifactLimits::default()).unwrap();
    let hash = cem_ml::content_cache::ContentHash::from_blake3(&artifact).header_value();
    let mut s = state();
    s["capability"] = json!({"kind":"native-surface","phase":"closed","defaultsInitialized":true,"state":{"returnValue":"","modal":false,"focusWithin":false,"closeReason":"native","invocation":null,"disposition":"reacquire","body":{
        "templateNode":"template","mode":"retain","materialized":true,"roots":["child"],"disposition":"rematerialize","reuseOnOpen":false,
        "projection":{"version":"cem-render-projection/1","origin":{"instanceId":"parent","templateArtifactId":"artifact","dataRevision":"1","scopePolicyStamp":"policy"},
            "nativeArtifacts":[{"contentHash":hash,"base64":STANDARD.encode(artifact)}],
            "entries":[{"kind":"element","nodeId":"child","tag":"cem-options","namespace":null,"attributes":[{"name":"native-options","value":"42é😀","nativeValue":{"artifact":0,"index":0}}]},{"kind":"end"}]}
    }}});
    s
}
#[test]
fn retained_native_capsules_keep_original_bytes_types_and_source_provenance() {
    let s = native_projection_state();
    let normalized = validate(&s.to_string()).unwrap();
    assert_eq!(validate(&normalized).unwrap(), normalized);
    let saved: Value = serde_json::from_str(&normalized).unwrap();
    assert_eq!(saved, s);
    let artifact = &saved["capability"]["state"]["body"]["projection"]["nativeArtifacts"][0];
    let bytes = STANDARD
        .decode(artifact["base64"].as_str().unwrap())
        .unwrap();
    let fresh =
        cem_ml::value::artifact::CemValueGraph::decode(&bytes, &Default::default()).unwrap();
    assert_eq!(fresh.records[1].datatype, "integer");
    assert_eq!(
        fresh.records[1]
            .provenance
            .as_ref()
            .unwrap()
            .source_uri
            .as_deref(),
        Some("https://example.test/options.cem")
    );
    assert_eq!(fresh.records[2].lexical, "é😀");
    let mut shared = s;
    let p = &mut shared["capability"]["state"]["body"]["projection"];
    p["entries"][0]["attributes"]
        .as_array_mut()
        .unwrap()
        .push(json!({"name":"another","value":""}));
    assert!(validate(&shared.to_string()).is_ok());
}
#[test]
fn retained_native_capsules_reject_corruption_authority_and_mismatched_bindings() {
    let s = native_projection_state();
    let prefix = "/capability/state/body/projection/";
    for (path, value) in [
        ("nativeArtifacts/0/contentHash", json!("forged")),
        (
            "nativeArtifacts/0/base64",
            json!(STANDARD.encode(b"CEMV invalid")),
        ),
        ("entries/0/attributes/0/nativeValue/artifact", json!(1)),
        ("entries/0/attributes/0/nativeValue/index", json!(1)),
        ("entries/0/attributes/0/name", json!("different")),
    ] {
        let mut bad = s.clone();
        *bad.pointer_mut(&format!("{prefix}{path}")).unwrap() = value;
        assert!(validate(&bad.to_string()).is_err(), "{path}");
    }
    for key in ["handle", "grant", "sourceHandle"] {
        let mut bad = s.clone();
        bad["capability"]["state"]["body"]["projection"]["nativeArtifacts"][0][key] =
            json!("private");
        assert!(validate(&bad.to_string()).is_err());
    }
    let mut bad = s.clone();
    bad["capability"]["state"]["body"]["projection"]["entries"][0]["attributes"][0]
        ["nativeValue"] = Value::Null;
    assert!(validate(&bad.to_string()).is_err()); // No unreferenced hidden artifact payloads.
}

#[test]
fn retained_native_capsules_share_artifacts_and_enforce_privacy_and_aggregate_limits() {
    use cem_ml::value::artifact::{CemValueGraph, CemValueRecord};
    let s = native_projection_state();
    let p = &s["capability"]["state"]["body"]["projection"];
    let decoded = STANDARD
        .decode(p["nativeArtifacts"][0]["base64"].as_str().unwrap())
        .unwrap();
    let graph = CemValueGraph::decode(&decoded, &Default::default()).unwrap();
    let install = |state: &mut Value, graph: &CemValueGraph| {
        let bytes = graph.encode(&Default::default()).unwrap();
        state["capability"]["state"]["body"]["projection"]["nativeArtifacts"][0] = json!({
            "contentHash":cem_ml::content_cache::ContentHash::from_blake3(&bytes).header_value(),"base64":STANDARD.encode(bytes)});
    };
    let mut shared = s.clone();
    let mut two = graph.clone();
    let mut attribute = two.records[0].clone();
    attribute.name = "second".into();
    two.roots.push(two.records.len() as u32);
    two.records.push(attribute);
    install(&mut shared, &two);
    shared["capability"]["state"]["body"]["projection"]["entries"][0]["attributes"]
        .as_array_mut()
        .unwrap()
        .push(json!({"name":"second","value":"","nativeValue":{"artifact":0,"index":1}}));
    assert!(validate(&shared.to_string()).is_ok());
    for (name, value) in [
        ("type", "PASSWORD"),
        ("autocomplete", "section-a one-time-code"),
    ] {
        let mut private = graph.clone();
        private.records.push(CemValueRecord {
            kind: "attribute".into(),
            name: name.into(),
            lexical: value.into(),
            ..Default::default()
        });
        let mut bad = s.clone();
        install(&mut bad, &private);
        assert!(validate(&bad.to_string()).is_err());
    }
    let mut credential = graph.clone();
    credential.records[1]
        .provenance
        .as_mut()
        .unwrap()
        .source_uri = Some("https://user:secret@example.test/source.cem".into());
    let mut bad = s.clone();
    install(&mut bad, &credential);
    assert!(validate(&bad.to_string()).is_err());
    let mut large = graph.clone();
    large.records[2].lexical = "x".repeat(140_000);
    let mut bad = s.clone();
    install(&mut bad, &large);
    bad["capability"]["state"]["body"]["projection"]["entries"][0]["attributes"]
        .as_array_mut()
        .unwrap()
        .push(json!({"name":"title","value":"y".repeat(130_000)}));
    assert!(bad.to_string().len() < cem_ml::checkpoint_runtime_state::MAX_BYTES);
    assert!(validate(&bad.to_string()).is_err());
    let mut bad = s.clone();
    bad["capability"]["state"]["body"]["projection"]["nativeArtifacts"]
        .as_array_mut()
        .unwrap()
        .push(p["nativeArtifacts"][0].clone());
    assert!(validate(&bad.to_string()).is_err());
    let mut corrupt = decoded;
    *corrupt.last_mut().unwrap() ^= 1;
    let mut bad = s;
    bad["capability"]["state"]["body"]["projection"]["nativeArtifacts"][0] = json!({"contentHash":cem_ml::content_cache::ContentHash::from_blake3(&corrupt).header_value(),"base64":STANDARD.encode(corrupt)});
    assert!(validate(&bad.to_string()).is_err());
}

fn native_inputs_state() -> Value {
    let p = native_projection_state();
    let artifact = p["capability"]["state"]["body"]["projection"]["nativeArtifacts"][0].clone();
    let mut s = state();
    s["capability"] = json!({"kind":"native-surface","phase":"open","defaultsInitialized":true,"state":{"returnValue":"","modal":false,"focusWithin":true,"closeReason":"native","disposition":"reacquire",
        "invocation":{"source":null,"returnDestination":null,"logicalParent":null,"command":"edit","contextKey":"row:1","inputKind":"programmatic","pointer":null,"selection":null,"context":{"artifact":artifact,"index":0}}}});
    s["references"] = json!({"version":"cem-native-source/1","revision":"prepare:1","rowCount":0,"select":"input.children","resolve":true,"requesting":0,
        "sources":[{"codec":"xml","base64":STANDARD.encode("<option value='same'/>"),"sourceUri":"memory:options.xml","primarySourceId":1,"context":true}],
        "bindings":[],"crossings":[],"labels":{"option":null,"group":null},"disposition":"reacquire",
        "preparation":{"dataJson":"{\"datadom\":{\"slices\":{\"query\":\"é😀\",\"enabled\":true}}}","limits":{"maxBytes":16777216,"maxValues":100000,"maxDepth":128},
        "nativeBindings":{"version":"cem-native-bindings/1","artifacts":[artifact],"bindings":[
            {"target":"attribute","name":"native-options","artifact":0,"index":0},
            {"target":"slice","name":"options","artifact":0,"index":0,"attribute":"native-options"},
            {"target":"slice","name":"raw","artifact":0,"index":0}]}}});
    s
}
#[test]
fn native_context_and_preparation_inputs_preserve_original_capsules_and_control_frames() {
    let s = native_inputs_state();
    let normalized = validate(&s.to_string()).unwrap();
    let saved: Value = serde_json::from_str(&normalized).unwrap();
    assert_eq!(saved["capability"], s["capability"]);
    assert_eq!(
        saved["references"]["preparation"],
        s["references"]["preparation"]
    );
    assert_eq!(validate(&normalized).unwrap(), normalized);
    let original = saved["references"]["preparation"]["dataJson"]
        .as_str()
        .unwrap();
    let mut documents = cem_ml::import::documents::CemDocuments::default();
    documents
        .retain(
            original.as_bytes(),
            "application/json",
            "memory:checkpoint-preparation.json",
        )
        .unwrap();
}
#[test]
fn native_context_and_preparation_inputs_refuse_incomplete_or_authoritative_records() {
    let s = native_inputs_state();
    for (path, value) in [
        ("/capability/state/invocation/context/index", json!(99)),
        (
            "/capability/state/invocation/context/artifact/contentHash",
            json!("forged"),
        ),
        ("/capability/state/invocation/context/handle", json!(7)),
        ("/references/preparation/dataJson", json!("[]")),
        ("/references/preparation/dataJson", json!("{invalid")),
        ("/references/preparation/limits/maxDepth", json!(0)),
        ("/references/preparation/limits/maxBytes", json!(16777217)),
        (
            "/references/preparation/nativeBindings/bindings/0/name",
            json!("wrong"),
        ),
        (
            "/references/preparation/nativeBindings/bindings/0/attribute",
            json!("native-options"),
        ),
        (
            "/references/preparation/nativeBindings/bindings/1/attribute",
            json!("wrong"),
        ),
        (
            "/references/preparation/nativeBindings/bindings/1/target",
            json!("live"),
        ),
        (
            "/references/preparation/nativeBindings/bindings/1/artifact",
            json!(1),
        ),
        ("/references/preparation/nativeBindings/grants", json!([])),
    ] {
        let mut bad = s.clone();
        // Insert both existing and forbidden fields through their parent object.
        let (parent, key) = path.rsplit_once('/').unwrap();
        bad.pointer_mut(parent)
            .unwrap()
            .as_object_mut()
            .unwrap()
            .insert(key.into(), value);
        assert!(validate(&bad.to_string()).is_err(), "{path}");
    }
    let mut bad = s.clone();
    let b = bad["references"]["preparation"]["nativeBindings"]["bindings"][0].clone();
    bad["references"]["preparation"]["nativeBindings"]["bindings"]
        .as_array_mut()
        .unwrap()
        .push(b);
    assert!(validate(&bad.to_string()).is_err());
    let mut bad = s.clone();
    let a = bad["references"]["preparation"]["nativeBindings"]["artifacts"][0].clone();
    bad["references"]["preparation"]["nativeBindings"]["artifacts"]
        .as_array_mut()
        .unwrap()
        .push(a);
    assert!(validate(&bad.to_string()).is_err());
    let mut bad = s;
    bad["references"]["preparation"]["dataJson"] =
        json!(format!("{{\"data\":\"{}\"}}", "x".repeat(256 * 1024)));
    assert!(validate(&bad.to_string()).is_err());
}

#[test]
fn native_input_capsules_enforce_privacy_effective_limits_and_shared_resource_budget() {
    use cem_ml::value::artifact::{CemValueGraph, CemValueProvenance, CemValueRecord};
    let s = native_inputs_state();
    for key in ["maxBytes", "maxValues"] {
        let mut bad = s.clone();
        bad["references"]["preparation"]["limits"][key] = json!(1);
        assert!(validate(&bad.to_string()).is_err(), "{key}");
    }
    let mut graph = CemValueGraph {
        roots: vec![0],
        records: vec![
            CemValueRecord {
                kind: "attribute".into(),
                name: "type".into(),
                native_content: true,
                values: vec![1],
                ..Default::default()
            },
            CemValueRecord {
                lexical: "password".into(),
                ..Default::default()
            },
        ],
    };
    for credential_uri in [false, true] {
        if credential_uri {
            graph.records[0].name = "native-options".into();
            graph.records[1].lexical = "safe".into();
            graph.records[1].provenance = Some(CemValueProvenance {
                source_uri: Some("https://user:secret@example.test/source".into()),
                ..Default::default()
            });
        }
        let bytes = graph.encode(&Default::default()).unwrap();
        let artifact = json!({"base64":STANDARD.encode(&bytes),"contentHash":cem_ml::content_cache::ContentHash::from_blake3(&bytes).header_value()});
        for path in [
            "/capability/state/invocation/context/artifact",
            "/references/preparation/nativeBindings/artifacts/0",
        ] {
            let mut bad = s.clone();
            *bad.pointer_mut(path).unwrap() = artifact.clone();
            assert!(
                validate(&bad.to_string()).is_err(),
                "{path} credential={credential_uri}"
            );
        }
    }
    let mut aggregate = s;
    aggregate["references"]["preparation"]["dataJson"] =
        json!(format!("{{\"caption\":\"{}\"}}", "x".repeat(145_000)));
    aggregate["resources"] = json!([{ "name":"data", "kind":"http", "disposition":"reimport-and-read-again", "revision":1, "cursor":0, "live":false, "state":"loaded", "url":"https://example.test/source.xml", "method":"GET",
        "document":{"base64":STANDARD.encode(format!("<root>{}</root>","x".repeat(140_000))),"contentType":"application/xml","sourceUri":"https://example.test/source.xml","digest":null}}]);
    assert!(validate(&aggregate.to_string()).is_err());
}

#[test]
fn preparation_limits_cover_encoded_overhead_and_combined_native_bindings() {
    let mut s = native_inputs_state();
    s["capability"] = Value::Null;
    s["references"]["preparation"]["dataJson"] = json!("{}");
    let encoded = s["references"]["preparation"]["nativeBindings"]["artifacts"][0]["base64"]
        .as_str()
        .unwrap();
    let bytes = STANDARD.decode(encoded).unwrap();
    let graph =
        cem_ml::value::artifact::CemValueGraph::decode(&bytes, &Default::default()).unwrap();
    // A payload can fit its graph's logical content but exceed the encoded limit.
    let limits = cem_ml::value::artifact::CemValueArtifactLimits {
        max_bytes: bytes.len() - 1,
        max_values: 4096,
        max_depth: 64,
    };
    assert!(graph.validate(&limits).is_ok());
    s["references"]["preparation"]["limits"]["maxBytes"] = json!(bytes.len() - 1);
    assert!(validate(&s.to_string()).is_err());
    let mut second = graph;
    second.records[1].lexical = "43".into();
    let second_bytes = second.encode(&Default::default()).unwrap();
    let second_artifact = json!({"base64":STANDARD.encode(&second_bytes),"contentHash":cem_ml::content_cache::ContentHash::from_blake3(&second_bytes).header_value()});
    s["references"]["preparation"]["nativeBindings"]["artifacts"]
        .as_array_mut()
        .unwrap()
        .push(second_artifact);
    s["references"]["preparation"]["nativeBindings"]["bindings"][2]["artifact"] = json!(1);
    s["references"]["preparation"]["limits"]["maxBytes"] =
        json!(bytes.len() + second_bytes.len() - 1);
    assert!(validate(&s.to_string()).is_err());
    s["references"]["preparation"]["limits"]["maxBytes"] = json!(bytes.len() + second_bytes.len());
    assert!(validate(&s.to_string()).is_ok());
}

fn custom_source_policy_state() -> Value {
    use cem_ml::{
        ast::reload::{ReferenceReloadBundle, ReloadLimits, ReloadSource},
        import::import_bytes_with_lexical_scopes,
        schema::vocab::CompiledSchema,
        source::SourceId,
    };
    let text = "{option @value=same | Original}";
    let imported = import_bytes_with_lexical_scopes(
        text.as_bytes(),
        "text/cem-ml",
        "memory:policies.cem",
        CompiledSchema::cem_core(),
    )
    .unwrap();
    let bundle = ReferenceReloadBundle::export(
        &imported.captured,
        vec![ReloadSource::new(
            SourceId(1),
            "memory:policies.cem",
            text.as_bytes(),
            true,
        )],
        ReloadLimits::default(),
    )
    .unwrap()
    .encode(ReloadLimits::default())
    .unwrap();
    let mut s = native_inputs_state();
    s["references"]["sources"][0] = json!({"codec":"cemb","base64":STANDARD.encode(bundle),"sourceUri":null,"primarySourceId":1,"context":true,"maxDepth":128,"maxWork":100000,"unresolved":"warning"});
    s
}
#[test]
fn custom_source_policies_preserve_explicit_values_and_unresolved_dispositions() {
    let s = custom_source_policy_state();
    let saved: Value = serde_json::from_str(&validate(&s.to_string()).unwrap()).unwrap();
    for key in ["maxDepth", "maxWork", "unresolved"] {
        assert_eq!(
            saved["references"]["sources"][0][key],
            s["references"]["sources"][0][key]
        );
    }
    for disposition in ["mandatory", "warning", "ignore", "neutral"] {
        let mut explicit = s.clone();
        explicit["references"]["sources"][0]["maxDepth"] = json!(32);
        explicit["references"]["sources"][0]["maxWork"] = json!(700);
        explicit["references"]["sources"][0]["unresolved"] = json!(disposition);
        assert!(validate(&explicit.to_string()).is_ok());
    }
    let mut inherited = s;
    for key in ["maxDepth", "maxWork", "unresolved"] {
        inherited["references"]["sources"][0]
            .as_object_mut()
            .unwrap()
            .remove(key);
    }
    let normalized: Value =
        serde_json::from_str(&validate(&inherited.to_string()).unwrap()).unwrap();
    assert!(normalized["references"]["sources"][0]
        .get("maxDepth")
        .is_none());
}
#[test]
fn custom_source_policies_reject_invalid_bounds_unknown_authority_and_xml_overrides() {
    let s = custom_source_policy_state();
    for (key, value) in [
        ("maxDepth", json!(0)),
        ("maxWork", json!(-1)),
        ("maxDepth", json!(1.5)),
        ("maxWork", json!(4294967296u64)),
        ("unresolved", json!("allow")),
        ("grant", json!(true)),
    ] {
        let mut bad = s.clone();
        bad["references"]["sources"][0][key] = value;
        assert!(validate(&bad.to_string()).is_err(), "{key}");
    }
    let mut xml = native_inputs_state();
    xml["references"]["sources"][0]["maxDepth"] = json!(32);
    assert!(validate(&xml.to_string()).is_err());
}

#[test]
fn opaque_owner_islands_import_into_independent_native_tables_without_writer_execution() {
    let island = br#"<template data-cem-instance-data="owner"><cem-context xmlns="urn:cem:context" version="1"><data-slices><slice name="draft">retained</slice></data-slices><resource-bindings><cem-location href="https://example.test/next"></cem-location></resource-bindings></cem-context></template>"#;
    let mut original = cem_ml::import::documents::CemDocuments::default();
    let old = original
        .retain(island, "text/html", "https://example.test/owner")
        .unwrap();
    assert!(original.dispose(old));
    let mut fresh = cem_ml::import::documents::CemDocuments::default();
    let id = fresh
        .retain(island, "text/html", "https://example.test/owner")
        .unwrap();
    let node = cem_ml::validation::xpath::XPathNativeNode::cem_document(fresh.get(id).unwrap());
    assert!(node.string_value().contains("retained"));
    assert!(fresh.dispose(id));
    assert!(fresh.get(id).is_none());
}

#[test]
fn validates_retained_action_disclosure_and_native_list_guards() {
    let endpoint = json!({"instanceId":"target","dataRevision":"2","nodeId":"dialog.1"});
    for capability in [
        json!({"kind":"action-command","controlNode":"button.1","command":null,"contextKey":null,"target":null,"descriptor":null,"disposition":"retain-without-invocation"}),
        json!({"kind":"action-command","controlNode":"button.1","command":"show-modal","contextKey":"project","target":endpoint,"descriptor":null,"disposition":"retain-without-invocation"}),
        json!({"kind":"action-control","controlNode":"link.1","submenu":null}),
        json!({"kind":"action-control","controlNode":"link.1","submenu":endpoint}),
        json!({"kind":"disclosure-panel","headingId":"heading","panelId":"panel","summaryId":"summary","headingLevel":"3","expanded":true}),
        json!({"kind":"native-list","selectable":true,"scrollTop":20.5,"scrollLeft":-2.0}),
        json!({"kind":"native-list","selectable":false,"scrollTop":0.0,"scrollLeft":0.0}),
    ] {
        let mut s = state(); s["capability"] = capability;
        let output: Value = serde_json::from_str(&validate(&s.to_string()).unwrap()).unwrap();
        assert_eq!(output, s);
        for key in ["nativeHandle", "callback", "replay"] {
            let mut bad = s.clone(); bad["capability"][key] = json!(1);
            assert!(validate(&bad.to_string()).is_err());
        }
    }
    for capability in [
        json!({"kind":"action-command","controlNode":"","command":null,"contextKey":null,"target":null,"descriptor":null,"disposition":"retain-without-invocation"}),
        json!({"kind":"action-command","controlNode":"button","command":"show-modal","contextKey":null,"target":null,"descriptor":null,"disposition":"retain-without-invocation"}),
        json!({"kind":"action-command","controlNode":"button","command":"execute","contextKey":null,"target":endpoint,"descriptor":null,"disposition":"retain-without-invocation"}),
        json!({"kind":"action-command","controlNode":"button","command":null,"contextKey":null,"target":null,"descriptor":endpoint,"disposition":"retain-without-invocation"}),
        json!({"kind":"action-command","controlNode":"button","command":"--action","contextKey":null,"target":{"instanceId":"","dataRevision":"2","nodeId":"host"},"descriptor":null,"disposition":"retain-without-invocation"}),
        json!({"kind":"action-control","controlNode":"link","submenu":{"instanceId":"peer","dataRevision":"","nodeId":"host"}}),
        json!({"kind":"disclosure-panel","headingId":"same","panelId":"same","summaryId":"summary","headingLevel":"3","expanded":false}),
        json!({"kind":"disclosure-panel","headingId":"heading","panelId":"panel","summaryId":"summary","headingLevel":"7","expanded":false}),
        json!({"kind":"native-list","selectable":true,"scrollTop":1000000001.0,"scrollLeft":0.0}),
    ] {
        let mut s = state(); s["capability"] = capability;
        assert!(validate(&s.to_string()).is_err(), "{s}");
    }
}

fn popup_policy_state() -> Value {
    let endpoint = json!({"instanceId":"trigger-owner","dataRevision":"2","nodeId":"button"});
    let mut s = state();
    s["capability"] = json!({"kind":"popup","open":true,"state":{
        "trigger":"button","panel":"aside","scrollTop":12,"scrollLeft":0,"disposition":"reacquire",
        "policies":{"externalTrigger":endpoint,"anchor":{"mode":"invoker","target":null},
            "boundary":{"mode":"viewport","target":null},"focusTarget":{"mode":"auto","target":null},
            "returnFocus":{"mode":"none","target":null},"anchorLost":"close","geometry":"recompute"}}});
    s
}
#[test]
fn popup_external_and_viewport_policies_round_trip() {
    let mut s = popup_policy_state();
    let normalized = validate(&s.to_string()).unwrap();
    assert_eq!(validate(&normalized).unwrap(), normalized);
    for role in ["anchor", "boundary", "focusTarget", "returnFocus"] {
        s["capability"]["state"]["policies"][role] = json!({"mode":"target","target":{"instanceId":"other","dataRevision":"3","nodeId":"node"}});
        assert!(validate(&s.to_string()).is_ok());
    }
    s["capability"]["state"]["policies"]["externalTrigger"] = Value::Null;
    assert!(validate(&s.to_string()).is_ok());
}
#[test]
fn popup_policies_refuse_private_authority_and_incomplete_roles() {
    let s = popup_policy_state();
    for (key, value) in [
        (
            "externalTrigger",
            json!({"instanceId":"x","dataRevision":"","nodeId":"node"}),
        ),
        (
            "externalTrigger",
            json!({"instanceId":"x","dataRevision":"1","nodeId":"other"}),
        ),
        ("anchorLost", json!("restore")),
        ("geometry", json!("restore-pixels")),
        ("rect", json!({"left":1})),
        ("grant", json!("private")),
    ] {
        let mut bad = s.clone();
        bad["capability"]["state"]["policies"][key] = value;
        assert!(validate(&bad.to_string()).is_err(), "{key}");
    }
    for role in ["anchor", "boundary", "focusTarget", "returnFocus"] {
        for value in [
            json!({"mode":"target","target":null}),
            json!({"mode":"unknown","target":null}),
            json!({"mode":"auto","target":{"instanceId":"x","dataRevision":"1","nodeId":"node"}}),
        ] {
            let mut bad = s.clone();
            bad["capability"]["state"]["policies"][role] = value;
            assert!(validate(&bad.to_string()).is_err(), "{role}");
        }
        let mut bad = s.clone();
        bad["capability"]["state"]["policies"]
            .as_object_mut()
            .unwrap()
            .remove(role);
        assert!(validate(&bad.to_string()).is_err());
    }
}

#[test]
fn retained_media_preview_has_explicit_mode_and_no_serialized_controller_authority() {
    for mode in ["live", "artifact-tree"] {
        let mut s = state();
        s["capability"] = json!({"kind":"media-preview", "mode":mode, "disposition":"retain-controller"});
        assert!(validate(&s.to_string()).is_ok());
        for (key, value) in [("mode", json!("image")), ("disposition", json!("replay")), ("controller", json!("handle"))] {
            let mut invalid = s.clone(); invalid["capability"][key] = value;
            assert!(validate(&invalid.to_string()).is_err());
        }
    }
}

#[test]
fn static_preview_policies_do_not_grant_controller_authority() {
    for mode in ["document", "text", "media", "message"] {
        let mut s = state();
        s["capability"] = json!({"kind":"media-preview", "mode":mode, "disposition":"retain-presentation","revision":1});
        assert!(validate(&s.to_string()).is_ok());
        for revision in [json!(null), json!(9007199254740992_u64)] {
            let mut bad = s.clone(); bad["capability"]["revision"] = revision;
            assert!(validate(&bad.to_string()).is_err());
        }
        s["capability"]["disposition"] = json!("retain-controller");
        assert!(validate(&s.to_string()).is_err());
    }
    for mode in ["live", "artifact-tree", "unknown"] {
        let mut s = state();
        s["capability"] = json!({"kind":"media-preview", "mode":mode, "disposition":"retain-presentation","revision":1});
        assert!(validate(&s.to_string()).is_err());
    }
}
#[test]
fn resource_map_http_sources_need_explicit_policy_and_safe_urls() {
    let mut s = state();
    s["resources"] = json!([{"name":"remote", "kind":"http", "disposition":"reimport-and-read-again",
        "revision":1,"cursor":0,"live":false,"state":"loaded","method":"GET","url":"studio://example/data.json",
        "sourcePolicy":"retained-resource-map", "document":{"base64":"e30=","contentType":"application/json",
        "sourceUri":"studio://example/data.json","digest":null}}]);
    assert!(validate(&s.to_string()).is_ok());
    for policy in [json!(null), json!("unknown")] {
        let mut bad = s.clone(); bad["resources"][0]["sourcePolicy"] = policy;
        assert!(validate(&bad.to_string()).is_err());
    }
    for uri in ["javascript:alert(1)", "file:///private/data", "studio://user:secret@example/data", "data:application/json,e30="] {
        for field in ["url", "sourceUri"] {
            let mut bad = s.clone();
            if field == "url" { bad["resources"][0][field] = json!(uri); }
            else { bad["resources"][0]["document"][field] = json!(uri); }
            assert!(validate(&bad.to_string()).is_err(), "{field}: {uri}");
        }
    }
}

#[test]
fn credential_metadata_cannot_hide_in_retained_projections_or_native_inputs() {
    use cem_ml::value::artifact::{CemValueGraph, CemValueRecord};
    for (name, value) in [
        ("type", "PASSWORD"),
        ("type", "file"),
        ("autocomplete", "section-login CURRENT-PASSWORD"),
        ("autocomplete", "shipping new-password"),
        ("autocomplete", "section-login\tone-time-code"),
    ] {
        let mut projection = native_projection_state();
        projection["capability"]["state"]["body"]["projection"]["entries"][0]["attributes"]
            .as_array_mut()
            .unwrap()
            .push(json!({"name":name,"value":value}));
        assert_eq!(
            validate(&projection.to_string()).unwrap_err(),
            "cem.checkpoint.runtime.projection"
        );
        let graph = CemValueGraph {
            roots: vec![0],
            records: vec![
                CemValueRecord {
                    kind: "attribute".into(),
                    name: name.into(),
                    native_content: true,
                    values: vec![1],
                    ..Default::default()
                },
                CemValueRecord {
                    lexical: value.into(),
                    ..Default::default()
                },
            ],
        };
        let bytes = graph.encode(&Default::default()).unwrap();
        let artifact = json!({"base64":STANDARD.encode(&bytes),"contentHash":cem_ml::content_cache::ContentHash::from_blake3(&bytes).header_value()});
        for path in [
            "/capability/state/invocation/context/artifact",
            "/references/preparation/nativeBindings/artifacts/0",
        ] {
            let mut input = native_inputs_state();
            *input.pointer_mut(path).unwrap() = artifact.clone();
            assert_eq!(
                validate(&input.to_string()).unwrap_err(),
                "cem.checkpoint.runtime.projection.native"
            );
        }
    }
}

#[test]
fn form_value_request_preserves_consumption_without_replaying_a_value() {
    let mut input = state();
    input["capability"]["authoredValue"] = "same default".into();
    input["capability"]["valueRequest"] = "graph-parameter-context:3".into();
    let normalized = validate(&input.to_string()).unwrap();
    assert_eq!(serde_json::from_str::<Value>(&normalized).unwrap(), input);
    assert_eq!(validate(&normalized).unwrap(), normalized);
}

#[test]
fn form_value_request_keeps_older_checkpoints_unchanged_and_omits_absence() {
    let legacy = state();
    let normalized = validate(&legacy.to_string()).unwrap();
    assert_eq!(serde_json::from_str::<Value>(&normalized).unwrap(), legacy);
    assert!(
        serde_json::from_str::<Value>(&normalized).unwrap()["capability"]
            .get("valueRequest")
            .is_none()
    );
    let mut explicit_absence = legacy.clone();
    explicit_absence["capability"]["valueRequest"] = Value::Null;
    assert_eq!(
        serde_json::from_str::<Value>(&validate(&explicit_absence.to_string()).unwrap()).unwrap(),
        legacy
    );
}

#[test]
fn form_value_request_refuses_non_scalar_and_repeated_consumption_fields() {
    for request in [
        json!(false),
        json!(1),
        json!([]),
        json!({}),
        json!(["request:1"]),
    ] {
        let mut input = state();
        input["capability"]["valueRequest"] = request;
        assert_eq!(
            validate(&input.to_string()).unwrap_err(),
            "cem.checkpoint.runtime.format"
        );
    }
    let repeated = state().to_string().replace(
        "\"kind\":\"form-control\"",
        "\"kind\":\"form-control\",\"valueRequest\":\"first\",\"valueRequest\":\"second\"",
    );
    assert_eq!(
        validate(&repeated).unwrap_err(),
        "cem.checkpoint.runtime.format"
    );
}
