//! Interactive Chromium fixture. JSON carries one scalar predicate/reply at a
//! time; retained sources and joined evidence never leave this native process.
use cem_ml::{
    css_emission::CssManagedScope,
    css_import_snapshot::CssSourceSnapshot,
    engine::EngineContext,
    html_base::*,
    html_base_evidence::*,
    html_base_host::*,
    module_resolution::*,
    parser::tree::CemTreeNodeKind,
    scheduler::AbortSignal,
    stylesheet_execution::*,
    template_snapshot::*,
    worker_control::{WorkerCoordinator, WorkerSlotId},
};
use cem_ql::template_execution_record::*;
use serde_json::{json, Value};
use std::{
    collections::BTreeMap,
    io::{self, Write},
    sync::Arc,
};
const URI: &str = "https://output.test/docs/whole.html";
const BASE: &str = "https://output.test/assets/";
const TEMPLATE: &str = r#"{style |```p{color:rgb(1,2,3)}```}
{style @scope=widgets |```p{font-weight:700}```}
{style |```p{background-color:rgb(4,5,6)}```}
{result-element @name=html @namespace='http://www.w3.org/1999/xhtml' |
 {result-element @name=head @namespace='http://www.w3.org/1999/xhtml' |
  {result-element @name=base @namespace='http://www.w3.org/1999/xhtml' | {result-attribute @name=href | ../assets/}}}
 {result-element @name=body @namespace='http://www.w3.org/1999/xhtml' |
  {result-element @name=cem-card @namespace='http://www.w3.org/1999/xhtml' |
   {result-attribute @name=scope | widgets}
   {result-element @name=template @namespace='http://www.w3.org/1999/xhtml' | {result-attribute @name=data-cem-island | instance}}
   {result-element @name=p @namespace='http://www.w3.org/1999/xhtml' | Card}
   {result-element @name=cem-inner @namespace='http://www.w3.org/1999/xhtml' |
    {result-element @name=template @namespace='http://www.w3.org/1999/xhtml' | {result-attribute @name=data-cem-island | instance}}
    {result-element @name=p @namespace='http://www.w3.org/1999/xhtml' | Inner}}}
  {result-element @name=p @namespace='http://www.w3.org/1999/xhtml' | Outside}}}"#;
