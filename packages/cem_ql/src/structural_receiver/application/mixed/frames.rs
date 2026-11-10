//! Per-render consumption of original mixed receiver values.
use super::{topology::FrameElement, *};
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
    pub(crate) receiver: Option<Item>,
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
    pub(crate) fn produce_typed_frame_value(
        &self,
        host: &mut FreshStructuralHost,
        root: &RetainedCemNode,
        field: &str,
        values: Vec<Item>,
        control: &OperationControl,
    ) -> Result<(AstNodeId, AttributeDatatypeContextLease, ExternalTypedValue), String> {
        let element = self.check_topology(host, root, Some(control), false)?;
        let attribute = *element.attributes.get(field).ok_or(REFUSED)?;
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
        typed: BTreeMap<AstNodeId, ExternalTypedValue>,
        control: &OperationControl,
    ) -> Result<ReceiverFrame<'a>, String> {
        let topology = self.check_frame_input(host, root, Some(control))?;
        let limits = crate::instance_island::LIMITS;
        let mut budget = Consumption {
            typed,
            control,
            remaining_values: limits.max_values,
            remaining_bytes: limits.max_bytes,
        };
        let mut values = self.consume_fields(host, root, &topology, &mut budget)?;
        let model = self.group.model.element(&topology.name).ok_or(REFUSED)?;
        let receiver = if !model.child_elements.is_empty()
            || model.allow_any_child
            || !model.field_contracts.is_empty()
        {
            budget.claim_values(1)?;
            let children = self.consume_children(host, root, &topology, &mut budget)?;
            Some(frame_record(&topology.name, values.clone(), children))
        } else {
            None
        };
        if !budget.typed.is_empty() {
            return Err(REFUSED.into());
        }
        for name in self.group.receivers.keys() {
            values.entry(name.clone()).or_default();
        }
        self.check(host)?;
        Ok(ReceiverFrame {
            owner: self,
            values,
            receiver,
        })
    }

    fn consume_children(
        &self,
        host: &mut FreshStructuralHost,
        root: &RetainedCemNode,
        element: &FrameElement,
        budget: &mut Consumption<'_>,
    ) -> Result<Vec<Item>, String> {
        element
            .children
            .iter()
            .map(|child| {
                budget.check()?;
                budget.claim_values(1)?;
                let fields = self.consume_fields(host, root, child, budget)?;
                let children = self.consume_children(host, root, child, budget)?;
                Ok(frame_record(&child.name, fields, children))
            })
            .collect()
    }

    fn consume_fields(
        &self,
        host: &mut FreshStructuralHost,
        root: &RetainedCemNode,
        element: &FrameElement,
        budget: &mut Consumption<'_>,
    ) -> Result<BTreeMap<String, ItemStream>, String> {
        let mut values = BTreeMap::new();
        for (name, id) in &element.attributes {
            budget.check()?;
            let binding = self
                .group
                .receivers
                .get(name)
                .map(|receiver| &receiver.binding);
            let input = ReceiverInput {
                source: root.owner().clone(),
                attribute: *id,
            };
            let items = match binding {
                Some(Binding::Node { .. }) => {
                    let report = self.validate_node(host, name, &input)?;
                    if !report.complete || report.failed {
                        return Err(REFUSED.into());
                    }
                    let mut items = vec![];
                    if report.targets.len() > budget.remaining_values {
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
                Some(Binding::Scalar { .. }) | None => {
                    let report = if binding.is_some() {
                        self.validate_scalar(host, name, &input)?
                    } else {
                        self.validate_nested_scalar(host, name, &input)?
                    };
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
                Some(Binding::Executable { .. }) => {
                    let values = if let Some(value) = budget.typed.remove(id) {
                        if value.byte_len() > budget.remaining_bytes {
                            return Err(REFUSED.into());
                        }
                        self.validate_frame_typed(
                            host,
                            name,
                            &input,
                            &value,
                            budget.remaining_values,
                        )?;
                        value.values().to_vec()
                    } else {
                        let invocation = self.preparation_invocation(
                            host,
                            name,
                            input,
                            PreparationLimits {
                                max_lexical_bytes: budget.remaining_bytes,
                                max_output_values: budget.remaining_values,
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
            budget.claim_values(items.items.len())?;
            for value in &items.items {
                if let Some(
                    crate::eval::AtomValue::String(text)
                    | crate::eval::AtomValue::Decimal(text)
                    | crate::eval::AtomValue::AnyUri(text),
                ) = value.atom()
                {
                    budget.remaining_bytes = budget
                        .remaining_bytes
                        .checked_sub(text.len())
                        .ok_or(REFUSED)?;
                }
            }
            budget.check()?;
            values.insert(name.clone(), items);
        }
        Ok(values)
    }
}

struct Consumption<'a> {
    typed: BTreeMap<AstNodeId, ExternalTypedValue>,
    control: &'a OperationControl,
    remaining_values: usize,
    remaining_bytes: usize,
}

impl Consumption<'_> {
    fn check(&self) -> Result<(), String> {
        self.control
            .check_scope(ROOT_EXECUTION_SCOPE_ID)
            .map_err(|_| REFUSED.into())
    }
    fn claim_values(&mut self, count: usize) -> Result<(), String> {
        self.remaining_values = self.remaining_values.checked_sub(count).ok_or(REFUSED)?;
        Ok(())
    }
}

fn frame_record(name: &str, attributes: BTreeMap<String, ItemStream>, children: Vec<Item>) -> Item {
    Item::Record(BTreeMap::from([
        (
            "name".into(),
            vec![Item::Atomic(crate::eval::AtomValue::String(name.into()))],
        ),
        (
            "attributes".into(),
            vec![Item::Record(
                attributes
                    .into_iter()
                    .map(|(name, value)| (name, value.items))
                    .collect(),
            )],
        ),
        ("children".into(), children),
    ]))
}
