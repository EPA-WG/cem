//! Original flat-instance admission and per-render mixed receiver consumption.
use super::*;
use crate::{
    datatype_validation::ValidationRuntime,
    eval::{Item, ItemStream, RetainedCemNode},
    external_typed::{ExternalTypedProducer, ExternalTypedValue},
    render::{OriginalReceiverFrame, TemplateArtifact},
};

#[derive(Debug)]
struct SourceValue {
    source: RetainedCemNode,
    value: Item,
    ordinal: usize,
    datatype: Option<String>,
}
impl crate::eval::QueryItemView for SourceValue {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
    fn representation_id(&self) -> &'static str {
        "cem.original-receiver-value/1"
    }
    fn identity(&self) -> String {
        format!(
            "{}:{}",
            self.source.query_item().view().unwrap().identity(),
            self.ordinal
        )
    }
    fn kind(&self) -> crate::eval::QueryItemViewKind {
        crate::eval::QueryItemViewKind::Atomic
    }
    fn atom(&self) -> Option<crate::eval::AtomValue> {
        self.value.atom()
    }
    fn field(&self, name: &str) -> Option<Vec<Item>> {
        if name == "datatype" {
            self.datatype
                .as_ref()
                .map(|s| vec![Item::Atomic(crate::eval::AtomValue::String(s.clone()))])
        } else {
            self.value.view().and_then(|v| v.field(name))
        }
    }
    fn source_map(&self) -> Option<cem_ml::source_map::SourceMapStack> {
        self.source.query_item().source_map()
    }
    fn provenance(&self) -> Option<cem_ml::value::artifact::CemValueProvenance> {
        self.source.query_item().view()?.provenance()
    }
}
fn sourced(
    root: &RetainedCemNode,
    id: u32,
    value: Item,
    ordinal: usize,
    datatype: Option<String>,
) -> Result<Item, String> {
    if value.atom().is_none() {
        return Err(REFUSED.into());
    }
    Ok(Item::native(SourceValue {
        source: RetainedCemNode::new(root.owner().clone(), id).ok_or(REFUSED)?,
        value,
        ordinal,
        datatype,
    }))
}

pub(crate) struct ReceiverFrame<'a> {
    owner: &'a LinkedMixedReceivers,
    values: BTreeMap<String, ItemStream>,
}
impl OriginalReceiverFrame for ReceiverFrame<'_> {
    fn bindings(&self, artifact: &TemplateArtifact) -> Result<&BTreeMap<String, ItemStream>, ()> {
        if !std::ptr::eq(artifact, self.owner.group.contract.artifact())
            || !self.owner.group.active.load(Ordering::Acquire)
            || !self.owner.group.context.is_live()
        {
            return Err(());
        }
        Ok(&self.values)
    }
}
impl LinkedMixedReceivers {
    pub(crate) fn check_frame_input(
        &self,
        host: &mut FreshStructuralHost,
        root: &RetainedCemNode,
    ) -> Result<BTreeMap<String, AstNodeId>, String> {
        let _entry = self.group.enter(host)?;
        self.group.refresh(host)?;
        if self
            .group
            .receivers
            .keys()
            .any(|name| !crate::application_runtime::inputs::name_allowed(name))
        {
            return Err(REFUSED.into());
        }
        if host.registered_source_scope(root.owner()).is_none()
            || root.owner().ast().nodes.len() > 16_384
        {
            return Err(REFUSED.into());
        }
        let CemAstNode::Element {
            expanded_name,
            attributes,
            children,
            ..
        } = root.node()
        else {
            return Err(REFUSED.into());
        };
        let declaration =
            SchemaDeclarationNode::new(root.owner().ast_owner().clone(), root.node_id())
                .ok_or(REFUSED)?;
        if cem_ml::schema::declaration_references::SchemaDeclarationHost::input_expanded_name(
            &host.inner,
            &declaration,
        )
        .is_none_or(|name| !name.namespace_uri.is_empty())
        {
            return Err(REFUSED.into());
        }
        if !expanded_name.namespace_uri.is_empty() || attributes.len() > 64 {
            return Err(REFUSED.into());
        }
        let model = cem_ml::schema::document_model::compile_schema_document_model_from_retained(
            &self.group.schema_uri,
            &self.group.source,
        );
        let element = model
            .elements
            .get(&expanded_name.local_name)
            .ok_or(REFUSED)?;
        if model.elements.len() != 1
            || !element.child_elements.is_empty()
            || element.allow_any_child
            || !element.field_contracts.is_empty()
            || element
                .required_attributes
                .iter()
                .chain(&element.optional_attributes)
                .any(|n| !self.group.receivers.contains_key(n))
            || children.iter().any(|id| match root.owner().ast().get(*id) {
                Some(CemAstNode::Comment { .. }) => false,
                Some(CemAstNode::Text { data, .. } | CemAstNode::Whitespace { data, .. }) => {
                    !data.trim().is_empty()
                }
                _ => true,
            })
        {
            return Err(REFUSED.into());
        }
        let mut fields = BTreeMap::new();
        for id in attributes {
            let Some(CemAstNode::Attribute { expanded_name, .. }) = root.owner().ast().get(*id)
            else {
                return Err(REFUSED.into());
            };
            let name = &expanded_name.local_name;
            let declaration =
                SchemaDeclarationNode::new(root.owner().ast_owner().clone(), *id).ok_or(REFUSED)?;
            if cem_ml::schema::declaration_references::SchemaDeclarationHost::input_expanded_name(
                &host.inner,
                &declaration,
            )
            .is_none_or(|name| !name.namespace_uri.is_empty())
            {
                return Err(REFUSED.into());
            }
            if !expanded_name.namespace_uri.is_empty()
                || !(element.required_attributes.contains(name)
                    || element.optional_attributes.contains(name))
                || fields.insert(name.clone(), *id).is_some()
            {
                return Err(REFUSED.into());
            }
            values::input(
                host,
                name,
                &ReceiverInput {
                    source: root.owner().clone(),
                    attribute: *id,
                },
            )?;
        }
        if !element
            .required_attributes
            .iter()
            .all(|n| fields.contains_key(n))
        {
            return Err(REFUSED.into());
        }
        self.group.refresh(host)?;
        Ok(fields)
    }

