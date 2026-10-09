//! Explicit datatype result consumption, independent of behavior execution.
//! A matching result never registers a rule or authorizes a schema @type slot.
use crate::eval::{self, AtomValue, Item, ItemStream, QueryItemViewKind};
use cem_ml::{
    diagnostics::{Diagnostic, Severity},
    schema::value_contracts::{ContractName, ContractValue, ValueContractError, ValueContracts},
    source_map::{FrameSpan, SourceMapStack},
};
use std::{cell::OnceCell, rc::Rc, sync::Arc};

/// One-read view of a native result: validation and decoding share the same
/// typed fields and scalar observations, while original handles stay retained.
#[derive(Clone)]
struct QueryValue {
    original: Item,
    atom: Rc<OnceCell<Option<AtomValue>>>,
    fields: Rc<OnceCell<Option<Vec<(String, Vec<QueryValue>)>>>>,
    native_kind: Option<QueryItemViewKind>,
}
impl QueryValue {
    fn new(original: Item) -> Self {
        let native_kind = original.view().map(|v| v.kind());
        Self {
            original,
            atom: Rc::new(OnceCell::new()),
            fields: Rc::new(OnceCell::new()),
            native_kind,
        }
    }
    fn read_atom(&self) -> Option<AtomValue> {
        if matches!(&self.original, Item::Atomic(_))
            || self.native_kind == Some(QueryItemViewKind::Atomic)
        {
            self.original.atom()
        } else {
            None
        }
    }
    fn read_fields(&self) -> Option<Vec<(String, Vec<Self>)>> {
        // Extra native diagnostic metadata stays on the retained value; these
        // are the logical fields checked by the schema-owned diagnostic shape.
        let fields = if let Some(diagnostic) = eval::native_diagnostic(&self.original) {
            vec![
                (
                    "code".into(),
                    vec![Item::Atomic(AtomValue::String(diagnostic.code.clone()))],
                ),
                ("severity".into(), self.original.view()?.field("severity")?),
                (
                    "message".into(),
                    vec![Item::Atomic(AtomValue::String(diagnostic.message.clone()))],
                ),
            ]
        } else {
            match &self.original {
                Item::Record(fields) => {
                    fields.iter().map(|(k, v)| (k.clone(), v.clone())).collect()
                }
                Item::Native(view) if self.native_kind == Some(QueryItemViewKind::Record) => {
                    view.fields()?
                }
                _ => return None,
            }
        };
        Some(
            fields
                .into_iter()
                .map(|(key, values)| (key, values.into_iter().map(Self::new).collect()))
                .collect(),
        )
    }
}
impl ContractValue for QueryValue {
    fn string(&self) -> Option<String> {
        match self.atom.get_or_init(|| self.read_atom()).as_ref()? {
            AtomValue::String(s) => Some(s.clone()),
            _ => None,
        }
    }
    fn boolean(&self) -> Option<bool> {
        match self.atom.get_or_init(|| self.read_atom()).as_ref()? {
            AtomValue::Boolean(b) => Some(*b),
            _ => None,
        }
    }
    fn is_native_node(&self) -> bool {
        self.native_kind == Some(QueryItemViewKind::Node)
    }
    fn record_fields(&self) -> Option<Vec<(String, Vec<Self>)>> {
        self.fields.get_or_init(|| self.read_fields()).clone()
    }
}
pub fn validate_values(
    contracts: &ValueContracts,
    name: &ContractName,
    values: &[Item],
) -> Result<(), ValueContractError> {
    if values.len() != 1 {
        return Err(ValueContractError::new("result-cardinality"));
    }
    contracts.validate(name, &[QueryValue::new(values[0].clone())])
}
#[derive(Debug, Clone, Default, PartialEq)]
pub struct DiagnosticAttribution {
    pub uri: Option<String>,
    pub node: Option<String>,
    pub line: Option<u32>,
    pub column: Option<u32>,
    pub byte_offset: Option<u64>,
    pub source_map: Option<SourceMapStack>,
}
impl DiagnosticAttribution {
    pub fn from_node(node: &Item) -> Self {
        let mut attribution = Self {
            node: node.identity(),
            source_map: node.source_map(),
            ..Default::default()
        };
        if let Some(retained) = eval::retained_cem_node(node) {
            attribution.uri = (!retained.owner().source_uri().is_empty())
                .then(|| retained.owner().source_uri().to_owned());
            if let Some(range) = retained.owner().source_node_range(retained.node_id()) {
                attribution.byte_offset = Some(range.offset);
                attribution.line = (range.line > 0).then_some(range.line);
                attribution.column = (range.column > 0).then_some(range.column);
            }
        }
        if attribution.byte_offset.is_none() {
            attribution.byte_offset = attribution
                .source_map
                .as_ref()
                .and_then(|s| s.frames.last())
                .and_then(|f| match &f.span {
                    FrameSpan::Single(r) => Some(r.start),
                    FrameSpan::Multi(r) => r.first().map(|r| r.start),
                });
        }
        attribution
    }
    fn apply(&self, diagnostic: &mut Diagnostic) {
        diagnostic.uri = self.uri.clone();
        diagnostic.node = self.node.clone();
        diagnostic.line = self.line;
        diagnostic.column = self.column;
        diagnostic.byte_offset = self.byte_offset;
        diagnostic.source_map = self.source_map.clone();
    }
}
/// Retain an existing shared diagnostic as a native query value.
pub fn native_diagnostic_value(diagnostic: Diagnostic) -> Item {
    eval::diagnostic_item(diagnostic)
}
#[derive(Debug, Clone)]
pub enum DatatypeResultError {
    Contract(ValueContractError),
    /// Preserve the original failed outcome and its attributed diagnostics.
    Execution(ItemStream),
}
impl From<ValueContractError> for DatatypeResultError {
    fn from(error: ValueContractError) -> Self {
        Self::Contract(error)
    }
}
#[derive(Debug, Clone)]
pub struct DatatypeValidationResult {
    pub accepted: bool,
    /// Existing shared report values; caller applies host reporting/abort policy.
    pub diagnostics: Vec<Diagnostic>,
    /// Already-emitted query diagnostics remain separate from returned rule data.
    pub execution_diagnostics: Vec<Diagnostic>,
    /// Retains the original result and all native source/diagnostic owners.
    pub original: Item,
}
#[derive(Debug, Clone)]
pub struct DatatypeResultAdapter {
    contracts: Arc<ValueContracts>,
    result: ContractName,
    diagnostic: ContractName,
}
impl DatatypeResultAdapter {
    pub fn new(
        contracts: Arc<ValueContracts>,
        result: ContractName,
        diagnostic: ContractName,
    ) -> Result<Self, ValueContractError> {
        use cem_ml::schema::value_contracts::{Cardinality, ValueFieldType};
        let shape = contracts
            .get(&result)
            .ok_or_else(|| ValueContractError::new("unknown-result-contract"))?;
        let accepted = shape
            .fields
            .get("accepted")
            .ok_or_else(|| ValueContractError::new("incompatible-result-contract"))?;
        let diagnostics = shape
            .fields
            .get("diagnostics")
            .ok_or_else(|| ValueContractError::new("incompatible-result-contract"))?;
        if accepted.value_type != ValueFieldType::Boolean
            || !accepted.required
            || accepted.cardinality != Cardinality::One
            || diagnostics.value_type != ValueFieldType::Record(diagnostic.clone())
            || !diagnostics.required
            || diagnostics.cardinality != Cardinality::ZeroOrMore
        {
            return Err(ValueContractError::new("incompatible-result-contract"));
        }
        let diagnostic_shape = contracts
            .get(&diagnostic)
            .ok_or_else(|| ValueContractError::new("unknown-diagnostic-contract"))?;
        for name in ["code", "severity", "message"] {
            if !diagnostic_shape.fields.get(name).is_some_and(|field| {
                field.value_type == ValueFieldType::String
                    && field.required
                    && field.cardinality == Cardinality::One
            }) {
                return Err(ValueContractError::new("incompatible-diagnostic-contract"));
            }
        }
        if !diagnostic_shape.fields.get("source").is_some_and(|field| {
            field.value_type == ValueFieldType::Node
                && !field.required
                && field.cardinality == Cardinality::ZeroOrOne
        }) {
            return Err(ValueContractError::new("incompatible-diagnostic-contract"));
        }
        Ok(Self {
            contracts,
            result,
            diagnostic,
        })
    }
    pub fn diagnostic_contract(&self) -> &ContractName {
        &self.diagnostic
    }
    pub fn result_contract(&self) -> &ContractName {
        &self.result
    }
    /// Only complete streams can be consumed. Execution failures remain separate
    /// from validation rejection; no pending state is inferred from an empty stream.
    pub fn consume(
        &self,
        stream: ItemStream,
        fallback: &DiagnosticAttribution,
    ) -> Result<DatatypeValidationResult, DatatypeResultError> {
        if stream.error.is_some() {
            return Err(DatatypeResultError::Execution(stream));
        }
        if stream.items.len() != 1 {
            return Err(ValueContractError::new("result-cardinality").into());
        }
        let original = stream.items.into_iter().next().unwrap();
        let snapshot = QueryValue::new(original.clone());
        self.contracts.validate(&self.result, &[snapshot.clone()])?;
        let fields = snapshot
            .record_fields()
            .ok_or_else(|| ValueContractError::new("record-required"))?;
        let field = |name: &str| {
            fields
                .iter()
                .find(|(key, _)| key == name)
                .map(|(_, v)| v.as_slice())
        };
        let accepted = field("accepted")
            .and_then(|v| v.first())
            .and_then(ContractValue::boolean)
            .ok_or_else(|| ValueContractError::new("accepted-required"))?;
        let mut diagnostics = Vec::new();
        for value in
            field("diagnostics").ok_or_else(|| ValueContractError::new("diagnostics-required"))?
        {
            if let Some(diagnostic) = eval::native_diagnostic(&value.original) {
                diagnostics.push(diagnostic.clone());
                continue;
            }
            let fields = value
                .record_fields()
                .ok_or_else(|| ValueContractError::new("diagnostic-record-required"))?;
            let get = |key: &str| {
                fields
                    .iter()
                    .find(|(name, _)| name == key)
                    .map(|(_, v)| v.as_slice())
            };
            let string = |key: &str| -> Result<String, ValueContractError> {
                match get(key) {
                    Some([value]) => value
                        .string()
                        .ok_or_else(|| ValueContractError::new("diagnostic-string-required")),
                    _ => Err(ValueContractError::new("diagnostic-string-required")),
                }
            };
            let severity = match string("severity")?.as_str() {
                "info" => Severity::Info,
                "warning" => Severity::Warning,
                "error" => Severity::Error,
                "fatal" => Severity::Fatal,
                _ => return Err(ValueContractError::new("invalid-severity").into()),
            };
            let mut diagnostic = Diagnostic {
                code: string("code")?,
                message: string("message")?,
                severity,
                ..Default::default()
            };
            match get("source") {
                None | Some([]) => fallback.apply(&mut diagnostic),
                Some([source]) if source.is_native_node() => {
                    DiagnosticAttribution::from_node(&source.original).apply(&mut diagnostic)
                }
                _ => return Err(ValueContractError::new("invalid-diagnostic-source").into()),
            }
            diagnostics.push(diagnostic);
        }
        if !accepted && diagnostics.is_empty() {
            let mut diagnostic = Diagnostic {
                code: "cem.datatype.rejected".into(),
                severity: Severity::Error,
                message: "Datatype rule rejected the supplied value".into(),
                ..Default::default()
            };
            fallback.apply(&mut diagnostic);
            diagnostics.push(diagnostic);
        }
        Ok(DatatypeValidationResult {
            accepted,
            diagnostics,
            execution_diagnostics: stream.diagnostics,
            original,
        })
    }
}
