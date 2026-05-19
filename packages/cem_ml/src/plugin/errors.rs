//! Public `PluginError` taxonomy (AC-PL-3, AC-PL-4, AC-PL-8, AC-PL-15,
//! AC-PL-17, AC-PL-19, AC-PL-20).

use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum PluginError {
    /// Registration rejected because the plugin's static evidence
    /// references capabilities outside the declared `requires` set
    /// (AC-PL-20).
    Capability {
        plugin: String,
        missing: Vec<String>,
    },
    /// Registration rejected because a `mutate`-mode plugin failed
    /// to set `supports_source_map = true` (AC-PL-4).
    SourceMapRequired { plugin: String },
    /// A descendant scope attempted to remove, reorder, or bypass an
    /// ancestor-installed plugin (AC-PL-8).
    Inheritance { plugin: String, scope: u32 },
    /// An `observe`-mode plugin returned output that differs from its
    /// input (AC-PL-3, AC-PL-V-6).
    ObserverViolation { plugin: String },
    /// Per-plugin time / memory budget exceeded (AC-PL-17).
    Budget {
        plugin: String,
        elapsed_ms: u128,
        budget_ms: u128,
    },
    /// Invocation was cancelled via `AbortSignal` (AC-PL-19, AC-A-7).
    Cancelled { plugin: String },
    /// Plugin's `invoke` returned an error.
    Invoke { plugin: String, message: String },
    /// Input content type doesn't match the descriptor's declared
    /// `input_content_types`.
    ContentTypeMismatch {
        plugin: String,
        expected: Vec<String>,
        found: String,
    },
}

impl PluginError {
    /// Stable diagnostic code per AC-PL-15.
    pub fn code(&self) -> &'static str {
        match self {
            PluginError::Capability { .. } => "cem.plugin.capability_error",
            PluginError::SourceMapRequired { .. } => "cem.plugin.source_map_required",
            PluginError::Inheritance { .. } => "cem.plugin.inheritance_error",
            PluginError::ObserverViolation { .. } => "cem.plugin.observer_violation",
            PluginError::Budget { .. } => "cem.plugin.budget_error",
            PluginError::Cancelled { .. } => "cem.plugin.cancelled",
            PluginError::Invoke { .. } => "cem.plugin.invoke_error",
            PluginError::ContentTypeMismatch { .. } => "cem.plugin.content_type_mismatch",
        }
    }

    /// Plugin name the error is attributed to (for diagnostics).
    pub fn plugin_name(&self) -> &str {
        match self {
            PluginError::Capability { plugin, .. }
            | PluginError::SourceMapRequired { plugin }
            | PluginError::Inheritance { plugin, .. }
            | PluginError::ObserverViolation { plugin }
            | PluginError::Budget { plugin, .. }
            | PluginError::Cancelled { plugin }
            | PluginError::Invoke { plugin, .. }
            | PluginError::ContentTypeMismatch { plugin, .. } => plugin,
        }
    }
}

impl fmt::Display for PluginError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PluginError::Capability { plugin, missing } => write!(
                f,
                "plugin `{plugin}` declares no permission for {missing:?}; declare them in `requires` or remove the AST reference"
            ),
            PluginError::SourceMapRequired { plugin } => write!(
                f,
                "plugin `{plugin}` is mode=mutate but did not set supports_source_map=true (AC-PL-4)"
            ),
            PluginError::Inheritance { plugin, scope } => write!(
                f,
                "scope `{scope}` may not remove, reorder, or bypass ancestor plugin `{plugin}` (AC-PL-8)"
            ),
            PluginError::ObserverViolation { plugin } => write!(
                f,
                "observe-mode plugin `{plugin}` produced an output that differs from its input (AC-PL-3)"
            ),
            PluginError::Budget {
                plugin,
                elapsed_ms,
                budget_ms,
            } => write!(
                f,
                "plugin `{plugin}` exceeded budget: elapsed {elapsed_ms}ms > budget {budget_ms}ms (AC-PL-17)"
            ),
            PluginError::Cancelled { plugin } => write!(
                f,
                "plugin `{plugin}` was cancelled via AbortSignal (AC-PL-19)"
            ),
            PluginError::Invoke { plugin, message } => write!(
                f,
                "plugin `{plugin}` invocation failed: {message}"
            ),
            PluginError::ContentTypeMismatch {
                plugin,
                expected,
                found,
            } => write!(
                f,
                "plugin `{plugin}` cannot accept content-type `{found}`; expected one of {expected:?}"
            ),
        }
    }
}

impl std::error::Error for PluginError {}
