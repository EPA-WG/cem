//! Layer 3: name resolution.

use std::collections::{BTreeSet, HashMap};

use cem_ml::diagnostics::{Diagnostic, Severity};
use cem_ml::schema::ScopeId;
use cem_ml::source::ByteRange;

use crate::parser::{
    Expression, FunctionDecl, FunctionParam, ImportDecl, PipelineStep, QName, SurfaceModule,
    SurfaceNode, TypeExpr, VariableDecl,
};

pub mod overlay;

pub use overlay::{ModuleUri, OverlayFingerprint, OverlayKey, OverlayMap, StdlibOverlay};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct BindingId(pub u32);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Arity(pub u32);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct SchemaTypeId(pub u32);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct TemplateRefId(pub u32);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct StateSlotId(pub u32);

#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct QNameKey {
    pub prefix: Option<String>,
    pub local: String,
}

impl QNameKey {
    pub fn new(prefix: Option<String>, local: impl Into<String>) -> Self {
        Self {
            prefix,
            local: local.into(),
        }
    }

    pub fn from_qname(name: &QName) -> Self {
        Self {
            prefix: name.prefix.clone(),
            local: name.local.clone(),
        }
    }

    pub fn display(&self) -> String {
        match &self.prefix {
            Some(prefix) => format!("{prefix}:{}", self.local),
            None => self.local.clone(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct FunctionKey {
    pub name: QNameKey,
    pub arity: Arity,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BindingKind {
    Variable,
    Function,
    StdlibFunction,
    OverlayBinding,
    SchemaType,
    TemplateRef,
    StateSlot,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BindingEntry {
    pub id: BindingId,
    pub kind: BindingKind,
    pub name: QNameKey,
    pub arity: Option<Arity>,
    pub scope_id: ScopeId,
    pub declaring_range: Option<ByteRange>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct BindingTable {
    entries: Vec<BindingEntry>,
}

impl BindingTable {
    pub fn insert(&mut self, mut entry: BindingEntry) -> BindingId {
        let id = BindingId(self.entries.len().try_into().unwrap_or(u32::MAX));
        entry.id = id;
        self.entries.push(entry);
        id
    }

    pub fn get(&self, id: BindingId) -> Option<&BindingEntry> {
        self.entries.get(id.0 as usize)
    }

    pub fn entries(&self) -> &[BindingEntry] {
        &self.entries
    }
}

#[derive(Debug, Clone, Default)]
pub struct BindingSet {
    pub scope_id: ScopeId,
    pub variables: HashMap<QNameKey, BindingId>,
    pub functions: HashMap<FunctionKey, BindingId>,
    pub types: HashMap<QNameKey, BindingId>,
    pub namespaces: HashMap<String, String>,
    pub templates: HashMap<QNameKey, BindingId>,
    pub state_slots: HashMap<String, BindingId>,
    pub overlay: StdlibOverlay,
}

impl BindingSet {
    pub fn new(scope_id: ScopeId) -> Self {
        Self {
            scope_id,
            ..Default::default()
        }
    }

    pub fn insert_variable(&mut self, name: QNameKey, id: BindingId) -> Option<BindingId> {
        self.variables.insert(name, id)
    }

    pub fn insert_function(
        &mut self,
        name: QNameKey,
        arity: Arity,
        id: BindingId,
    ) -> Option<BindingId> {
        self.functions.insert(FunctionKey { name, arity }, id)
    }

    pub fn insert_type(&mut self, name: QNameKey, id: BindingId) -> Option<BindingId> {
        self.types.insert(name, id)
    }

    pub fn insert_template(&mut self, name: QNameKey, id: BindingId) -> Option<BindingId> {
        self.templates.insert(name, id)
    }

    pub fn insert_state_slot(
        &mut self,
        name: impl Into<String>,
        id: BindingId,
    ) -> Option<BindingId> {
        self.state_slots.insert(name.into(), id)
    }

    pub fn lookup(&self, name: &QName) -> Option<BindingId> {
        let key = QNameKey::from_qname(name);
        self.lookup_key(&key)
    }

    pub fn lookup_key(&self, key: &QNameKey) -> Option<BindingId> {
        self.variables
            .get(key)
            .copied()
            .or_else(|| self.first_function_named(key))
            .or_else(|| self.types.get(key).copied())
            .or_else(|| self.templates.get(key).copied())
            .or_else(|| self.state_slots.get(&key.local).copied())
            .or_else(|| self.overlay.lookup(key))
    }

    pub fn lookup_function(&self, name: &QName, arity: Arity) -> Option<BindingId> {
        let key = QNameKey::from_qname(name);
        self.functions
            .get(&FunctionKey {
                name: key.clone(),
                arity,
            })
            .copied()
            .or_else(|| self.overlay.lookup(&key))
    }

    pub fn lookup_type(&self, ty: &TypeExpr) -> Option<BindingId> {
        self.types.get(&QNameKey::from_qname(&ty.name)).copied()
    }

    fn first_function_named(&self, key: &QNameKey) -> Option<BindingId> {
        self.functions
            .iter()
            .find_map(|(candidate, binding)| (&candidate.name == key).then_some(*binding))
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolutionTraceEvent {
    pub name: QNameKey,
    pub binding_id: BindingId,
    pub binding_kind: BindingKind,
    pub scope_id: ScopeId,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Resolution {
    Resolved(BindingId),
    Unknown,
}

#[derive(Debug, Clone, Default)]
pub struct ResolutionReport {
    pub diagnostics: Vec<Diagnostic>,
    pub trace: Vec<ResolutionTraceEvent>,
}

#[derive(Debug, Clone)]
pub struct NameResolver {
    pub table: BindingTable,
    pub sites: Vec<BindingSet>,
    pub diagnostics: Vec<Diagnostic>,
    pub trace: Vec<ResolutionTraceEvent>,
    next_scope_id: ScopeId,
}

impl Default for NameResolver {
    fn default() -> Self {
        Self {
            table: BindingTable::default(),
            sites: Vec::new(),
            diagnostics: Vec::new(),
            trace: Vec::new(),
            next_scope_id: 1,
        }
    }
}

impl NameResolver {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_sites(sites: Vec<BindingSet>) -> Self {
        Self {
            sites,
            ..Default::default()
        }
    }

    pub fn push_site(&mut self, site: BindingSet) {
        self.sites.push(site);
    }

    pub fn declare_binding(
        &mut self,
        set: &mut BindingSet,
        kind: BindingKind,
        name: QNameKey,
        arity: Option<Arity>,
        range: Option<ByteRange>,
    ) -> BindingId {
        let id = self.table.insert(BindingEntry {
            id: BindingId(u32::MAX),
            kind,
            name: name.clone(),
            arity,
            scope_id: set.scope_id,
            declaring_range: range,
        });
        match (kind, arity) {
            (BindingKind::Variable, _) => {
                set.insert_variable(name, id);
            }
            (BindingKind::Function | BindingKind::StdlibFunction, Some(arity)) => {
                set.insert_function(name, arity, id);
            }
            (BindingKind::SchemaType, _) => {
                set.insert_type(name, id);
            }
            (BindingKind::TemplateRef, _) => {
                set.insert_template(name, id);
            }
            (BindingKind::StateSlot, _) => {
                set.insert_state_slot(name.local, id);
            }
            (BindingKind::OverlayBinding, _) => {}
            (BindingKind::Function | BindingKind::StdlibFunction, None) => {
                set.insert_function(name, Arity(0), id);
            }
        }
        id
    }

    pub fn resolve(&mut self, name: &QName) -> Resolution {
        let sites = self.sites.clone();
        self.resolve_in_sites(name, &sites)
    }

    pub fn resolve_function(&mut self, name: &QName, arity: Arity) -> Resolution {
        let sites = self.sites.clone();
        self.resolve_function_in_sites(name, arity, &sites)
    }

    pub fn resolve_type(&mut self, ty: &TypeExpr) -> Resolution {
        let sites = self.sites.clone();
        self.resolve_type_in_sites(ty, &sites)
    }

    pub fn resolve_surface_module(
        &mut self,
        module: &SurfaceModule,
        import_policy: &ImportPolicy,
    ) -> ResolutionReport {
        let diagnostics_start = self.diagnostics.len();
        let trace_start = self.trace.len();
        let mut module_set = BindingSet::new(0);
        self.declare_module_bindings(&mut module_set, module);
        for node in &module.nodes {
            if let SurfaceNode::Import(import) = node {
                match import_policy.resolve_import(import) {
                    Ok(resolved) => {
                        if let Some(alias) = &import.alias {
                            module_set.namespaces.insert(alias.clone(), resolved.uri);
                        }
                    }
                    Err(diagnostic) => self.diagnostics.push(*diagnostic),
                }
            }
        }

        let mut sites = vec![module_set.clone()];
        sites.extend(self.sites.clone());
        for node in &module.nodes {
            match node {
                SurfaceNode::DeclareVariable(var) => self.resolve_variable_decl(var, &sites),
                SurfaceNode::DeclareFunction(fun) => self.resolve_function_decl(fun, &sites),
                SurfaceNode::Expression(expr) => self.resolve_expression(expr, &sites),
                SurfaceNode::Module(_) | SurfaceNode::Import(_) => {}
            }
        }
        ResolutionReport {
            diagnostics: self.diagnostics[diagnostics_start..].to_vec(),
            trace: self.trace[trace_start..].to_vec(),
        }
    }

    pub fn resolve_import(
        &mut self,
        import: &ImportDecl,
        policy: &ImportPolicy,
    ) -> Option<ImportResolution> {
        match policy.resolve_import(import) {
            Ok(import) => Some(import),
            Err(diagnostic) => {
                self.diagnostics.push(*diagnostic);
                None
            }
        }
    }

    fn declare_module_bindings(&mut self, set: &mut BindingSet, module: &SurfaceModule) {
        for node in &module.nodes {
            match node {
                SurfaceNode::DeclareVariable(var) => {
                    self.declare_binding(
                        set,
                        BindingKind::Variable,
                        QNameKey::from_qname(&var.name),
                        None,
                        Some(var.range),
                    );
                }
                SurfaceNode::DeclareFunction(fun) => {
                    self.declare_binding(
                        set,
                        BindingKind::Function,
                        QNameKey::from_qname(&fun.name),
                        Some(Arity(fun.params.len().try_into().unwrap_or(u32::MAX))),
                        Some(fun.range),
                    );
                }
                _ => {}
            }
        }
    }

    fn resolve_variable_decl(&mut self, var: &VariableDecl, sites: &[BindingSet]) {
        self.resolve_expression(&var.value, sites);
    }

    fn resolve_function_decl(&mut self, fun: &FunctionDecl, sites: &[BindingSet]) {
        let mut local = BindingSet::new(self.allocate_scope_id());
        for param in &fun.params {
            self.declare_param(&mut local, param);
        }
        let mut body_sites = vec![local];
        body_sites.extend_from_slice(sites);
        self.resolve_expression(&fun.body, &body_sites);
    }

    fn declare_param(&mut self, local: &mut BindingSet, param: &FunctionParam) {
        self.declare_binding(
            local,
            BindingKind::Variable,
            QNameKey::from_qname(&param.name),
            None,
            Some(param.name.range),
        );
    }

    fn resolve_expression(&mut self, expr: &Expression, sites: &[BindingSet]) {
        match expr {
            Expression::Literal(_, _) | Expression::LeadingDot(_) => {}
            Expression::Path { steps, .. } => {
                for step in steps {
                    if let crate::parser::PathStep::Axis { predicates, .. } = step {
                        for predicate in predicates {
                            self.resolve_expression(predicate, sites);
                        }
                    }
                }
            }
            Expression::Name(name, _) => {
                self.resolve_in_sites(name, sites);
            }
            Expression::Pipeline { source, steps, .. } => {
                self.resolve_expression(source, sites);
                for step in steps {
                    match step {
                        PipelineStep::Named { name, args, .. } => {
                            self.resolve_function_in_sites(
                                name,
                                Arity(args.len().try_into().unwrap_or(u32::MAX)),
                                sites,
                            );
                            for arg in args {
                                self.resolve_expression(arg, sites);
                            }
                        }
                        PipelineStep::Lambda { lambda, .. } => {
                            self.resolve_expression(lambda, sites)
                        }
                    }
                }
            }
            Expression::BinaryOp { lhs, rhs, .. } | Expression::SetOp { lhs, rhs, .. } => {
                self.resolve_expression(lhs, sites);
                self.resolve_expression(rhs, sites);
            }
            Expression::UnaryOp { operand, .. } => self.resolve_expression(operand, sites),
            Expression::If {
                cond,
                then_branch,
                else_branch,
                ..
            } => {
                self.resolve_expression(cond, sites);
                self.resolve_expression(then_branch, sites);
                self.resolve_expression(else_branch, sites);
            }
            Expression::Let {
                name, value, body, ..
            } => {
                self.resolve_expression(value, sites);
                let mut local = BindingSet::new(self.allocate_scope_id());
                self.declare_binding(
                    &mut local,
                    BindingKind::Variable,
                    QNameKey::from_qname(name),
                    None,
                    Some(name.range),
                );
                let mut body_sites = vec![local];
                body_sites.extend_from_slice(sites);
                self.resolve_expression(body, &body_sites);
            }
            Expression::For {
                var, source, body, ..
            } => {
                self.resolve_expression(source, sites);
                let mut local = BindingSet::new(self.allocate_scope_id());
                self.declare_binding(
                    &mut local,
                    BindingKind::Variable,
                    QNameKey::from_qname(var),
                    None,
                    Some(var.range),
                );
                let mut body_sites = vec![local];
                body_sites.extend_from_slice(sites);
                self.resolve_expression(body, &body_sites);
            }
            Expression::Quantified {
                var,
                source,
                predicate,
                ..
            } => {
                self.resolve_expression(source, sites);
                let mut local = BindingSet::new(self.allocate_scope_id());
                self.declare_binding(
                    &mut local,
                    BindingKind::Variable,
                    QNameKey::from_qname(var),
                    None,
                    Some(var.range),
                );
                let mut predicate_sites = vec![local];
                predicate_sites.extend_from_slice(sites);
                self.resolve_expression(predicate, &predicate_sites);
            }
            Expression::Record { entries, .. } => {
                for entry in entries {
                    self.resolve_expression(&entry.value, sites);
                }
            }
            Expression::Sequence { items, .. } => {
                for item in items {
                    self.resolve_expression(item, sites);
                }
            }
            Expression::Call { callee, args, .. } => {
                if let Expression::Name(name, _) = callee.as_ref() {
                    self.resolve_function_in_sites(
                        name,
                        Arity(args.len().try_into().unwrap_or(u32::MAX)),
                        sites,
                    );
                } else {
                    self.resolve_expression(callee, sites);
                }
                for arg in args {
                    self.resolve_expression(arg, sites);
                }
            }
            Expression::InstanceOf { value, ty, .. }
            | Expression::CastAs { value, ty, .. }
            | Expression::TreatAs { value, ty, .. } => {
                self.resolve_expression(value, sites);
                self.resolve_type_in_sites(ty, sites);
            }
        }
    }

    fn resolve_in_sites(&mut self, name: &QName, sites: &[BindingSet]) -> Resolution {
        for site in sites {
            if let Some(binding_id) = site.lookup(name) {
                self.trace_binding(QNameKey::from_qname(name), binding_id, site.scope_id);
                return Resolution::Resolved(binding_id);
            }
        }
        self.diagnostics.push(diagnostic(
            "cem.ql.unknown_variable",
            format!(
                "unknown variable `{}`",
                QNameKey::from_qname(name).display()
            ),
            name.range,
            Severity::Error,
        ));
        Resolution::Unknown
    }

    fn resolve_function_in_sites(
        &mut self,
        name: &QName,
        arity: Arity,
        sites: &[BindingSet],
    ) -> Resolution {
        for site in sites {
            if let Some(binding_id) = site.lookup_function(name, arity) {
                self.trace_binding(QNameKey::from_qname(name), binding_id, site.scope_id);
                return Resolution::Resolved(binding_id);
            }
        }
        self.diagnostics.push(diagnostic(
            "cem.ql.unknown_function",
            format!(
                "unknown function `{}` with arity {}",
                QNameKey::from_qname(name).display(),
                arity.0
            ),
            name.range,
            Severity::Error,
        ));
        Resolution::Unknown
    }

    fn resolve_type_in_sites(&mut self, ty: &TypeExpr, sites: &[BindingSet]) -> Resolution {
        for site in sites {
            if let Some(binding_id) = site.lookup_type(ty) {
                self.trace_binding(QNameKey::from_qname(&ty.name), binding_id, site.scope_id);
                return Resolution::Resolved(binding_id);
            }
        }
        self.diagnostics.push(diagnostic(
            "cem.ql.unknown_type",
            format!(
                "unknown type `{}`",
                QNameKey::from_qname(&ty.name).display()
            ),
            ty.range,
            Severity::Error,
        ));
        Resolution::Unknown
    }

    fn trace_binding(&mut self, name: QNameKey, binding_id: BindingId, scope_id: ScopeId) {
        let binding_kind = self
            .table
            .get(binding_id)
            .map(|entry| entry.kind)
            .unwrap_or(BindingKind::OverlayBinding);
        self.trace.push(ResolutionTraceEvent {
            name,
            binding_id,
            binding_kind,
            scope_id,
        });
    }

    fn allocate_scope_id(&mut self) -> ScopeId {
        let scope_id = self.next_scope_id;
        self.next_scope_id = self.next_scope_id.saturating_add(1);
        scope_id
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ImportPolicy {
    allowed_schemes: BTreeSet<String>,
    registered_urn_cem: BTreeSet<String>,
}

impl ImportPolicy {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn allow_scheme(mut self, scheme: impl Into<String>) -> Result<Self, Box<Diagnostic>> {
        let scheme = scheme.into();
        if is_reserved_grant(&scheme) {
            return Err(Box::new(reserved_scheme_diagnostic(
                &scheme,
                ByteRange::new(0, 0),
            )));
        }
        self.allowed_schemes.insert(scheme);
        Ok(self)
    }

    pub fn register_urn_cem(mut self, uri: impl Into<String>) -> Self {
        self.registered_urn_cem.insert(uri.into());
        self
    }

    pub fn resolve_import(&self, import: &ImportDecl) -> Result<ImportResolution, Box<Diagnostic>> {
        if import.uri.starts_with("cem:") {
            return Ok(ImportResolution {
                uri: import.uri.clone(),
                kind: ImportKind::PlatformStdlib,
            });
        }
        if import.uri.starts_with("urn:cem:") {
            if self.registered_urn_cem.contains(&import.uri) {
                return Ok(ImportResolution {
                    uri: import.uri.clone(),
                    kind: ImportKind::PluginRegistry,
                });
            }
            return Err(Box::new(diagnostic(
                "cem.ql.import_unresolved",
                format!("import `{}` is not registered", import.uri),
                import.range,
                Severity::Error,
            )));
        }
        let Some(scheme) = scheme_of(&import.uri) else {
            return Err(Box::new(diagnostic(
                "cem.ql.import_denied",
                format!("import `{}` has no URI scheme grant", import.uri),
                import.range,
                Severity::Warning,
            )));
        };
        if self.allowed_schemes.contains(scheme) {
            Ok(ImportResolution {
                uri: import.uri.clone(),
                kind: ImportKind::External,
            })
        } else {
            Err(Box::new(diagnostic(
                "cem.ql.import_denied",
                format!("import `{}` denied by scope policy", import.uri),
                import.range,
                Severity::Warning,
            )))
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImportResolution {
    pub uri: String,
    pub kind: ImportKind,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ImportKind {
    PlatformStdlib,
    PluginRegistry,
    External,
}

fn scheme_of(uri: &str) -> Option<&str> {
    uri.split_once(':').map(|(scheme, _)| scheme)
}

fn is_reserved_grant(scheme: &str) -> bool {
    scheme == "cem" || scheme == "cem:" || scheme == "urn:cem" || scheme.starts_with("urn:cem:")
}

fn reserved_scheme_diagnostic(scheme: &str, range: ByteRange) -> Diagnostic {
    diagnostic(
        "cem.ql.reserved_scheme",
        format!("scope policy cannot grant reserved scheme `{scheme}`"),
        range,
        Severity::Error,
    )
}

fn diagnostic(
    code: &'static str,
    message: impl Into<String>,
    range: ByteRange,
    severity: Severity,
) -> Diagnostic {
    Diagnostic {
        uri: None,
        line: None,
        column: None,
        byte_offset: Some(range.start),
        code: code.to_owned(),
        severity,
        message: message.into(),
        node: None,
        source_map: None,
    }
}
