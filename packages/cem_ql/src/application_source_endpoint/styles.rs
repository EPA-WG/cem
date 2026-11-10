//! Styles retained by the same original publication as the application frames.
use super::*;
use crate::{
    application_envelope::v4::VerifiedApplicationEnvelope,
    application_runtime::{
        owners::{declaration_stylesheet_scope, stylesheet_receipt},
        MAX_INPUT_BYTES,
    },
    retained_template::{RetainedTemplate, StylesheetLoadOptions},
};
use cem_ml::{
    css_imports::CssImportResponsePolicy, module_resolution::CemModuleUrlResolutionCapability,
};

const CONSUMER: &str = "application-publication";
const MAX_OCCURRENCES: usize = 64;

struct DeclarationStyles {
    _owner: RetainedTemplate,
    receipt: String,
}

pub(super) struct ApplicationStyles {
    capability: CemModuleUrlResolutionCapability,
    stamp: String,
    declarations: BTreeMap<String, DeclarationStyles>,
    active: AtomicBool,
    _lease: Option<NativeResourceLease>,
}

impl ApplicationStyles {
    pub(super) fn stage(
        application: &VerifiedApplicationEnvelope,
        capability: CemModuleUrlResolutionCapability,
        response_policy: CssImportResponsePolicy,
        control: &OperationControl,
    ) -> Result<Self, String> {
        let stamp = capability
            .cache_identity()
            .filter(|stamp| !stamp.is_empty())
            .ok_or(INVALID)?;
        let mut declarations = BTreeMap::new();
        let mut remaining_occurrences = MAX_OCCURRENCES;
        let mut remaining_bytes = MAX_INPUT_BYTES;
        for tag in application.component_order() {
            let artifact = application.artifact(tag).ok_or(INVALID)?;
            if artifact.stylesheets.is_empty() {
                continue;
            }
            let declaration = application.declaration(tag).ok_or(INVALID)?;
            let demand = application
                .requirements()
                .declarations
                .get(tag)
                .ok_or(INVALID)?;
            if !matches!(
                declaration.template_format.as_str(),
                "cem-template-artifact/2" | "cem-template-artifact/3"
            ) || declaration.module_map != artifact.module_map
            {
                return Err(INVALID.into());
            }
            remaining_occurrences = remaining_occurrences
                .checked_sub(artifact.stylesheets.len())
                .ok_or(INVALID)?;
            let mut owner = RetainedTemplate::new(artifact.clone());
            remaining_bytes = remaining_bytes.checked_sub(2).ok_or(INVALID)?;
            let mut receipt = String::from("[");
            for (index, sheet) in artifact.stylesheets.iter().enumerate() {
                control
                    .check_scope(ROOT_EXECUTION_SCOPE_ID)
                    .map_err(|_| INVALID)?;
                if sheet.scope.is_some() && sheet.scope != declaration.scope {
                    return Err(INVALID.into());
                }
                let record = owner.load_compiled_stylesheet(
                    StylesheetLoadOptions {
                        consumer: CONSUMER.into(),
                        index,
                        scope: declaration_stylesheet_scope(tag, sheet.scope.as_deref()),
                        declaration_identity: demand.metadata_hash.clone(),
                        base_url: declaration.resource_base_url.clone(),
                        capability: capability.clone(),
                        response_policy,
                    },
                    remaining_bytes,
                )?;
                if record.import_closure().sheets().iter().any(|sheet| {
                    sheet
                        .resources
                        .references
                        .iter()
                        .any(|reference| reference.resolution.is_err())
                }) {
                    return Err(INVALID.into());
                }
                let entry = serde_json::json!({
                    "index": index, "scope": sheet.scope, "stylesheet": stylesheet_receipt(&record)?,
                }).to_string();
                remaining_bytes = remaining_bytes
                    .checked_sub(entry.len() + usize::from(index > 0))
                    .ok_or(INVALID)?;
                if index > 0 {
                    receipt.push(',');
                }
                receipt.push_str(&entry);
            }
            receipt.push(']');
            declarations.insert(
                tag.to_owned(),
                DeclarationStyles {
                    _owner: owner,
                    receipt,
                },
            );
        }
        control
            .check_scope(ROOT_EXECUTION_SCOPE_ID)
            .map_err(|_| INVALID)?;
        let styles = Self {
            capability,
            stamp,
            declarations,
            active: AtomicBool::new(true),
            _lease: None,
        };
        if !styles.current() {
            return Err(INVALID.into());
        }
        Ok(styles)
    }

    pub(super) fn current(&self) -> bool {
        if self.active.load(Ordering::Acquire)
            && self.capability.cache_identity().as_deref() != Some(self.stamp.as_str())
        {
            self.active.store(false, Ordering::Release);
        }
        self.active.load(Ordering::Acquire)
    }

    pub(super) fn enroll(&mut self, generation: &Arc<NativeGeneration>) -> Result<(), String> {
        self._lease = Some(
            generation
                .resource(NativeRootKind::Registry)
                .map_err(|_| INVALID)?,
        );
        Ok(())
    }

    fn receipt(&self, tag: &str) -> Option<&str> {
        self.declarations
            .get(tag)
            .map(|declaration| declaration.receipt.as_str())
    }
}

impl ApplicationSourceSession {
    /// Owned CSS and scoping metadata from the original live publication.
    pub fn stylesheets_json(&self, tag: &str) -> Result<String, String> {
        let _entry = self.enter()?;
        let artifact = self
            .prepared
            .current()?
            .envelope()
            .artifact(tag)
            .ok_or(INVALID)?;
        let receipt = if artifact.stylesheets.is_empty() {
            "[]"
        } else {
            self.prepared
                .styles
                .as_ref()
                .and_then(|styles| styles.receipt(tag))
                .ok_or(INVALID)?
        }
        .to_owned();
        self.check()?;
        Ok(receipt)
    }
}
