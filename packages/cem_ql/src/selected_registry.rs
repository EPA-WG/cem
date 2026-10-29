//! Native prototype of an immutable selected standard-library registry. This is
//! an execution selection, not a sandbox, native grant or linked-code proof.
use serde::Serialize;
use std::collections::BTreeSet;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum SelectedBuiltin {
    SequenceFirst,
    SequenceLast,
    SequenceCount,
    SequenceTake,
    SequenceDrop,
    SequenceMap,
    SequenceFlatMap,
    SequenceWhere,
    SequenceAny,
    SequenceAll,
    SequencePeek,
    StringLength,
    StringLower,
    StringUpper,
    StringContains,
}
impl SelectedBuiltin {
    pub fn key(self) -> (&'static str, &'static str, usize) {
        use SelectedBuiltin::*;
        let (module, name, arity) = match self {
            SequenceFirst => ("sequence", "first", 1),
            SequenceLast => ("sequence", "last", 1),
            SequenceCount => ("sequence", "count", 1),
            SequenceTake => ("sequence", "take", 2),
            SequenceDrop => ("sequence", "drop", 2),
            SequenceMap => ("sequence", "map", 2),
            SequenceFlatMap => ("sequence", "flat_map", 2),
            SequenceWhere => ("sequence", "where", 2),
            SequenceAny => ("sequence", "any", 2),
            SequenceAll => ("sequence", "all", 2),
            SequencePeek => ("sequence", "peek", 2),
            StringLength => ("strings", "length", 1),
            StringLower => ("strings", "lower", 1),
            StringUpper => ("strings", "upper", 1),
            StringContains => ("strings", "contains", 2),
        };
        (
            if module == "sequence" {
                "cem:stdlib/sequence"
            } else {
                "cem:stdlib/strings"
            },
            name,
            arity,
        )
    }
    pub fn from_key(module: &str, name: &str, arity: usize) -> Result<Self, String> {
        use SelectedBuiltin::*;
        [
            SequenceFirst,
            SequenceLast,
            SequenceCount,
            SequenceTake,
            SequenceDrop,
            SequenceMap,
            SequenceFlatMap,
            SequenceWhere,
            SequenceAny,
            SequenceAll,
            SequencePeek,
            StringLength,
            StringLower,
            StringUpper,
            StringContains,
        ]
        .into_iter()
        .find(|entry| entry.key() == (module, name, arity))
        .ok_or_else(|| format!("function outside reviewed selected core: {module}/{name}#{arity}"))
    }
    pub(crate) fn implementation(self) -> &'static str {
        use SelectedBuiltin::*;
        if matches!(
            self,
            SequenceMap
                | SequenceFlatMap
                | SequenceWhere
                | SequenceAny
                | SequenceAll
                | SequencePeek
        ) {
            "macro"
        } else {
            "native"
        }
    }
    pub(crate) fn rust_variant(self) -> &'static str {
        use SelectedBuiltin::*;
        match self {
            SequenceFirst => "SequenceFirst",
            SequenceLast => "SequenceLast",
            SequenceCount => "SequenceCount",
            SequenceTake => "SequenceTake",
            SequenceDrop => "SequenceDrop",
            SequenceMap => "SequenceMap",
            SequenceFlatMap => "SequenceFlatMap",
            SequenceWhere => "SequenceWhere",
            SequenceAny => "SequenceAny",
            SequenceAll => "SequenceAll",
            SequencePeek => "SequencePeek",
            StringLength => "StringLength",
            StringLower => "StringLower",
            StringUpper => "StringUpper",
            StringContains => "StringContains",
        }
    }
    pub fn execution_units(self) -> BTreeSet<NativeExecutionUnit> {
        use NativeExecutionUnit::*;
        use SelectedBuiltin::*;
        let mut pending = vec![
            SelectedDispatch,
            match self {
                SequenceMap | SequenceFlatMap | SequenceWhere | SequenceAny | SequenceAll => {
                    SequenceCallbacks
                }
                SequenceFirst | SequenceLast | SequenceCount | SequenceTake | SequenceDrop
                | SequencePeek => SequenceValues,
                StringLength | StringContains => TextCoercion,
                StringLower | StringUpper => UnicodeCase,
            },
        ];
        let mut closure = BTreeSet::new();
        while let Some(unit) = pending.pop() {
            if closure.insert(unit) {
                pending.extend(unit.dependencies());
            }
        }
        closure
    }
}
/// Coarse reviewed execution contracts, not a complete Rust symbol/link graph.
/// The current full evaluator and fallback implementation remain linked.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum NativeExecutionUnit {
    SelectedDispatch,
    EvaluatorCore,
    OperationControl,
    RetainedValues,
    Provenance,
    FieldNavigation,
    LocalLambdas,
    BooleanCoercion,
    SequenceValues,
    SequenceCallbacks,
    TextCoercion,
    UnicodeCase,
}
impl NativeExecutionUnit {
    pub fn source_paths(self) -> &'static [&'static str] {
        use NativeExecutionUnit::*;
        match self {
            SelectedDispatch => &["packages/cem_ql/src/eval/selected.rs"],
            OperationControl => &[
                "packages/cem_ql/src/eval/budget.rs",
                "packages/cem_ml/src/operation_control.rs",
            ],
            EvaluatorCore | LocalLambdas | BooleanCoercion => &["packages/cem_ql/src/eval.rs"],
            RetainedValues => &[
                "packages/cem_ql/src/eval.rs",
                "packages/cem_ql/src/eval/values.rs",
            ],
            Provenance => &[
                "packages/cem_ql/src/eval/source.rs",
                "packages/cem_ml/src/source_map.rs",
            ],
            FieldNavigation | SequenceValues | SequenceCallbacks | TextCoercion | UnicodeCase => {
                &["packages/cem_ql/src/eval/pipeline.rs"]
            }
        }
    }
    fn dependencies(self) -> &'static [Self] {
        use NativeExecutionUnit::*;
        match self {
            SelectedDispatch => &[EvaluatorCore, OperationControl],
            EvaluatorCore => &[
                OperationControl,
                RetainedValues,
                Provenance,
                FieldNavigation,
                LocalLambdas,
                BooleanCoercion,
            ],
            FieldNavigation => &[RetainedValues, Provenance],
            LocalLambdas => &[EvaluatorCore, SelectedDispatch],
            SequenceValues => &[RetainedValues],
            SequenceCallbacks => &[SequenceValues, LocalLambdas, BooleanCoercion],
            BooleanCoercion => &[RetainedValues],
            TextCoercion => &[RetainedValues, Provenance],
            UnicodeCase => &[TextCoercion],
            OperationControl | RetainedValues | Provenance => &[],
        }
    }
}
#[derive(Debug, Clone)]
pub struct SelectedFunctionRegistry {
    entries: BTreeSet<SelectedBuiltin>,
}
impl SelectedFunctionRegistry {
    pub fn new(entries: impl IntoIterator<Item = SelectedBuiltin>) -> Result<Self, String> {
        let mut selected = BTreeSet::new();
        for entry in entries {
            if !selected.insert(entry) {
                return Err("duplicate selected function".into());
            }
        }
        Ok(Self { entries: selected })
    }
    pub fn resolve(&self, module: &str, name: &str, arity: usize) -> Option<SelectedBuiltin> {
        SelectedBuiltin::from_key(module, name, arity)
            .ok()
            .filter(|entry| self.entries.contains(entry))
    }
    pub fn entries(&self) -> impl Iterator<Item = SelectedBuiltin> + '_ {
        self.entries.iter().copied()
    }
}

#[cfg(test)]
mod tests;
