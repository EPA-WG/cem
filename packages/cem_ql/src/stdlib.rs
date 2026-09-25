//! Tier A standard-library registry.

pub mod cemml;
pub mod cemt;
pub mod content_types;
pub mod data;
pub mod datetime;
pub mod dom;
pub mod items;
pub mod modules;
pub mod native;
pub mod numbers;
pub mod records;
pub mod report;
pub mod sequence;
pub mod state;
pub mod strings;
pub mod template;
pub mod user;
pub mod url;
pub mod url_origin;
pub mod url_params;
pub mod url_setters;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tier {
    A,
    B,
    C,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StdlibImplKind {
    Native,
    Macro,
    HostContext,
    Deferred,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StdlibFunction {
    pub module: &'static str,
    pub name: &'static str,
    pub min_arity: u8,
    pub max_arity: u8,
    pub tier: Tier,
    pub implementation: StdlibImplKind,
}

impl StdlibFunction {
    pub const fn native(module: &'static str, name: &'static str, arity: u8, tier: Tier) -> Self {
        Self {
            module,
            name,
            min_arity: arity,
            max_arity: arity,
            tier,
            implementation: StdlibImplKind::Native,
        }
    }

    pub const fn native_range(
        module: &'static str,
        name: &'static str,
        min_arity: u8,
        max_arity: u8,
        tier: Tier,
    ) -> Self {
        Self {
            module,
            name,
            min_arity,
            max_arity,
            tier,
            implementation: StdlibImplKind::Native,
        }
    }

    pub const fn macro_form(
        module: &'static str,
        name: &'static str,
        arity: u8,
        tier: Tier,
    ) -> Self {
        Self {
            module,
            name,
            min_arity: arity,
            max_arity: arity,
            tier,
            implementation: StdlibImplKind::Macro,
        }
    }

    pub const fn host_context(
        module: &'static str,
        name: &'static str,
        arity: u8,
        tier: Tier,
    ) -> Self {
        Self {
            module,
            name,
            min_arity: arity,
            max_arity: arity,
            tier,
            implementation: StdlibImplKind::HostContext,
        }
    }

    pub const fn host_context_range(
        module: &'static str,
        name: &'static str,
        min_arity: u8,
        max_arity: u8,
        tier: Tier,
    ) -> Self {
        Self {
            module,
            name,
            min_arity,
            max_arity,
            tier,
            implementation: StdlibImplKind::HostContext,
        }
    }

    pub const fn deferred(module: &'static str, name: &'static str, arity: u8, tier: Tier) -> Self {
        Self {
            module,
            name,
            min_arity: arity,
            max_arity: arity,
            tier,
            implementation: StdlibImplKind::Deferred,
        }
    }

    pub fn accepts_arity(self, arity: usize) -> bool {
        let arity = arity.min(u8::MAX as usize) as u8;
        arity >= self.min_arity && arity <= self.max_arity
    }
}

#[derive(Debug, Clone)]
pub struct ModuleRegistry {
    pub modules: Vec<&'static str>,
    pub functions: Vec<StdlibFunction>,
}

impl Default for ModuleRegistry {
    fn default() -> Self {
        Self::tier_a()
    }
}

impl ModuleRegistry {
    pub fn tier_a() -> Self {
        let functions = tier_a_functions();
        let mut modules = functions
            .iter()
            .map(|function| function.module)
            .collect::<Vec<_>>();
        modules.sort_unstable();
        modules.dedup();
        Self { modules, functions }
    }

    pub fn with_all_known() -> Self {
        #[cfg(test)]
        let _profile = crate::compile_profile::Span::new("stdlib/assemble-registry");
        let functions = all_known_functions();
        let mut modules = functions
            .iter()
            .map(|function| function.module)
            .collect::<Vec<_>>();
        modules.sort_unstable();
        modules.dedup();
        Self { modules, functions }
    }

    pub fn resolve(&self, module: &str, name: &str, arity: usize) -> Option<&StdlibFunction> {
        self.functions.iter().find(|function| {
            function.module == module && function.name == name && function.accepts_arity(arity)
        })
    }
}

pub fn tier_a_functions() -> Vec<StdlibFunction> {
    [
        sequence::FUNCTIONS,
        strings::FUNCTIONS,
        numbers::FUNCTIONS,
        datetime::FUNCTIONS,
        dom::FUNCTIONS,
        items::FUNCTIONS,
        modules::FUNCTIONS,
        records::FUNCTIONS,
        report::FUNCTIONS,
        state::FUNCTIONS,
        template::FUNCTIONS,
        cemml::FUNCTIONS,
        cemt::FUNCTIONS,
    ]
    .into_iter()
    .flatten()
    .copied()
    .collect()
}

pub fn all_known_functions() -> Vec<StdlibFunction> {
    let mut functions = tier_a_functions();
    functions.extend_from_slice(content_types::FUNCTIONS);
    functions.extend_from_slice(data::FUNCTIONS);
    functions.extend_from_slice(native::FUNCTIONS);
    functions.extend_from_slice(sequence::TIER_B_FUNCTIONS);
    functions.extend_from_slice(report::TIER_B_FUNCTIONS);
    functions.extend_from_slice(cemml::TIER_B_FUNCTIONS);
    functions.extend_from_slice(user::FUNCTIONS);
    functions
}