    pub(crate) fn produce_typed_frame_value(
        &self,
        host: &mut FreshStructuralHost,
        root: &RetainedCemNode,
        field: &str,
        values: Vec<Item>,
        control: &OperationControl,
    ) -> Result<(AstNodeId, AttributeDatatypeContextLease, ExternalTypedValue), String> {
        let fields = self.check_frame_input(host, root)?;
        let attribute = *fields.get(field).ok_or(REFUSED)?;
        if !matches!(root.owner().ast().get(attribute),
            Some(CemAstNode::Attribute { value: None, value_nodes, .. }) if value_nodes.is_empty())
        {
            return Err(REFUSED.into());
        }
        let input = ReceiverInput {
            source: root.owner().clone(),
            attribute,
        };
        let receiver = self.group.receivers.get(field).ok_or(REFUSED)?;
        let Binding::Executable { root } = &receiver.binding else {
            return Err(REFUSED.into());
        };
        let facets = root
            .upgrade()
            .ok_or(REFUSED)?
            .bind_receiver(host, receiver.declaration.clone())?
            .compile_facets(&self.group.schema_uri, Default::default())
            .map_err(|_| REFUSED)?;
        let producer = ExternalTypedProducer::new(&facets, field).map_err(|_| REFUSED)?;
        let limits = crate::instance_island::LIMITS;
        let value = producer
            .produce(
                Some(values),
                &ValidationRuntime {
                    control,
                    scope: ROOT_EXECUTION_SCOPE_ID,
                    query: Default::default(),
                },
                PreparationLimits {
                    max_lexical_bytes: limits.max_bytes,
                    max_output_values: limits.max_values,
                    ..PreparationLimits::default()
                },
            )
            .map_err(|_| REFUSED)?;
        self.validate_frame_typed(host, field, &input, &value, limits.max_values)?;
        Ok((attribute, host.context.lease(), value))
    }

    fn validate_frame_typed(
        &self,
        host: &mut FreshStructuralHost,
        name: &str,
        input: &ReceiverInput,
        value: &ExternalTypedValue,
        max_input_values: usize,
    ) -> Result<(), String> {
        value.check_available().map_err(|_| REFUSED)?;
        let report = self.validate_executable_typed(
            host,
            name,
            input,
            value.values().to_vec(),
            ValidationLimits {
                max_input_values,
                ..ValidationLimits::default()
            },
        )?;
        if report.accepted != Some(true) {
            return Err(REFUSED.into());
        }
        value.check_available().map_err(|_| REFUSED.into())
    }

