//! Preparation of existing host controls, before runtime region activation.
use super::{CemQlSchemaDeclarationHost, SchemaScopePreparation};
use cem_ml::{
    parser::CemAstNode,
    schema::{
        declaration_references::SchemaDeclarationNode,
        input_references::native_attribute_expression,
        reference_traversal::ReferenceTraversalLimits,
        scope_controls::{
            decode_schema_host_control, SchemaHostControl, SchemaHostControlError, SchemaHostSource,
        },
    },
    value::reference_resolution::ReferenceResolutionError,
};

#[derive(Debug, Clone)]
pub struct PreparedSchemaHostControl {
    pub control: SchemaHostControl,
    /// URI loading is a separate consumer stage. None retains its authored URI
    /// without treating it as a query or assuming a ready inherited schema.
    pub preparation: Option<SchemaScopePreparation>,
}
impl PreparedSchemaHostControl {
    pub fn is_ready(&self) -> bool {
        self.preparation
            .as_ref()
            .is_some_and(SchemaScopePreparation::is_ready)
    }
}
#[derive(Debug)]
pub enum SchemaHostPreparationError {
    Control(SchemaHostControlError),
    Resolution(ReferenceResolutionError),
}
impl std::fmt::Display for SchemaHostPreparationError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Control(error) => write!(formatter, "Schema host control: {:?}", error.issue),
            Self::Resolution(error) => std::fmt::Display::fmt(error, formatter),
        }
    }
}
impl std::error::Error for SchemaHostPreparationError {}

impl CemQlSchemaDeclarationHost {
    /// Decode an original host using attached namespace metadata. Literal
    /// selectors are implicit consumer reference occurrences on the attribute;
    /// native references and expression slots retain their own original handle.
    /// Selection, chains, admission and compilation share the established bounds,
    /// contexts and directed grants. This neither loads URIs nor installs scopes,
    /// grants, body policies or enclosing host-attribute validation contracts.
    pub fn prepare_schema_host_control(
        &mut self,
        schema_uri: &str,
        host: SchemaDeclarationNode,
        limits: ReferenceTraversalLimits,
    ) -> Result<Option<PreparedSchemaHostControl>, SchemaHostPreparationError> {
        let Some(control) =
            decode_schema_host_control(host, |source| self.captured_expanded_name(source).cloned())
                .map_err(SchemaHostPreparationError::Control)?
        else {
            return Ok(None);
        };
        let preparation = self
            .prepare_decoded_host_control(schema_uri, &control, limits)
            .map_err(SchemaHostPreparationError::Resolution)?;
        Ok(Some(PreparedSchemaHostControl {
            control,
            preparation,
        }))
    }

    pub(super) fn prepare_decoded_host_control(
        &mut self,
        schema_uri: &str,
        control: &SchemaHostControl,
        limits: ReferenceTraversalLimits,
    ) -> Result<Option<SchemaScopePreparation>, ReferenceResolutionError> {
        prepare_decoded_host_control(self, schema_uri, control, limits)
    }
}

pub(super) fn prepare_decoded_host_control<H: super::scope_preparation::SchemaPreparationHost>(
    host: &mut H,
    schema_uri: &str,
    control: &SchemaHostControl,
    limits: ReferenceTraversalLimits,
) -> Result<Option<SchemaScopePreparation>, ReferenceResolutionError> {
    let root = match &control.source {
        SchemaHostSource::Uri(_) => None,
        SchemaHostSource::LiteralSelector(expression) => {
            let mut root = host.source_reference(control.attribute.clone());
            root.selector_expression = Some(expression.clone());
            Some(root)
        }
        SchemaHostSource::NativeSelector(source) => {
            let mut root = host.source_reference(source.clone());
            if !matches!(source.node(), CemAstNode::Reference { .. }) {
                root.selector_expression = native_attribute_expression(source)
                    .and_then(|occurrence| occurrence.expression);
            }
            Some(root)
        }
    };
    let preparation = root
        .map(|root| {
            super::scope_preparation::prepare_schema_scope_node(host, schema_uri, root, limits)
        })
        .transpose()?;
    Ok(preparation)
}