fn exchange(message: Value) -> Result<Value, String> {
    println!("{message}");
    io::stdout().flush().map_err(|e| e.to_string())?;
    let mut line = String::new();
    io::stdin()
        .read_line(&mut line)
        .map_err(|e| e.to_string())?;
    if line.len() > 4096 {
        return Err("bounded predicate reply required".into());
    }
    let reply: Value = serde_json::from_str(&line).map_err(|e| e.to_string())?;
    if reply.get("error").is_some() {
        return Err("browser predicate rejected".into());
    }
    Ok(reply)
}
fn slots(slot: HtmlBaseDomSlot<'_>) -> Result<u32, String> {
    let node = slot.node;
    let kind = match node.kind {
        CemTreeNodeKind::Document => 9,
        CemTreeNodeKind::Element => 1,
        CemTreeNodeKind::Attribute => 2,
        CemTreeNodeKind::Text => 3,
        CemTreeNodeKind::Comment => 8,
        _ => return Err("unsupported fixture node".into()),
    };
    let name = node.name.as_ref();
    let reply = exchange(
        json!({ "kind":"slot", "id":slot.id, "parent":node.parent, "index":slot.index, "nodeKind":kind,
        "namespace":name.map(|n| n.namespace_uri.as_str()).unwrap_or(""), "name":name.map(|n| n.local_name.as_str()).unwrap_or(""),
        "value":node.value, "children":node.children.len(), "attributes":node.attributes.len() }),
    )?;
    u32::try_from(reply.as_u64().ok_or("slot identity required")?)
        .map_err(|_| "slot identity bound exceeded".into())
}
fn host() -> HtmlBaseExecutionHost {
    HtmlBaseExecutionHost::new(
        HtmlBaseCompatibilityProfile::StandardAgreement,
        HtmlBaseBrowser {
            engine: "chromium".into(),
            version: CHROMIUM_BASE_PROFILE_VERSION.into(),
        },
        HtmlBaseLimits {
            nodes: 10000,
            url_bytes: 4096,
        },
    )
    .unwrap()
}
fn context() -> HtmlBaseContext {
    HtmlBaseContext {
        fallback: HtmlFallbackBase::DocumentUrl(URI.into()),
        encoding: HtmlBaseEncoding::Utf8,
        policy_stamp: "fixture:1".into(),
    }
}
struct Observer<'a>(&'a RecordedTemplateOutput);
impl StylesheetExecutionObserver for Observer<'_> {
    fn environment_revision(&mut self) -> Result<String, String> {
        exchange(json!({"kind":"revision"}))?
            .as_str()
            .map(str::to_owned)
            .ok_or("revision required".into())
    }
    fn observe(
        &mut self,
        slot: StylesheetExecutionSlot<'_>,
    ) -> Result<StylesheetObservation, String> {
        let StylesheetOwner::Document { owner, source_node } = slot.owner else {
            return Err("fixture document sheet required".into());
        };
        if owner != source_node || slot.parent_sheet.is_some() {
            return Err("fixture compiled root required".into());
        }
        let text = self
            .0
            .tree()
            .text_fragments(*source_node)
            .unwrap()
            .collect::<String>();
        let reply = exchange(json!({"kind":"sheet", "owner":owner, "text":text}))?;
        Ok(StylesheetObservation {
            selected: reply["selected"].as_bool().ok_or("selection required")?,
            conditions_match: true,
            loaded_source: reply["matches"]
                .as_bool()
                .ok_or("comparison required")?
                .then(|| slot.source_key.into()),
            installation: reply["installation"].as_u64(),
        })
    }
}
fn run() -> Result<(), String> {
    let snapshot = TemplateSourceSnapshot::new(
        vec![TemplateSnapshotSource {
            uri: "https://templates.test/root.cemt".into(),
            revision: "v1".into(),
            bytes: TEMPLATE.as_bytes().to_vec(),
            content_type: "text/cem-ml".into(),
        }],
        Default::default(),
    )?;
    let engine = Arc::new(EngineContext::default());
    let preflight = Arc::new(preflight_template_snapshot(
        snapshot.clone(),
        engine.clone(),
        "https://templates.test/root.cemt",
        8,
    )?);
    let handle = CemResolutionContextHandle::new("fixture");
    let cap = CemModuleUrlResolutionCapability::new(
        Arc::new(CemScopedModuleUrlResolver::new().with_context(
            handle.clone(),
            CemModuleUrlContext {
                identity: "fixture".into(),
                resolver_identity: "native".into(),
                resource_policy_stamp: "1".into(),
                frames: vec![CemModuleUrlFrame::new("fixture", URI)],
            },
        )),
        handle,
    );
    let execution =
        execute_snapshot_xhtml_scoped(preflight, &cap, &BTreeMap::new(), URI, Default::default())?;
    // First capture the actual original render before native stylesheet compilation.
    exchange(json!({"kind":"output", "html":execution.html_output()?.rendered}))?;
    let mut html = host();
    html.prepare(execution.tree().clone(), context(), slots, |_| {
        Ok(HtmlBasePolicyDecision::Allow)
    })?;
    let base = exchange(json!({"kind":"freeze"}))?
        .as_str()
        .ok_or("base required")?
        .to_owned();
    let base = html.confirm(base)?;
    let document = Arc::new(html.inspect_dependencies(&base, &cap, 10000, slots, BASE)?);
    let prepared = Arc::new(
        execution
            .prepare_dependencies(&mut html, document, &snapshot, &engine, &cap, slots, BASE)?,
    );
    let sources = CssSourceSnapshot::new(vec![], Default::default())?;
    let css = Arc::new(prepared.prepare_css_imports(
        &mut html,
        &execution,
        &snapshot,
        &engine,
        &cap,
        sources,
        Default::default(),
        slots,
        BASE,
    )?);
    let uri = execution.bodies().bodies[0].uri.clone();
    let choices = vec![
        BodyStylesheetInstallation {
            body_uri: uri.clone(),
            index: 0,
            scope: CssManagedScope::Private {
                tag: "cem-card".into(),
                context: None,
            },
        },
        BodyStylesheetInstallation {
            body_uri: uri.clone(),
            index: 1,
            scope: CssManagedScope::Shared {
                name: "widgets".into(),
                context: None,
            },
        },
        BodyStylesheetInstallation {
            body_uri: uri,
            index: 2,
            scope: CssManagedScope::Private {
                tag: "cem-inner".into(),
                context: None,
            },
        },
    ];
    let compiled = css.compile_installations(&choices, 100000)?;
    let head = (0..execution.tree().ast().nodes.len() as u32)
        .find(|id| {
            execution
                .tree()
                .node(*id)
                .is_some_and(|n| n.name.as_ref().is_some_and(|n| n.local_name == "head"))
        })
        .ok_or("head missing")?;
    let output = compiled.compose_output(&[head; 3], 10000, 100000)?;
    exchange(json!({"kind":"output", "html":output.html_output().rendered}))?;
    // New document/lifetime; retain all source owners locally across the delivery.
    let mut html = host();
    html.prepare(output.tree().clone(), context(), slots, |_| {
        Ok(HtmlBasePolicyDecision::Allow)
    })?;
    let base = exchange(json!({"kind":"freeze"}))?
        .as_str()
        .ok_or("base required")?
        .to_owned();
    let base = html.confirm(base)?;
    let document = Arc::new(html.inspect_dependencies(&base, &cap, 10000, slots, BASE)?);
    let mut coordinator = WorkerCoordinator::new(1).map_err(|e| e.to_string())?;
    let address = coordinator
        .worker(WorkerSlotId::from_raw(1))
        .map_err(|e| e.to_string())?;
    let mut worker = RecordedPreparationWorker::new(address, 1)?;
    let lifetime = worker.lifetime();
    let cancel = AbortSignal::new();
    let handle = worker.prepare(
        output.clone(),
        html,
        document,
        cancel.clone(),
        &mut Observer(&output),
        slots,
        BASE,
        Default::default(),
    )?;
    let blockers = worker.read(&handle, &mut Observer(&output), slots, BASE)?;
    let action = exchange(json!({"kind":"ready", "sheets":output.stylesheet_nodes().len()}))?;
    let expected_valid = action["expectedValid"]
        .as_bool()
        .ok_or("expected outcome required")?;
    let mode = action["mode"].as_str().ok_or("fixture mode required")?;
    match mode {
        "release" => worker.release(&handle)?,
        "cancel" => cancel.abort(),
        "close" => worker.close(),
        "generation" => {
            let replacement = coordinator
                .replace_worker(address.slot)
                .map_err(|e| e.to_string())?;
            worker.replace(replacement.replacement)?;
        }
        _ => {}
    }
    let valid = worker
        .read(
            &handle,
            &mut Observer(&output),
            |slot| {
                let identity = slots(slot)?;
                match mode {
                    "callback-cancel" => cancel.abort(),
                    "callback-close" => lifetime.close(),
                    "callback-release" => handle.release(),
                    _ => {}
                }
                Ok(identity)
            },
            BASE,
        )
        .is_ok();
    if valid != expected_valid {
        return Err(format!(
            "whole-output validation expected {expected_valid}, got {valid}"
        ));
    }
    if !valid {
        exchange(json!({"kind":"restore"}))?;
        if !handle.is_revoked() || !worker.is_empty() {
            return Err("failed preparation read retained a live handle".into());
        }
        if worker
            .read(&handle, &mut Observer(&output), slots, BASE)
            .is_ok()
        {
            return Err("restoration revived evidence".into());
        }
    }
    println!(
        "{}",
        json!({"kind":"result", "valid":valid, "sheets":3, "blockers":blockers, "generation":worker.address().generation.get()})
    );
    Ok(())
}
fn main() {
    if let Err(error) = run() {
        eprintln!("{error}");
        std::process::exit(1);
    }
}
