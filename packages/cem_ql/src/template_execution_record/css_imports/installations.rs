//! Compiled output retains the exact recorded body roots and lexical policies.
mod output;
use super::*;
use crate::{
    render::TemplateArtifact, retained_template::RetainedTemplate, stylesheet_installation::*,
};
use cem_ml::{css_emission::CssManagedScope, scheduler::AbortSignal};
pub use output::*;
pub struct BodyStylesheetInstallation {
    pub body_uri: String,
    pub index: usize,
    pub scope: CssManagedScope,
}
pub struct RecordedBodyStylesheetInstallations {
    source: Arc<ExecutedTemplateCssImports>,
    _owners: Vec<RetainedTemplate>,
    set: CompiledStylesheetSet,
    released: AbortSignal,
}
pub struct RecordedBodyStylesheetEvidence {
    owner: Arc<RecordedBodyStylesheetInstallations>,
    execution: CompiledStylesheetEvidence,
}
impl ExecutedTemplateCssImports {
    pub fn compile_installations(
        self: &Arc<Self>,
        choices: &[BodyStylesheetInstallation],
        max_bytes: usize,
    ) -> Result<Arc<RecordedBodyStylesheetInstallations>, String> {
        self.validate_recorded_sources()?;
        self.document
            .validate_sources(&self.preparation.execution.capability, &self.snapshot)?;
        let count: usize = self.bodies.iter().map(|b| b.closures.len()).sum();
        if choices.len() != count || count > 256 {
            return Err("complete bounded body stylesheet installation choices required".into());
        }
        let mut position = 0;
        let mut sets = Vec::new();
        let mut owners = Vec::new();
        let mut bytes = 0usize;
        for (captured, body) in self
            .bodies
            .iter()
            .zip(&self.preparation.execution.bodies.bodies)
        {
            let mut owner = RetainedTemplate::new(TemplateArtifact {
                nodes: Vec::new(),
                stylesheets: body.stylesheets.clone(),
                module_map: None,
                diagnostics: Vec::new(),
            });
            for (index, closure) in captured.closures.iter().enumerate() {
                let choice = &choices[position];
                position += 1;
                if choice.body_uri != body.uri || choice.index != index {
                    return Err(
                        "body installation choices must preserve exact recorded occurrence order"
                            .into(),
                    );
                }
                let record = owner.retain_stylesheet(
                    "recorded-body",
                    index,
                    closure.closure_owner().clone(),
                    &choice.scope,
                    &body.uri,
                )?;
                let emission = record.emission()?;
                if !emission.diagnostics.is_empty() {
                    return Err(format!(
                        "complete body CSS emission required: {:?}",
                        emission.diagnostics
                    ));
                }
                for fragment in &emission.fragments {
                    bytes = bytes
                        .checked_add(fragment.fragment.text.len())
                        .ok_or("compiled body CSS byte overflow")?;
                    if bytes > max_bytes {
                        return Err("compiled body CSS byte bound exceeded".into());
                    }
                }
            }
            sets.push(owner.stylesheet_installations(
                "recorded-body",
                &(0..body.stylesheets.len()).collect::<Vec<_>>(),
            )?);
            owners.push(owner);
        }
        let result = Arc::new(RecordedBodyStylesheetInstallations {
            source: self.clone(),
            _owners: owners,
            set: CompiledStylesheetSet::join(sets)?,
            released: AbortSignal::new(),
        });
        result.validate_current()?;
        Ok(result)
    }
}
impl RecordedBodyStylesheetInstallations {
    fn validate_current(&self) -> Result<(), String> {
        if self.released.is_aborted() {
            return Err("recorded body installation plan released".into());
        }
        self.source.validate_recorded_sources()?;
        self.source.document.validate_sources(
            &self.source.preparation.execution.capability,
            &self.source.snapshot,
        )?;
        self.set.validate_current()
    }
    pub fn len(&self) -> usize {
        self.set.len()
    }
    pub fn is_empty(&self) -> bool {
        self.set.is_empty()
    }
    pub fn css(&self, index: usize) -> Result<String, String> {
        self.validate_current()?;
        self.set.css(index)
    }
    pub fn release(&self) {
        self.released.abort();
    }
    pub fn capture(
        self: &Arc<Self>,
        host: &mut CompiledStylesheetHost,
        observer: &mut impl CompiledStylesheetObserver,
        max_bytes: usize,
    ) -> Result<RecordedBodyStylesheetEvidence, String> {
        self.validate_current()?;
        let execution = host.capture(&self.set, observer, 256, max_bytes)?;
        if let Err(error) = self.validate_current() {
            host.invalidate()?;
            return Err(error);
        }
        host.validate(&execution, &self.set, observer)?;
        if let Err(error) = self.validate_current() {
            host.invalidate()?;
            return Err(error);
        }
        Ok(RecordedBodyStylesheetEvidence {
            owner: self.clone(),
            execution,
        })
    }
}
impl RecordedBodyStylesheetEvidence {
    pub fn validate(
        &self,
        host: &mut CompiledStylesheetHost,
        observer: &mut impl CompiledStylesheetObserver,
    ) -> Result<(), String> {
        let result = (|| {
            self.owner.validate_current()?;
            host.validate(&self.execution, &self.owner.set, observer)?;
            self.owner.validate_current()
        })();
        if result.is_err() {
            host.invalidate()?;
        }
        result
    }
    pub fn blockers(&self) -> Vec<String> {
        let mut result = self.owner.source.blockers().to_vec();
        result.extend(self.execution.blockers().iter().map(|s| s.to_string()));
        result.sort();
        result.dedup();
        result
    }
}
