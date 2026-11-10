//! Build/test embedding, never a library deployment constructor. WASM restores
//! original native snapshots; source authoring is native-only.
use cem_ml::{
    operation_control::OperationControl,
    parser::{tree::RetainedCemTree, CemAstNode},
    runtime_admission::generation::NativeGeneration,
    schema::{
        native_source::{dependencies::NativeDependencySourceSnapshot, SourceIdentity},
        reference_policy::ReferenceScopePolicy,
    },
};
use cem_ql::{
    api::{StandaloneExpressionBinding, StandaloneExpressionContext},
    application_envelope::{self as app, mixed, v3, v4},
    application_source_endpoint::{ApplicationBootstrap, PreparedApplication},
    application_source_transport::{self as transport, SourceInput, VerifiedApplicationSources},
    eval::{ItemStream, RetainedCemNode},
    structural_receiver::{
        application::{self, publication},
        FreshStructuralHost,
    },
};
use std::{
    collections::{BTreeMap, BTreeSet},
    sync::Arc,
};
pub const RECEIVER_URI: &str = "https://runtime.test/receivers/1";
pub const SCALAR_URI: &str = "https://runtime.test/scalars/1";
pub const RECEIVER:&str="{schema @name=receivers @namespace=https://runtime.test/receivers/1 | {elements | {element @name=view @optional-attributes=\"items count choice\"}} {attributes | {attribute @name=items @type=node @maxItems=2} {attribute @name=count @type=integer @minInclusive=3 @maxInclusive=4} {attribute @name=choice @type={#scalar}}}}";
pub const SCALARS:&str="{schema @name=scalars @namespace=https://runtime.test/scalars/1 | {attributes | {attribute @name=cart-count @type=integer @minInclusive=2 @maxInclusive=5}}}";
pub const VIEW: &str = "{view @items={#data} @count=3 @choice=ok}";
pub const CART: &str = "{cart @title=Tea | {item @quantity=3}}";
const COUNT_BODY:&str="{attribute @name=count @type=cart-count}{p | {$count}/{$datadom.dataset.fruitName}/{$datadom.slices.cart.title}{$datadom.slices.cart.children.children.children.children.value}/{$instanceID}}{section | {$datadom.slots.heading}}";
pub fn hash(b: &[u8]) -> String {
    cem_ml::content_cache::ContentHash::from_blake3(b).header_value()
}
pub fn elements(t: &RetainedCemTree, kind: &str) -> BTreeMap<String, u32> {
    t.ast()
        .nodes
        .iter()
        .filter_map(|n| match n {
            CemAstNode::Element {
                node_id,
                expanded_name,
                attributes,
                ..
            } if expanded_name.local_name == kind => {
                attributes.iter().find_map(|id| match t.ast().get(*id) {
                    Some(CemAstNode::Attribute {
                        expanded_name,
                        value: Some(name),
                        ..
                    }) if expanded_name.local_name == "name" => Some((name.clone(), *node_id)),
                    _ => None,
                })
            }
            _ => None,
        })
        .collect()
}
pub fn attributes(t: &RetainedCemTree) -> BTreeMap<String, u32> {
    t.ast()
        .nodes
        .iter()
        .filter_map(|n| match n {
            CemAstNode::Attribute {
                node_id,
                expanded_name,
                ..
            } => Some((expanded_name.local_name.clone(), *node_id)),
            _ => None,
        })
        .collect()
}
#[derive(serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct Record {
    identity: SourceIdentity,
    snapshot: NativeDependencySourceSnapshot,
}
pub struct Inputs {
    pub view: SourceInput,
    pub cart: SourceInput,
}
impl Inputs {
    #[cfg(target_arch = "wasm32")]
    pub fn from_receiver_checkpoint(
        checkpoint: &cem_ql::application_source_endpoint::VerifiedReceiverCheckpoint,
    ) -> Result<Arc<Self>, String> {
        let source = |uri| {
            checkpoint.sources().find(|source| source.identity.uri == uri)
                .map(|source| SourceInput {
                    identity: source.identity.clone(),
                    tree: source.tree.clone(),
                    capture: source.capture.clone(),
                })
                .ok_or_else(|| "fixture receiver source".to_string())
        };
        Ok(Arc::new(Self {
            view: source("file:///runtime/view.cem")?,
            cart: source("file:///runtime/cart-data.cem")?,
        }))
    }

