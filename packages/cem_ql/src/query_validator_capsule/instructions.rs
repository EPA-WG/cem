//! Conservative, exhaustive native instruction and function closure. No source
//! parser, optimiser, host lookup or general stdlib fallback is used here.
use super::*;
use crate::{
    ir::{IrId, IrNode, IrRecordKey, IrStep},
    parser::UnaryOp,
    selected_registry::SelectedBuiltin,
    types::AtomType,
};
use cem_ml::source_map::{FrameSpan, TransformKind};
use std::collections::BTreeSet;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct FunctionRequirement {
    pub(super) module: String,
    pub(super) name: String,
    pub(super) arity: usize,
    pub(super) signature: String,
    pub(super) implementation_abi: String,
}
impl FunctionRequirement {
    pub(super) fn selected(&self) -> Result<SelectedBuiltin, String> {
        let selected = SelectedBuiltin::from_key(&self.module, &self.name, self.arity)?;
        if *self != requirement(selected)? {
            return Err(INVALID.into());
        }
        Ok(selected)
    }
}
fn requirement(entry: SelectedBuiltin) -> Result<FunctionRequirement, String> {
    use SelectedBuiltin::*;
    let signature = match entry {
        SequenceFirst | SequenceLast => "items->optional-item",
        SequenceCount => "items->integer",
        SequenceTake | SequenceDrop => "items,integer->items",
        StringLength => "text->integer",
        StringLower | StringUpper => "text->string",
        StringContains => "text,text->boolean",
        // Callback-based and effect-bearing operations need their own closure.
        _ => return Err(INVALID.into()),
    };
    let (module, name, arity) = entry.key();
    Ok(FunctionRequirement {
        module: module.into(),
        name: name.into(),
        arity,
        signature: signature.into(),
        implementation_abi: format!("cem.selected-validator/1/{}", entry.rust_variant()),
    })
}
fn inspect_type(ty: &Type, work: &Work<'_>, depth: usize) -> Result<(), String> {
    work.check()?;
    if depth > 32 {
        return Err(INVALID.into());
    }
    match ty {
        Type::Any | Type::Empty | Type::Node(NodeKind::Node) => (),
        Type::Atom(
            AtomType::String
            | AtomType::Integer
            | AtomType::Decimal
            | AtomType::Double
            | AtomType::Boolean
            | AtomType::AnyUri,
        ) => (),
        Type::Stream(item) | Type::Array(item) => inspect_type(item, work, depth + 1)?,
        Type::Record(fields) if fields.len() <= 64 => {
            let mut names = BTreeSet::new();
            for field in fields {
                work.check()?;
                if field.name.len() > 256 || !names.insert(&field.name) {
                    return Err(INVALID.into());
                }
                inspect_type(&field.ty, work, depth + 1)?;
            }
        }
        _ => return Err(INVALID.into()),
    }
    Ok(())
}
fn valid_name(name: &crate::parser::QName, body_length: usize) -> bool {
    !name.local.is_empty()
        && name.local.len() <= 256
        && name
            .prefix
            .as_ref()
            .is_none_or(|p| !p.is_empty() && p.len() <= 256)
        && name
            .range
            .start
            .checked_add(name.range.len as u64)
            .is_some_and(|end| end <= body_length as u64)
}
pub(super) struct Inspection {
    pub(super) functions: Vec<FunctionRequirement>,
    pub(super) expanded_work: u64,
}
pub(super) fn inspect(capsule: &Capsule, work: &Work<'_>) -> Result<Inspection, String> {
    let tree = &capsule.tree;
    let len = tree.nodes.len();
    if len == 0
        || len > MAX_RECORDS
        || tree.root.0 as usize >= len
        || tree.types.len() != len
        || tree.resolutions.len() != len
        || tree.source_maps.len() != len
        || capsule.bindings.len() != 3
        || !capsule.bindings.windows(2).all(|w| w[0].0 < w[1].0)
        || capsule
            .bindings
            .iter()
            .map(|p| p.1.as_str())
            .collect::<BTreeSet<_>>()
            != BTreeSet::from(["value", "datatype", "candidate"])
    {
        return Err(INVALID.into());
    }
    let bindings = capsule
        .bindings
        .iter()
        .map(|p| p.0)
        .collect::<BTreeSet<_>>();
    let prefix = capsule.declaration.maps.last().ok_or(INVALID)?;
    let mut requirements = BTreeSet::new();
    let mut edges = Vec::with_capacity(len);
    for (index, node) in tree.nodes.iter().enumerate() {
        work.check()?;
        inspect_type(&tree.types[index], work, 0)?;
        let map = &tree.source_maps[index];
        if map.frames.len() != prefix.frames.len() + 1
            || map.frames[..prefix.frames.len()] != prefix.frames
        {
            return Err(INVALID.into());
        }
        let relative = map.frames.last().ok_or(INVALID)?;
        if relative.source_id != cem_ml::source::SourceId(0)
            || !matches!(
                relative.transform,
                TransformKind::Query | TransformKind::QueryStep
            )
            || !matches!(&relative.span,FrameSpan::Single(r) if r.start.checked_add(r.len as u64).is_some_and(|end|end<=capsule.declaration.body_length as u64))
        {
            return Err(INVALID.into());
        }
        if tree.resolutions[index]
            != match node {
                IrNode::LocalVar(binding) => Some(*binding),
                _ => None,
            }
        {
            return Err(INVALID.into());
        }
        let mut children = Vec::<IrId>::new();
        match node {
            IrNode::LitString(s) if s.len() <= 16 * 1024 => (),
            IrNode::LitInt(_) | IrNode::LitBool(_) | IrNode::LitNull => (),
            IrNode::LocalVar(binding) if bindings.contains(binding) => (),
            IrNode::Record(fields) if fields.len() <= 64 => {
                let mut names = BTreeSet::new();
                for (key, value) in fields {
                    work.check()?;
                    let IrRecordKey::Static(key) = key else {
                        return Err(INVALID.into());
                    };
                    if key.len() > 256 || !names.insert(key) {
                        return Err(INVALID.into());
                    }
                    children.push(*value);
                }
            }
            IrNode::Array(ids) | IrNode::Sequence(ids) if ids.len() <= 1024 => children.extend(ids),
            IrNode::StdlibCall { module, name, args } => {
                let selected = SelectedBuiltin::from_key(&module.0, &name.local, args.len())
                    .map_err(|_| INVALID)?;
                requirements.insert(requirement(selected)?);
                if !valid_name(name, capsule.declaration.body_length) {
                    return Err(INVALID.into());
                }
                children.extend(args);
            }
            IrNode::BinaryOp { lhs, rhs, .. } | IrNode::Is { lhs, rhs } => {
                children.extend([*lhs, *rhs])
            }
            IrNode::UnaryOp {
                operand,
                op: UnaryOp::Negate | UnaryOp::Not,
            } => children.push(*operand),
            IrNode::If {
                cond,
                then_branch,
                else_branch,
            } => children.extend([*cond, *then_branch, *else_branch]),
            IrNode::Pipeline { source, steps } if steps.len() <= 64 => {
                children.push(*source);
                for step in steps {
                    work.check()?;
                    // Pure, scoped field navigation uses the existing native
                    // views. Builtin/receiver/dynamic pipeline dispatch refuses.
                    let IrStep::Named {
                        name,
                        binding: None,
                        args,
                    } = step
                    else {
                        return Err(INVALID.into());
                    };
                    if !valid_name(name, capsule.declaration.body_length)
                        || name.prefix.is_some()
                        || !args.is_empty()
                        || name.local.is_empty()
                        || name.local.len() > 256
                        || matches!(
                            name.local.as_str(),
                            "first" | "last" | "take" | "drop" | "nth" | "where" | "target"
                        )
                    {
                        return Err(INVALID.into());
                    }
                }
            }
            _ => return Err(INVALID.into()),
        }
        // Native lowering is postorder. This also forbids dangling edges and
        // cycles before the shared recursive evaluator can see the program.
        for child in &children {
            work.check()?;
            if child.0 as usize >= index {
                return Err(INVALID.into());
            }
        }
        edges.push(children);
    }
    // Charge expanded DAG paths as well as stored instructions. No per-branch
    // work counter reset and no silently unreviewed unreachable program records.
    let mut visited = BTreeSet::new();
    let mut pending = vec![(tree.root, 0usize)];
    let mut expanded_work = 0u64;
    while let Some((id, depth)) = pending.pop() {
        work.check()?;
        expanded_work += 1;
        if depth > 64 {
            return Err(INVALID.into());
        }
        visited.insert(id);
        pending.extend(edges[id.0 as usize].iter().map(|id| (*id, depth + 1)));
    }
    if visited.len() != len {
        return Err(INVALID.into());
    }
    Ok(Inspection {
        functions: requirements.into_iter().collect(),
        expanded_work,
    })
}