    pub(crate) fn consume_frame<'a>(
        &'a self,
        host: &mut FreshStructuralHost,
        root: &RetainedCemNode,
        mut typed: BTreeMap<AstNodeId, ExternalTypedValue>,
        control: &OperationControl,
    ) -> Result<ReceiverFrame<'a>, String> {
        let fields = self.check_frame_input(host, root)?;
        let mut values = BTreeMap::new();
        let mut remaining_values = crate::instance_island::LIMITS.max_values;
        let mut remaining_bytes = crate::instance_island::LIMITS.max_bytes;
        for (name, receiver) in &self.group.receivers {
            control
                .check_scope(ROOT_EXECUTION_SCOPE_ID)
                .map_err(|_| REFUSED)?;
            let Some(id) = fields.get(name) else {
                values.insert(name.clone(), ItemStream::empty());
                continue;
            };
            let input = ReceiverInput {
                source: root.owner().clone(),
                attribute: *id,
            };
            let items = match &receiver.binding {
                Binding::Node { .. } => {
                    let report = self.validate_node(host, name, &input)?;
                    if !report.complete || report.failed {
                        return Err(REFUSED.into());
                    }
                    let mut items = vec![];
                    if report.targets.len() > remaining_values {
                        return Err(REFUSED.into());
                    }
                    for target in report.targets {
                        let source = host.inner.source_tree(&target).ok_or(REFUSED)?.clone();
                        items.push(
                            RetainedCemNode::new(source, target.node_id())
                                .ok_or(REFUSED)?
                                .query_item(),
                        );
                    }
                    ItemStream::from_items(items)
                }
                Binding::Scalar { .. } => {
                    let report = self.validate_scalar(host, name, &input)?;
                    if !report.accepted {
                        return Err(REFUSED.into());
                    }
                    let converted = report.value.ok_or(REFUSED)?;
                    let datatype = converted.datatype.clone();
                    ItemStream::once(sourced(
                        root,
                        *id,
                        crate::typed_scalar::from_typed_value(
                            converted,
                            Some(values::input(host, name, &input)?.stack.clone()),
                        ),
                        0,
                        Some(datatype),
                    )?)
                }
                Binding::Executable { .. } => {
                    let values = if let Some(value) = typed.remove(id) {
                        if value.byte_len() > remaining_bytes {
                            return Err(REFUSED.into());
                        }
                        self.validate_frame_typed(host, name, &input, &value, remaining_values)?;
                        value.values().to_vec()
                    } else {
                        let invocation = self.preparation_invocation(
                            host,
                            name,
                            input,
                            PreparationLimits {
                                max_lexical_bytes: remaining_bytes,
                                max_output_values: remaining_values,
                                ..PreparationLimits::default()
                            },
                        )?;
                        let prepared = invocation.prepare(host)?;
                        if prepared.stopped.is_some() {
                            return Err(REFUSED.into());
                        }
                        let evidence = prepared.evidence.as_ref().ok_or(REFUSED)?;
                        invocation
                            .consume_frame(
                                host,
                                &ReceiverInput {
                                    source: root.owner().clone(),
                                    attribute: *id,
                                },
                                evidence,
                            )?
                            .items
                    };
                    ItemStream::from_items(
                        values
                            .into_iter()
                            .enumerate()
                            .map(|(n, v)| sourced(root, *id, v, n, None))
                            .collect::<Result<_, _>>()?,
                    )
                }
            };
            remaining_values = remaining_values
                .checked_sub(items.items.len())
                .ok_or(REFUSED)?;
            for value in &items.items {
                if let Some(
                    crate::eval::AtomValue::String(text)
                    | crate::eval::AtomValue::Decimal(text)
                    | crate::eval::AtomValue::AnyUri(text),
                ) = value.atom()
                {
                    remaining_bytes = remaining_bytes.checked_sub(text.len()).ok_or(REFUSED)?;
                }
            }
            control
                .check_scope(ROOT_EXECUTION_SCOPE_ID)
                .map_err(|_| REFUSED)?;
            values.insert(name.clone(), items);
        }
        if !typed.is_empty() {
            return Err(REFUSED.into());
        }
        self.check(host)?;
        Ok(ReceiverFrame {
            owner: self,
            values,
        })
    }
}
