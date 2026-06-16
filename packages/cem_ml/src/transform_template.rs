//! Transform template adapter registry.
//!
//! Template documents are versioned independently from the base CEM-ML
//! document/AST language. This registry keeps template content-type and schema
//! dispatch pluggable so CEM-native template iterations can ship as built-in
//! adapters or be installed by hosts at runtime.

use crate::engine::{FormatIdentity, TransformTemplateKind};
use std::fmt;
use std::sync::Arc;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TransformTemplateAdapterSelection {
    pub adapter_id: &'static str,
    pub kind: TransformTemplateKind,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TransformTemplateAdapterResolution {
    Matched(TransformTemplateAdapterSelection),
    Ambiguous(Vec<&'static str>),
    Unsupported,
}

pub trait TransformTemplateAdapter: Send + Sync {
    fn id(&self) -> &'static str;
    fn kind(&self) -> TransformTemplateKind;
    fn matches_template(&self, identity: &FormatIdentity) -> bool;
}

#[derive(Clone, Default)]
pub struct TransformTemplateAdapterRegistry {
    adapters: Vec<Arc<dyn TransformTemplateAdapter>>,
}

impl fmt::Debug for TransformTemplateAdapterRegistry {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("TransformTemplateAdapterRegistry")
            .field("adapter_count", &self.adapters.len())
            .finish()
    }
}

impl TransformTemplateAdapterRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_builtin_adapters() -> Self {
        let mut registry = Self::new();
        registry.register(StaticTransformTemplateAdapter::new(
            "cem-native-template",
            TransformTemplateKind::CemNative,
            &[
                "application/cem+xml",
                "application/cem",
                "text/cem",
                "text/cem-ml",
            ],
            &[crate::schema::ir::CEM_CORE_NAMESPACE],
            &[crate::schema::ir::CEM_CORE_NAMESPACE],
        ));
        registry.register(StaticTransformTemplateAdapter::new(
            "xslt-template",
            TransformTemplateKind::Xslt,
            crate::legacy_custom_element::TEMPLATE_CONTENT_TYPES,
            &[],
            &[crate::schema::xslt::XSL_NAMESPACE],
        ));
        registry
    }

    pub fn register(&mut self, adapter: impl TransformTemplateAdapter + 'static) {
        self.adapters.push(Arc::new(adapter));
    }

    pub fn register_arc(&mut self, adapter: Arc<dyn TransformTemplateAdapter>) {
        self.adapters.push(adapter);
    }

    pub fn select(&self, identity: &FormatIdentity) -> TransformTemplateAdapterResolution {
        let matches = self
            .adapters
            .iter()
            .filter(|adapter| adapter.matches_template(identity))
            .map(|adapter| TransformTemplateAdapterSelection {
                adapter_id: adapter.id(),
                kind: adapter.kind(),
            })
            .collect::<Vec<_>>();

        match matches.as_slice() {
            [selection] => TransformTemplateAdapterResolution::Matched(selection.clone()),
            [] => TransformTemplateAdapterResolution::Unsupported,
            many => TransformTemplateAdapterResolution::Ambiguous(
                many.iter().map(|selection| selection.adapter_id).collect(),
            ),
        }
    }
}

#[derive(Debug, Clone)]
pub struct StaticTransformTemplateAdapter {
    id: &'static str,
    kind: TransformTemplateKind,
    content_types: Vec<String>,
    schemas: Vec<String>,
    namespaces: Vec<String>,
}

impl StaticTransformTemplateAdapter {
    pub fn new(
        id: &'static str,
        kind: TransformTemplateKind,
        content_types: &[&str],
        schemas: &[&str],
        namespaces: &[&str],
    ) -> Self {
        Self {
            id,
            kind,
            content_types: content_types
                .iter()
                .map(|content_type| content_type_essence(content_type))
                .collect(),
            schemas: schemas
                .iter()
                .map(|schema| schema.trim().to_owned())
                .collect(),
            namespaces: namespaces
                .iter()
                .map(|namespace| namespace.trim().to_owned())
                .collect(),
        }
    }
}

