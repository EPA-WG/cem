//! Exhaustive typed-IR inventory; all bodies/branches are conservatively kept.
use super::*;
use crate::ir::{CompiledQuery, IrId, IrNode, IrRecordKey, IrStep};
use crate::stdlib::StdlibImplKind;
impl Inventory {
    pub(super) fn expression(
        &mut self,
        expression: &CompiledTemplateExpression,
    ) -> Result<(), String> {
        self.charge(1)?;
        let site = format!("expression:{}", self.output.expression_count);
        self.output.expression_count += 1;
        let Some(query) = &expression.query else {
            self.gap("uncompiled-expression", &site);
            return Ok(());
        };
        self.query(query, &site)
    }
    pub(super) fn query(&mut self, query: &CompiledQuery, site: &str) -> Result<(), String> {
        let tree = &query.tree;
        self.charge(tree.nodes.len())?;
        self.output.ir_node_count += tree.nodes.len();
        if tree.root.0 as usize >= tree.nodes.len()
            || tree.resolutions.len() != tree.nodes.len()
            || tree.types.len() != tree.nodes.len()
            || tree.source_maps.len() != tree.nodes.len()
        {
            return Err("requirements invalid IR tables/root".into());
        }
        let functions: BTreeSet<_> = tree
            .nodes
            .iter()
            .zip(&tree.resolutions)
            .filter_map(|(node, binding)| {
                matches!(node, IrNode::Lambda { .. })
                    .then_some(*binding)
                    .flatten()
            })
            .collect();
        let mut edges = Vec::with_capacity(tree.nodes.len());
        for (index, node) in tree.nodes.iter().enumerate() {
            let at = format!("{site}/ir:{index}");
            let mut children = Vec::new();
            match node {
                IrNode::StdlibCall { module, name, args } => {
                    self.call(query, &module.0, &name.local, args, false, &functions, &at);
                    children.extend(args);
                }
                IrNode::FunctionRef(binding) => {
                    if !functions.contains(binding) {
                        self.gap("unbound-function", &at);
                    }
                }
                IrNode::Call { callee, args } => {
                    if !matches!(tree.node(*callee), Some(IrNode::Lambda { .. }))
                        && !matches!(tree.node(*callee), Some(IrNode::FunctionRef(binding)) if functions.contains(binding))
                    {
                        self.gap("dynamic-call", &at);
                    }
                    children.push(callee);
                    children.extend(args);
                }
                IrNode::Lambda { body, .. } => children.push(body),
                IrNode::Pipeline { source, steps } => {
                    children.push(source);
                    for step in steps {
                        match step {
                            IrStep::NamedStdlib { module, name, args } => {
                                self.call(
                                    query,
                                    &module.0,
                                    &name.local,
                                    args,
                                    true,
                                    &functions,
                                    &at,
                                );
                                children.extend(args);
                            }
                            IrStep::Named {
                                binding,
                                name,
                                args,
                            } => {
                                if let Some(binding) = binding {
                                    if !functions.contains(binding) {
                                        self.gap("unbound-pipeline-function", &at);
                                    }
                                } else {
                                    let builtin = matches!(
                                        name.local.as_str(),
                                        "first"
                                            | "last"
                                            | "take"
                                            | "drop"
                                            | "nth"
                                            | "target"
                                            | "where"
                                    );
                                    self.output.pipeline_operations.insert(PipelineOperation {
                                        kind: if builtin { "builtin" } else { "field" }.into(),
                                        name: name.local.clone(),
                                        explicit_arity: args.len(),
                                    });
                                    if name.prefix.is_some() || (!builtin && !args.is_empty()) {
                                        self.gap("unknown-pipeline-contract", &at);
                                    }
                                }
                                if binding.is_none() && name.local == "where" {
                                    self.callback(query, args.first().copied(), &functions, &at);
                                }
                                if binding.is_none() && name.local == "target" {
                                    self.gap("reference-provider", &at);
                                }
                                children.extend(args);
                            }
                            IrStep::Method { name, args } => {
                                self.output.pipeline_operations.insert(PipelineOperation {
                                    kind: "method".into(),
                                    name: name.local.clone(),
                                    explicit_arity: args.len(),
                                });
                                if !crate::stdlib::dom::chain_method_arity(&name.local)
                                    .is_some_and(|(min, max)| (min..=max).contains(&args.len()))
                                {
                                    self.gap("unknown-chain-method", &at);
                                }
                                // Receiver callbacks may dispatch through host-provided lambdas.
                                if matches!(
                                    name.local.as_str(),
                                    "map" | "flat_map" | "filter" | "any" | "all" | "sorted_by_key"
                                ) {
                                    self.callback(query, args.first().copied(), &functions, &at);
                                }
                                children.extend(args);
                            }
                            IrStep::Lambda(id) => {
                                if !matches!(tree.node(*id), Some(IrNode::Lambda { .. })) {
                                    self.gap("dynamic-pipeline-lambda", &at);
                                }
                                children.push(id);
                            }
                        }
                    }
                }
                IrNode::Record(entries) => {
                    for (key, value) in entries {
                        if let IrRecordKey::Computed(id) = key {
                            children.push(id);
                        }
                        children.push(value);
                    }
                }
                IrNode::Array(ids) | IrNode::Sequence(ids) => children.extend(ids),
                IrNode::AxisStep { predicates, .. } => children.extend(predicates),
                IrNode::BinaryOp { lhs, rhs, .. }
                | IrNode::SetOp { lhs, rhs, .. }
                | IrNode::Is { lhs, rhs } => {
                    children.push(lhs);
                    children.push(rhs);
                }
                IrNode::UnaryOp { operand, .. } => children.push(operand),
                IrNode::If {
                    cond,
                    then_branch,
                    else_branch,
                } => children.extend([cond, then_branch, else_branch]),
                IrNode::Let { value, body, .. } => children.extend([value, body]),
                IrNode::For { source, body, .. } => children.extend([source, body]),
                IrNode::Quantified {
                    source, predicate, ..
                } => children.extend([source, predicate]),
                IrNode::InstanceOf { value, .. }
                | IrNode::CastAs { value, .. }
                | IrNode::TreatAs { value, .. } => children.push(value),
                IrNode::TryCatch { body, handler, .. } => children.extend([body, handler]),
                IrNode::SchemaType(_) | IrNode::TemplateRef(_) => {
                    self.gap("external-type-or-template-reference", &at)
                }
                IrNode::StateSlot(_) => self.gap("state-provider", &at),
                IrNode::LitString(_)
                | IrNode::LitInt(_)
                | IrNode::LitDecimal(_)
                | IrNode::LitDouble(_)
                | IrNode::LitBool(_)
                | IrNode::LitNull
                | IrNode::LocalVar(_)
                | IrNode::Parent
                | IrNode::Self_
                | IrNode::Reference
                | IrNode::LeadingDot => (),
            }
            self.charge(children.len())?;
            if children.iter().any(|id| id.0 as usize >= tree.nodes.len()) {
                return Err("requirements dangling IR reference".into());
            }
            edges.push(
                children
                    .into_iter()
                    .map(|id| id.0 as usize)
                    .collect::<Vec<_>>(),
            );
        }
        // Validate storage edges iteratively; function recursion travels through
        // binding IDs and remains legitimate. Never recurse through untrusted IR.
        let mut incoming = vec![0usize; edges.len()];
        for children in &edges {
            for &id in children {
                incoming[id] += 1;
            }
        }
        let mut ready: Vec<_> = incoming
            .iter()
            .enumerate()
            .filter_map(|(id, n)| (*n == 0).then_some(id))
            .collect();
        let mut visited = 0;
        while let Some(id) = ready.pop() {
            visited += 1;
            for &child in &edges[id] {
                incoming[child] -= 1;
                if incoming[child] == 0 {
                    ready.push(child);
                }
            }
        }
        if visited != edges.len() {
            return Err("requirements cyclic IR storage".into());
        }
        Ok(())
    }
    fn callback(
        &mut self,
        query: &CompiledQuery,
        id: Option<IrId>,
        functions: &BTreeSet<crate::resolve::BindingId>,
        site: &str,
    ) {
        if !matches!(
            id.and_then(|id| query.tree.node(id)),
            Some(IrNode::Lambda { .. })
        ) && !matches!(id.and_then(|id|query.tree.node(id)), Some(IrNode::FunctionRef(binding)) if functions.contains(binding))
        {
            self.gap("dynamic-callback", site);
        }
    }
    fn call(
        &mut self,
        query: &CompiledQuery,
        module: &str,
        name: &str,
        args: &[IrId],
        pipeline: bool,
        functions: &BTreeSet<crate::resolve::BindingId>,
        site: &str,
    ) {
        let arity = args.len() + usize::from(pipeline);
        if pipeline
            && !(module == "cem:stdlib/sequence"
                && matches!(name, "first" | "last" | "take" | "drop" | "nth" | "where"))
            && !(module == "cem:stdlib/dom"
                && ((matches!(name, "parent" | "children" | "descendants") && args.is_empty())
                    || (name == "attribute" && args.len() == 1)))
        {
            self.gap("unavailable-pipeline-dispatch", site);
        }
        let implementation = if arity <= u8::MAX as usize {
            self.registry.resolve(module, name, arity)
        } else {
            None
        };
        let implementation = match implementation.map(|function| function.implementation) {
            Some(StdlibImplKind::Native) => "native",
            Some(StdlibImplKind::Macro) => "macro",
            Some(StdlibImplKind::HostContext) => "host-context",
            Some(StdlibImplKind::Deferred) => "deferred",
            None => "unknown",
        };
        self.output.functions.insert(FunctionRequirement {
            module: module.into(),
            name: name.into(),
            arity,
            dispatch: if pipeline { "pipeline" } else { "call" }.into(),
            implementation: implementation.into(),
        });
        if matches!(implementation, "unknown" | "deferred") {
            self.gap("unavailable-function", site);
        }
        if module == "cem:stdlib/sequence"
            && matches!(
                name,
                "map" | "flat_map" | "where" | "peek" | "any" | "all" | "group_by" | "sorted"
            )
        {
            self.callback(
                query,
                args.first()
                    .copied()
                    .filter(|_| pipeline)
                    .or_else(|| args.get(1).copied()),
                functions,
                site,
            );
        }
        match (module, name) {
            ("cem:stdlib/data", "read" | "parse") => {
                let format = literal(query, args.get(1).copied());
                let schema = if name == "read" {
                    args.get(2)
                        .map(|id| literal(query, Some(*id)))
                        .unwrap_or(Some("cem".into()))
                } else {
                    None
                };
                if format.is_none() {
                    self.gap("dynamic-data-format", site);
                }
                if name == "read" && schema.is_none() {
                    self.gap("dynamic-data-schema", site);
                }
                if name == "parse" && args.len() > 2 {
                    self.gap("data-import-options-contract", site);
                }
                self.output.data_imports.insert(DataImportRequirement {
                    operation: name.into(),
                    format,
                    schema,
                });
            }
            ("cem:stdlib/native", "call") => self.gap("native-host-dispatch", site),
            ("cem:stdlib/cemt", _) => self.gap("template-host-dispatch", site),
            ("cem:stdlib/template", _) => self.gap("template-provider", site),
            ("cem:stdlib/state", _) => self.gap("state-provider", site),
            ("cem:stdlib/modules", _) => self.gap("module-resolution-provider", site),
            ("cem:stdlib/content-types", "read") => self.gap("external-source-provider", site),
            ("cem:stdlib/cemml", _) => self.gap("runtime-source-tooling", site),
            ("cem:stdlib/user", _) => self.gap("user-policy-provider", site),
            _ => (),
        }
    }
}

fn literal(query: &CompiledQuery, id: Option<IrId>) -> Option<String> {
    match id.and_then(|id| query.tree.node(id)) {
        Some(IrNode::LitString(value)) if value.len() <= 1024 => Some(value.clone()),
        _ => None,
    }
}
