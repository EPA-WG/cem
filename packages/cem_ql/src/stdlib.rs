//! Tier A standard-library registry shell.

pub mod cemml;
pub mod content_types;
pub mod datetime;
pub mod dom;
pub mod numbers;
pub mod report;
pub mod sequence;
pub mod state;
pub mod strings;
pub mod template;

#[derive(Debug, Clone, Default)]
pub struct ModuleRegistry {
    pub modules: Vec<&'static str>,
}
