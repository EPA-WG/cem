//! Opt-in native read receipts. These retain stamps, never inputs, outputs or
//! provider handles. Matching is a necessary pure-evaluation input check, not
//! permission to skip operation admission, events, commands or resource work.
use super::*;
use cem_ml::content_cache::{ContentHash, HASH_SCHEME};
use cem_ml::dependency_identity::DependencyVersion;
use std::collections::{BTreeMap, BTreeSet};
use std::io::{self, Write};

pub const MAX_EVALUATION_READS: usize = 1024;
pub const MAX_EVALUATION_RECEIPT_BYTES: usize = 1024 * 1024;

/// Provider-owned contracts: stamps cover complete values, provenance, access
/// policy and absence, not object/DOM identities or local revision counters.
/// Non-opaque stamps attest side-effect-free reads as well as complete freshness;
/// providers unable to uphold both requirements must remain opaque.
/// Environment covers provider configuration, resolver/reader freshness and
/// policy changes outside explicit inputs. Opaque is the default, never stable.
#[derive(Debug, Clone)]
pub struct EvaluationFreshness {
    pub environment: DependencyVersion,
    pub bindings: BTreeMap<String, DependencyVersion>,
    pub current_item: DependencyVersion,
}
impl Default for EvaluationFreshness {
    fn default() -> Self {
        Self {
            environment: DependencyVersion::Opaque,
            bindings: BTreeMap::new(),
            current_item: DependencyVersion::Opaque,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EvaluationTrackingError {
    OpaqueProvider,
    MissingFreshness,
    InvalidFreshness,
    UnsuccessfulEvaluation,
    Bounds,
}

#[derive(Debug, Clone)]
pub struct EvaluationDependencies {
    query: ContentHash,
    scope: QueryContextScope,
    policy: ScopePolicy,
    environment: DependencyVersion,
    bindings: BTreeMap<String, DependencyVersion>,
    current_item: Option<DependencyVersion>,
}
impl EvaluationDependencies {
    pub fn binding_names(&self) -> impl Iterator<Item = &str> {
        self.bindings.keys().map(String::as_str)
    }

    /// Compare only observed inputs. The producer must advance stamps on every
    /// semantic/provenance/access change. Consumers must independently admit the
    /// current operation and runtime owner; no result cache is enabled here.
    pub fn matches(
        &self,
        query: &CompiledQuery,
        context: &EvaluationContext,
        freshness: &EvaluationFreshness,
    ) -> bool {
        self.scope == context.scope
            && self.policy == context.scope_policy
            && context.diagnostics.is_empty()
            && self.environment == freshness.environment
            && self
                .bindings
                .iter()
                .all(|(name, stamp)| freshness.bindings.get(name) == Some(stamp))
            && self
                .current_item
                .as_ref()
                .is_none_or(|stamp| stamp == &freshness.current_item)
            && query_identity(query).is_ok_and(|hash| hash == self.query)
    }
}

#[derive(Default)]
pub(super) struct ReadTracker {
    bindings: BTreeSet<BindingId>,
    pub(super) current_item: bool,
    overflow: bool,
}
impl EvalCtx<'_> {
    pub(super) fn track_binding_read(&mut self, binding: BindingId) {
        let Some(reads) = &mut self.reads else {
            return;
        };
        if !self.query.policy_bindings.contains_key(&binding) {
            return;
        }
        if reads.bindings.len() < MAX_EVALUATION_READS {
            reads.bindings.insert(binding);
        } else if !reads.bindings.contains(&binding) {
            reads.overflow = true;
        }
    }
}

/// Normal evaluation always runs, even if tracking cannot produce a receipt.
/// Use the control variant when joining an existing operation/budget scope.
pub fn evaluate_tracked(
    query: &CompiledQuery,
    context: &EvaluationContext,
    freshness: &EvaluationFreshness,
) -> (
    ItemStream,
    Result<EvaluationDependencies, EvaluationTrackingError>,
) {
    let policy = ScopePolicy::host_root()
        .with_queue_size(context.scope_policy.queue_size)
        .with_cpu_workers(context.scope_policy.cpu_workers)
        .with_io_streams(context.scope_policy.io_streams);
    let control = OperationControl::with_root_policy(AbortSignal::new(), policy)
        .expect("standalone control policy has valid bounded defaults");
    evaluate_tracked_with_control(query, context, freshness, &control, ROOT_EXECUTION_SCOPE_ID)
}

pub fn evaluate_tracked_with_control(
    query: &CompiledQuery,
    context: &EvaluationContext,
    freshness: &EvaluationFreshness,
    control: &OperationControl,
    scope: ExecutionScopeId,
) -> (
    ItemStream,
    Result<EvaluationDependencies, EvaluationTrackingError>,
) {
    let mut ctx = EvalCtx::new(query, context, control, scope);
    ctx.reads = Some(ReadTracker::default());
    let result = ctx.evaluate_root();
    let receipt = capture(query, context, freshness, &ctx, &result);
    (result, receipt)
}

fn capture(
    query: &CompiledQuery,
    context: &EvaluationContext,
    freshness: &EvaluationFreshness,
    ctx: &EvalCtx<'_>,
    result: &ItemStream,
) -> Result<EvaluationDependencies, EvaluationTrackingError> {
    use EvaluationTrackingError::*;
    if result.error.is_some() || !result.diagnostics.is_empty() {
        return Err(UnsuccessfulEvaluation);
    }
    // Existing closure proof rejects native extensions, dynamic callbacks and
    // template dispatch. Reader/resolver/DOM/data/report modules need their own
    // provider contracts; a stamp alone cannot declare an effectful call pure.
    if query.binding_dependencies().is_none() || !pure_operations(query) {
        return Err(OpaqueProvider);
    }
    let reads = ctx.reads.as_ref().expect("tracked evaluation");
    if reads.overflow {
        return Err(Bounds);
    }
    let mut budget = MAX_EVALUATION_RECEIPT_BYTES;
    validate_stamp(&freshness.environment, &mut budget)?;
    let mut bindings = BTreeMap::new();
    for binding in &reads.bindings {
        let name = &query.policy_bindings[binding];
        budget = budget.checked_sub(name.len()).ok_or(Bounds)?;
        let stamp = freshness.bindings.get(name).ok_or(MissingFreshness)?;
        validate_stamp(stamp, &mut budget)?;
        bindings.insert(name.clone(), stamp.clone());
    }
    let current_item = if reads.current_item {
        validate_stamp(&freshness.current_item, &mut budget)?;
        Some(freshness.current_item.clone())
    } else {
        None
    };
    // The effective budget may be narrower than the supplied context. Receipts
    // store that policy so matching a wider scope cannot silently authorize reuse.
    Ok(EvaluationDependencies {
        query: query_identity(query)?,
        scope: context.scope,
        policy: ctx.scope_policy,
        environment: freshness.environment.clone(),
        bindings,
        current_item,
    })
}

fn pure_operations(query: &CompiledQuery) -> bool {
    let pure_module = |module: &crate::resolve::ModuleUri| {
        matches!(
            module.0.as_str(),
            "cem:stdlib/sequence"
                | "cem:stdlib/strings"
                | "cem:stdlib/numbers"
                | "cem:stdlib/records"
        )
    };
    query.tree.nodes.iter().all(|node| match node {
        IrNode::StdlibCall { module, .. } => pure_module(module),
        IrNode::Pipeline { steps, .. } => steps.iter().all(|step| match step {
            crate::ir::IrStep::NamedStdlib { module, .. } => pure_module(module),
            crate::ir::IrStep::Method { .. } => false,
            crate::ir::IrStep::Named {
                binding: None,
                name,
                ..
            } if name.local == "target" => false,
            _ => true,
        }),
        _ => true, // exhaustive closed-operation proof above remains required
    })
}

fn validate_stamp(
    stamp: &DependencyVersion,
    budget: &mut usize,
) -> Result<(), EvaluationTrackingError> {
    use EvaluationTrackingError::*;
    let bytes = match stamp {
        DependencyVersion::Opaque => return Err(MissingFreshness),
        DependencyVersion::Revision { owner, .. } => {
            if owner.is_empty() {
                return Err(InvalidFreshness);
            }
            owner.len() + 16
        }
        DependencyVersion::Content(hash) => {
            if hash.scheme != HASH_SCHEME
                || hash.hex.len() != 64
                || !hash
                    .hex
                    .bytes()
                    .all(|v| v.is_ascii_digit() || (b'a'..=b'f').contains(&v))
            {
                return Err(InvalidFreshness);
            }
            hash.scheme.len() + hash.hex.len()
        }
    };
    *budget = budget.checked_sub(bytes).ok_or(Bounds)?;
    Ok(())
}

// Include actual native IR, source maps, types, resolutions and policy bindings;
// the portable artifact serializer currently encodes source + binding names.
// Bound encoding before allocation, including programmatically constructed IR.
fn query_identity(query: &CompiledQuery) -> Result<ContentHash, EvaluationTrackingError> {
    struct Bounded(Vec<u8>);
    impl Write for Bounded {
        fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
            if bytes.len() > MAX_EVALUATION_RECEIPT_BYTES.saturating_sub(self.0.len()) {
                return Err(io::Error::other("evaluation identity bound"));
            }
            self.0.extend_from_slice(bytes);
            Ok(bytes.len())
        }
        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }
    let mut out = Bounded(b"cem.ql.evaluation-reads/1\n".to_vec());
    rmp_serde::encode::write_named(&mut out, &(crate::VERSION, query))
        .map_err(|_| EvaluationTrackingError::Bounds)?;
    Ok(ContentHash::from_blake3(&out.0))
}