impl TransformTemplateAdapter for StaticTransformTemplateAdapter {
    fn id(&self) -> &'static str {
        self.id
    }

    fn kind(&self) -> TransformTemplateKind {
        self.kind
    }

    fn matches_template(&self, identity: &FormatIdentity) -> bool {
        if let Some(content_type) = identity.content_type.as_deref() {
            return self
                .content_types
                .iter()
                .any(|allowed| allowed == &content_type_essence(content_type));
        }

        let schema = identity
            .schema
            .as_deref()
            .map(str::trim)
            .unwrap_or_default();
        if !schema.is_empty() {
            return self.schemas.iter().any(|allowed| allowed == schema);
        }

        identity
            .default_namespace
            .as_deref()
            .is_some_and(|uri| self.namespaces.iter().any(|allowed| allowed == uri))
            || identity
                .namespaces
                .values()
                .any(|uri| self.namespaces.iter().any(|allowed| allowed == uri))
    }
}

fn content_type_essence(content_type: &str) -> String {
    content_type
        .split(';')
        .next()
        .unwrap_or(content_type)
        .trim()
        .to_ascii_lowercase()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builtins_select_cem_native_and_xslt_template_adapters() {
        let registry = TransformTemplateAdapterRegistry::with_builtin_adapters();
        let cem = FormatIdentity {
            content_type: Some("text/cem-ml; charset=utf-8".to_owned()),
            ..FormatIdentity::default()
        };
        let xslt = FormatIdentity {
            default_namespace: Some(crate::schema::xslt::XSL_NAMESPACE.to_owned()),
            ..FormatIdentity::default()
        };

        assert_eq!(
            registry.select(&cem),
            TransformTemplateAdapterResolution::Matched(TransformTemplateAdapterSelection {
                adapter_id: "cem-native-template",
                kind: TransformTemplateKind::CemNative,
            })
        );
        assert_eq!(
            registry.select(&xslt),
            TransformTemplateAdapterResolution::Matched(TransformTemplateAdapterSelection {
                adapter_id: "xslt-template",
                kind: TransformTemplateKind::Xslt,
            })
        );
    }

    #[test]
    fn runtime_adapter_can_claim_new_cem_native_template_schema() {
        let mut registry = TransformTemplateAdapterRegistry::new();
        registry.register(StaticTransformTemplateAdapter::new(
            "cem-native-template-v2",
            TransformTemplateKind::CemNative,
            &["application/vnd.cem.template+cem;version=2"],
            &["https://cem.dev/ns/template/cem-native/2"],
            &[],
        ));
        let identity = FormatIdentity {
            schema: Some("https://cem.dev/ns/template/cem-native/2".to_owned()),
            ..FormatIdentity::default()
        };

        assert_eq!(
            registry.select(&identity),
            TransformTemplateAdapterResolution::Matched(TransformTemplateAdapterSelection {
                adapter_id: "cem-native-template-v2",
                kind: TransformTemplateKind::CemNative,
            })
        );
    }

    #[test]
    fn ambiguous_template_adapter_matches_are_reported() {
        let mut registry = TransformTemplateAdapterRegistry::new();
        registry.register(StaticTransformTemplateAdapter::new(
            "one",
            TransformTemplateKind::CemNative,
            &["text/cem-ml"],
            &[],
            &[],
        ));
        registry.register(StaticTransformTemplateAdapter::new(
            "two",
            TransformTemplateKind::CemNative,
            &["text/cem-ml"],
            &[],
            &[],
        ));
        let identity = FormatIdentity {
            content_type: Some("text/cem-ml".to_owned()),
            ..FormatIdentity::default()
        };

        assert_eq!(
            registry.select(&identity),
            TransformTemplateAdapterResolution::Ambiguous(vec!["one", "two"])
        );
    }
}
