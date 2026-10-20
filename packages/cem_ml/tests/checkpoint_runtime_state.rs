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