    pub fn root(&self) -> RetainedCemNode {
        let id = self
            .view
            .tree
            .ast()
            .nodes
            .iter()
            .find_map(|n| match n {
                CemAstNode::Element { node_id, .. } => Some(*node_id),
                _ => None,
            })
            .unwrap();
        RetainedCemNode::new(self.view.tree.clone(), id).unwrap()
    }
    pub fn restore(bytes: &[u8], control: &OperationControl) -> Result<Arc<Self>, String> {
        if bytes.len() > 65536 {
            return Err("fixture input limit".into());
        }
        let mut poll = || {
            control
                .check_scope(cem_ml::operation_control::ROOT_EXECUTION_SCOPE_ID)
                .map_err(|e| e.to_string())
        };
        cem_ml::native_binary::preflight_messagepack(bytes, 100000, 64, &mut poll)?;
        let records: Vec<Record> = rmp_serde::from_slice(bytes).map_err(|e| e.to_string())?;
        if records.len() != 2
            || records
                .iter()
                .map(|r| r.snapshot.node_count())
                .sum::<usize>()
                > 4096
        {
            return Err("fixture input count".into());
        }
        let mut values = vec![];
        for r in records {
            let capture = r.snapshot.restore(&r.identity, &mut poll)?;
            let tree = RetainedCemTree::from_shared(
                capture.document().clone(),
                &r.identity.uri,
                "",
                Default::default(),
                None,
            )
            .map_err(|e| format!("{e:?}"))?;
            values.push(SourceInput {
                identity: r.identity,
                tree,
                capture,
            });
        }
        let cart = values.pop().unwrap();
        let view = values.pop().unwrap();
        Ok(Arc::new(Self { view, cart }))
    }
}
pub struct Bootstrap {
    pub inputs: Arc<Inputs>,
}
#[derive(Debug)]
struct Preparer;
impl cem_ql::datatype_preparation::NativeLexicalPreparer for Preparer {
    fn prepare(
        &self,
        call: cem_ql::datatype_preparation::PreparationCall<'_>,
    ) -> cem_ql::datatype_preparation::PreparationExecution {
        use cem_ql::{
            datatype_preparation::PreparationExecution,
            eval::{AtomValue, Item},
        };
        if call.lexical.text.len() > 1024 {
            return PreparationExecution::Rejected(vec![]);
        }
        PreparationExecution::Prepared {
            value: vec![Item::Atomic(AtomValue::String(
                call.lexical.text.to_string(),
            ))],
            diagnostics: vec![],
        }
    }
}
#[cfg(not(target_arch = "wasm32"))]
fn preparation_profile(
    module: &mut cem_ql::datatype_module_capsule::v2::ModuleSource,
    root: u32,
) -> cem_ql::datatype_module_capsule::v2::ProfileArtifact {
    use cem_ml::schema::{
        datatype_registry::{DatatypeKind, DatatypeRegistry},
        datatype_validation::{CandidateRequirement, ScalarRepresentation, ValueRepresentation},
        declaration_references::SchemaDeclarationNode,
    };
    use cem_ql::{
        datatype_module_capsule::v2 as assembly,
        datatype_preparation::PreparationSignature,
        native_registrations::{
            dependency_sources, profiles::v2 as profiles, profiles::OperationRole,
            ImplementationIdentity, OperationMode, PreparationMode,
        },
    };
    let capture = &module.types.capture;
    let schema = capture
        .document()
        .nodes
        .iter()
        .find_map(|n| match n {
            CemAstNode::Element {
                node_id,
                expanded_name,
                ..
            } if expanded_name.local_name == "schema" => Some(*node_id),
            _ => None,
        })
        .unwrap();
    let schema = SchemaDeclarationNode::new(capture.document().clone(), schema).unwrap();
    let mut registry = DatatypeRegistry::default();
    registry
        .insert(
            schema.clone(),
            SchemaDeclarationNode::new(capture.document().clone(), root).unwrap(),
        )
        .unwrap();
    let source = registry.source(&schema, "value").unwrap();
    let representation = ValueRepresentation::Scalar(ScalarRepresentation::String);
    let implementation = ImplementationIdentity::preparation(
        "fixture-original-string",
        hash(b"fixture-preparer-code/1"),
        "fixture-original-string",
        PreparationSignature {
            kind: DatatypeKind::Scalar,
            output: representation,
            candidate: CandidateRequirement::Optional,
        },
    );
    let dependency = dependency_sources::export_operation(
        capture,
        &source,
        OperationMode::Preparation(PreparationMode::Ready),
        module.types.identity.clone(),
        implementation,
        &hash(b"registry"),
        &Default::default(),
    )
    .unwrap();
    let profile = profiles::OperationProfile {
        source: module.types.identity.clone(),
        scope: source.scope().node_id(),
        declaration: root,
        kind: DatatypeKind::Scalar,
        representation,
        dependencies: BTreeMap::from([(OperationRole::Preparation, dependency.pin.clone())]),
    };
    let sources = vec![dependency];
    let built = profiles::export(
        &BTreeMap::from([("prepare".into(), profile)]),
        &sources,
        &hash(b"registry"),
        &Default::default(),
    )
    .unwrap();
    module.profiles.insert(
        root,
        assembly::ProfileSelection {
            pin: built.pin.clone(),
            id: "prepare".into(),
        },
    );
    assembly::ProfileArtifact {
        bytes: built.bytes,
        pin: built.pin,
        sources,
    }
}
pub fn configured(
    app: &Arc<v4::VerifiedApplicationEnvelope>,
    receiver: &application::publication::MixedSource,
    scalar: &SourceInput,
    inputs: &Inputs,
    g: &Arc<NativeGeneration>,
    control: OperationControl,
) -> Result<
    (
        FreshStructuralHost,
        Arc<publication::RegistryOwner>,
        BTreeMap<String, Arc<cem_ql::structural_receiver::FreshDependencyDatatypeCompilation>>,
        Vec<application::RetainedApplicationOwner>,
    ),
    String,
> {
    let root = &app.parent().roots()["value"];
    let graph = app
        .parent()
        .assembly_owner(&root.assembly)
        .ok_or("graph")?
        .clone();
    let module = graph.module("app").ok_or("module")?;
    let mut host =
        FreshStructuralHost::new_in_generation(&app.pin().registry_hash, control.clone(), g)?;
    let scope = host.register_datatype_assembly_scope(
        &graph,
        "app",
        Some(Default::default()),
        ReferenceScopePolicy::schema_defaults().map_err(|e| format!("{e:?}"))?,
    )?;
    let ctx = StandaloneExpressionContext::default().with_binding(
        "scalar",
        StandaloneExpressionBinding::any(ItemStream::once(
            RetainedCemNode::new(module.source().clone(), root.datatype)
                .ok_or("dtype")?
                .query_item(),
        )),
    );
    let from = host.register_captured_scope(
        receiver.receiver.source.clone(),
        receiver.receiver.capture.clone(),
        Some(ctx.clone()),
        ReferenceScopePolicy::schema_defaults().unwrap(),
    )?;
    host.prepare_captured_lexical_scopes(&receiver.receiver.capture, ctx)?;
    host.allow_scope_crossing(from, scope);
    for s in receiver.scalars.values() {
        if !Arc::ptr_eq(&s.source, &scalar.tree) {
            return Err("scalar source".into());
        }
        host.register_captured_scope(
            s.source.clone(),
            scalar.capture.clone(),
            Some(Default::default()),
            ReferenceScopePolicy::schema_defaults().unwrap(),
        )?;
    }
    let cart = inputs
        .cart
        .tree
        .ast()
        .nodes
        .iter()
        .find_map(|n| match n {
            CemAstNode::Element {
                node_id,
                expanded_name,
                ..
            } if expanded_name.local_name == "cart" => Some(*node_id),
            _ => None,
        })
        .ok_or("cart")?;
    let ctx = StandaloneExpressionContext::default().with_binding(
        "data",
        StandaloneExpressionBinding::any(ItemStream::once(
            RetainedCemNode::new(inputs.cart.tree.clone(), cart)
                .ok_or("cart")?
                .query_item(),
        )),
    );
    let from = host.register_captured_scope(
        inputs.view.tree.clone(),
        inputs.view.capture.clone(),
        Some(ctx.clone()),
        ReferenceScopePolicy::schema_defaults().unwrap(),
    )?;
    host.prepare_captured_lexical_scopes(&inputs.view.capture, ctx)?;
    let to = host.register_captured_scope(
        inputs.cart.tree.clone(),
        inputs.cart.capture.clone(),
        Some(Default::default()),
        ReferenceScopePolicy::schema_defaults().unwrap(),
    )?;
    host.allow_scope_crossing(from, to);
    let primitive = cem_ql::datatype_compilation::DatatypeImplementation {
        source: module
            .sources()
            .get(&root.datatype)
            .ok_or("primitive")?
            .clone(),
        kind: cem_ml::schema::datatype_registry::DatatypeKind::Scalar,
        representation: cem_ml::schema::datatype_validation::ValueRepresentation::Scalar(
            cem_ml::schema::datatype_validation::ScalarRepresentation::String,
        ),
        accepted_bases: vec![],
        bounds: Default::default(),
        tokenizer: cem_ql::datatype_compilation::TokenizerBinding::Absent,
        validator: None,
    };
    let mut operations = BTreeMap::new();
    let mut owners = vec![];
    for (datatype, profile) in module.profiles() {
        for (role, requirement) in profile.requirements() {
            if *role != cem_ql::native_registrations::profiles::OperationRole::Preparation {
                return Err("fixture operation role".into());
            }
            host.register_native_preparer(requirement.implementation().clone(), Preparer)
                .map_err(|e| format!("{e:?}"))?;
            let owner = Arc::new(
                host.link_dependency_operation(requirement.clone(), scope)
                    .map_err(|e| format!("{e:?}"))?,
            );
            operations.insert(
                cem_ql::structural_receiver::DatatypeAssemblyOperationKey {
                    module: "app".into(),
                    datatype: *datatype,
                    role: *role,
                },
                cem_ql::structural_receiver::DatatypeAssemblyOperationBinding::Operation(
                    owner.clone(),
                ),
            );
            owners.push(application::RetainedApplicationOwner::Operation(owner));
        }
    }
    let compiled = Arc::new(
        host.compile_datatype_assembly(
            graph,
            scope,
            &BTreeMap::new(),
            &[],
            &[primitive],
            &BTreeMap::new(),
            &operations,
            cem_ml::schema::reference_traversal::ReferenceTraversalLimits {
                max_depth: 64,
                max_work: 100000,
            },
        )
        .map_err(|e| e.to_string())?,
    );
    let registry = Arc::new(publication::link_registry(
        &host,
        app,
        app.registry_demand().registry(),
    )?);
    Ok((
        host,
        registry,
        BTreeMap::from([(root.assembly.clone(), compiled)]),
        owners,
    ))
}
impl ApplicationBootstrap for Bootstrap {
    fn prepare(
        &self,
        sources: Arc<VerifiedApplicationSources>,
        g: &Arc<NativeGeneration>,
        control: OperationControl,
    ) -> Result<PreparedApplication, String> {
        let (_, receivers) = sources.receivers()?;
        let receiver = receivers.get("cem-value").ok_or("receiver")?;
        let (host, registry, compilations, owners) = configured(
            sources.application(),
            receiver,
            sources
                .source("file:///runtime/scalars.cem")
                .ok_or("scalar source")?,
            &self.inputs,
            g,
            control,
        )?;
        PreparedApplication::stage(
            host,
            sources,
            registry,
            BTreeMap::new(),
            compilations,
            owners,
        )
    }
}

#[cfg(not(target_arch = "wasm32"))]
pub fn imported(text: &str, uri: &str) -> SourceInput {
    let i = cem_ml::import::import_bytes_with_lexical_scopes(
        text.as_bytes(),
        "text/cem-ml",
        uri,
        cem_ml::schema::vocab::CompiledSchema::cem_core(),
    )
    .unwrap();
    SourceInput {
        identity: SourceIdentity::new(uri, cem_ml::source::SourceId(1), text.as_bytes()),
        tree: i.tree,
        capture: i.captured,
    }
}
#[cfg(not(target_arch = "wasm32"))]
pub fn build() -> (transport::BuiltApplicationSources, Vec<u8>) {
    build_count(COUNT_BODY)
}
#[cfg(not(target_arch = "wasm32"))]
pub fn build_count(body: &str) -> (transport::BuiltApplicationSources, Vec<u8>) {
    build_options(body, false)
}
#[cfg(not(target_arch = "wasm32"))]
pub fn build_prepared() -> (transport::BuiltApplicationSources, Vec<u8>) {
    build_options(COUNT_BODY, true)
}
#[cfg(not(target_arch = "wasm32"))]
fn build_options(body: &str, prepared: bool) -> (transport::BuiltApplicationSources, Vec<u8>) {
    build_sources(body, prepared, VIEW, RECEIVER, CART)
}
#[cfg(not(target_arch = "wasm32"))]
pub fn prepared_sources(
    view: &str,
    receiver: &str,
    cart: &str,
) -> (transport::BuiltApplicationSources, Vec<u8>) {
    build_sources(COUNT_BODY, true, view, receiver, cart)
}
#[cfg(not(target_arch = "wasm32"))]
fn build_sources(
    body: &str,
    prepared: bool,
    view: &str,
    receiver: &str,
    cart: &str,
) -> (transport::BuiltApplicationSources, Vec<u8>) {
    build_sources_named(body, prepared, view, receiver, cart, false)
}
#[cfg(not(target_arch = "wasm32"))]
pub fn build_reserved() -> (transport::BuiltApplicationSources, Vec<u8>) {
    build_sources_named(COUNT_BODY, true, VIEW, RECEIVER, CART, true)
}
#[cfg(not(target_arch = "wasm32"))]
fn build_sources_named(
    body: &str,
    prepared: bool,
    view: &str,
    receiver: &str,
    cart: &str,
    reserved: bool,
) -> (transport::BuiltApplicationSources, Vec<u8>) {
    let view = if reserved {
        view.replace("@items=", "@datadom=")
    } else {
        view.into()
    };
    let receiver = if reserved {
        receiver
            .replace("@name=items", "@name=datadom")
            .replace("items count choice", "datadom count choice")
    } else {
        receiver.into()
    };
    let control = OperationControl::default();
    let view = imported(&view, "file:///runtime/view.cem");
    let cart = imported(cart, "file:///runtime/cart-data.cem");
    let inputs = Inputs { view, cart };
    let mut records = vec![];
    for s in [&inputs.view, &inputs.cart] {
        records.push(Record {
            identity: s.identity.clone(),
            snapshot: NativeDependencySourceSnapshot::capture(&s.capture, &s.identity, &mut || {
                Ok(())
            })
            .unwrap(),
        });
    }
    let input_bytes = rmp_serde::to_vec_named(&records).unwrap();
    let receiver = imported(&receiver, "file:///runtime/receiver.cem");
    let scalar = imported(SCALARS, "file:///runtime/scalars.cem");
    let schema = imported(
        include_str!("../../../cem_ml/fixtures/structural-cart.cem"),
        "file:///runtime/cart-schema.cem",
    );
    let contract = cem_ml::schema::structural_capsule::export(
        &schema.capture,
        schema.identity.clone(),
        "https://app.test/schema/cart/1",
        &hash(b"registry"),
        &control,
    )
    .unwrap();
    let node_pin = contract.pin.clone();
    let declaration = |tag: &str, body: &str| {
        let body = if reserved {
            body.replace("@name=items", "@name=datadom")
                .replace("{$items", "{$datadom")
        } else {
            body.into()
        };
        cem_ql::declaration_artifact::build_declaration_artifact(&format!("<cem-element tag=\"{tag}\"><template id=\"{tag}\" type=\"text/cem-ml\">{body}</template></cem-element>"),&format!("file:///runtime/{tag}.xhtml"),cem_ql::template_artifact::TemplateArtifactSourceMapMode::Prod,None).unwrap()
    };
    let mut base = app::EnvelopeBuilder::new(&hash(b"registry")).unwrap();
    let node = base
        .add_contract(app::ContractArtifact::structural(contract), BTreeSet::new())
        .unwrap();
    base.add_component(declaration("cem-app", "{p | Base}"), BTreeMap::new())
        .unwrap();
    let base = base
        .build(
            BTreeSet::from(["cem-app".into()]),
            BTreeSet::from([node]),
            &control,
        )
        .unwrap();
    let text="@ns s = https://cem.dev/ns/schema/1\n@default s\n{schema @namespace=urn:runtime:module | {types | {type @name=value @kind=scalar}}}";
    let module = imported(text, "file:///runtime/types.cem");
    let root = elements(&module.tree, "type")["value"];
    let mut module = cem_ql::datatype_module_capsule::v2::ModuleSource::new(
        cem_ql::datatype_module_capsule::ModuleSource {
            identity: module.identity,
            capture: module.capture,
            roots: BTreeSet::from([root]),
            bindings: BTreeMap::new(),
        },
    );
    let profiles = if prepared {
        vec![preparation_profile(&mut module, root)]
    } else {
        vec![]
    };
    let graph = cem_ql::datatype_module_capsule::v2::export(
        "app",
        &BTreeMap::from([("app".into(), module)]),
        &[],
        &profiles,
        &hash(b"registry"),
        &control,
    )
    .unwrap();
    let mut parent = v3::EnvelopeBuilder::new(base).unwrap();
    let id = parent.add_assembly(graph, vec![], profiles).unwrap();
    let parent = parent
        .build(
            BTreeMap::from([(
                "value".into(),
                v3::AssemblyRoot {
                    assembly: id,
                    datatype: root,
                },
            )]),
            &control,
        )
        .unwrap();
    let owner = Arc::new(v3::verify(&parent.bytes, &parent.pin, &control).unwrap());
    let model = cem_ml::schema::document_model::compile_schema_document_model_from_retained(
        SCALAR_URI,
        &scalar.tree,
    );
    let scalar_contract = cem_ql::application_declaration::NamedReceiverType {
        name: "cart-count".into(),
        schema_uri: SCALAR_URI.into(),
        source_uri: scalar.identity.uri.clone(),
        source_hash: scalar.identity.hash.clone(),
        value: cem_ml::schema::document_model::AttributeValueContract {
            model: model.attributes["cart-count"].clone(),
            ..Default::default()
        },
    };
    let mixed=mixed::build(declaration("cem-value","{attribute @name=items @type=cart-lines @maxItems=2}{attribute @name=count @type=cart-count}{attribute @name=choice @type=app-value}{cem-app}{p | Mixed}"),owner.clone(),BTreeMap::from([("cart-lines".into(),mixed::ReceiverContract::Node(node_pin.clone())),("cart-count".into(),mixed::ReceiverContract::Scalar(scalar_contract.clone())),("app-value".into(),mixed::ReceiverContract::Executable{root:"value".into(),target:owner.roots()["value"].clone()})]),&control).unwrap();
    let count = mixed::build(
        declaration("cem-count", body),
        owner.clone(),
        BTreeMap::from([(
            "cart-count".into(),
            mixed::ReceiverContract::Scalar(scalar_contract.clone()),
        )]),
        &control,
    )
    .unwrap();
    let frame=mixed::build(declaration("cem-receiver-frame","{attribute @name=items @type=cart-lines @maxItems=2}{attribute @name=count @type=cart-count}{attribute @name=choice @type=app-value}{p | {$items.attributes.value}/{$count}/{$choice}/{$instanceID}}"),owner.clone(),BTreeMap::from([("cart-lines".into(),mixed::ReceiverContract::Node(node_pin.clone())),("cart-count".into(),mixed::ReceiverContract::Scalar(scalar_contract)),("app-value".into(),mixed::ReceiverContract::Executable{root:"value".into(),target:owner.roots()["value"].clone()})]),&control).unwrap();
    let b = v4::build(
        parent,
        vec![mixed, count, frame],
        BTreeSet::from([
            "cem-value".into(),
            "cem-count".into(),
            "cem-receiver-frame".into(),
        ]),
        &control,
    )
    .unwrap();
    let app = Arc::new(v4::verify(&b.bytes, &b.pin, &control).unwrap());
    let scalar_source = || application::mixed::ScalarReceiverSource {
        identity: scalar.identity.clone(),
        source: scalar.tree.clone(),
        attributes: vec![elements(&scalar.tree, "attribute")["cart-count"]],
    };
    let source = |fields: BTreeMap<String, u32>| publication::MixedSource {
        receiver: application::mixed::MixedReceiverSource {
            schema_uri: RECEIVER_URI.into(),
            source: receiver.tree.clone(),
            capture: receiver.capture.clone(),
            attributes: fields,
        },
        scalars: BTreeMap::from([("cart-count".into(), scalar_source())]),
    };
    let mixed = source(elements(&receiver.tree, "attribute"));
    let frame = source(elements(&receiver.tree, "attribute"));
    let count = source(BTreeMap::from([(
        "count".into(),
        elements(&receiver.tree, "attribute")["count"],
    )]));
    let g = NativeGeneration::claim(Arc::new(
        cem_ml::runtime_admission::NativeAdmission::default(),
    ))
    .unwrap();
    let (mut host, registry, compilations, _owners) =
        configured(&app, &mixed, &scalar, &inputs, &g, control.clone()).unwrap();
    let linked = publication::stage(
        &mut host,
        app,
        &registry,
        BTreeMap::new(),
        BTreeMap::new(),
        BTreeMap::from([
            ("cem-value".into(), mixed),
            ("cem-count".into(), count),
            ("cem-receiver-frame".into(), frame),
        ]),
        &compilations,
    )
    .unwrap();
    let source = transport::capture(&host, &linked, &[receiver, scalar], &control).unwrap();
    (source, input_bytes)
}

pub fn validate(
    session: &mut cem_ql::application_source_endpoint::ApplicationSourceSession,
    inputs: &Inputs,
) -> Result<String, String> {
    session.with_application(|app, host| {
        let ids = attributes(&inputs.view.tree);
        let source = |name: &str| application::mixed::ReceiverInput {
            source: inputs.view.tree.clone(),
            attribute: ids[name],
        };
        let receivers = app.receivers(host, "cem-value")?;
        let node = receivers.validate_node(host, "items", &source("items"))?;
        let scalar = receivers.validate_scalar(host, "count", &source("count"))?;
        // The native embedding explicitly supplies an owned root invocation.
        // Raw typed receiver ingress remains refused without a typed profile.
        let executable = app.validate_root(
            host,
            "value",
            &cem_ql::datatype_validation::ValidationInput {
                value: vec![cem_ql::eval::Item::Atomic(cem_ql::eval::AtomValue::String(
                    "ok".into(),
                ))],
                candidate: vec![],
                fallback: Default::default(),
            },
            Default::default(),
        )?;
        Ok(format!(
            "{}:{}:{}",
            node.complete && !node.failed,
            scalar.accepted,
            executable.accepted == Some(true)
        ))
    })
}
