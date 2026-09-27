use super::*;
use cem_ml::{
    css_imports::{CssImportLimits, CssImportRequest, CssImportResponsePolicy},
    module_resolution::CemModuleUrlResolutionCapability,
    resolver::ResolvedRead,
};

#[derive(Debug)]
pub enum StylesheetLoadError {
    Control(String),
    Import(Box<cem_ml::css_imports::CssImportFailure>),
}
impl From<String> for StylesheetLoadError {
    fn from(message: String) -> Self {
        Self::Control(message)
    }
}
impl From<&str> for StylesheetLoadError {
    fn from(message: &str) -> Self {
        Self::Control(message.into())
    }
}
impl From<cem_ml::css_imports::CssImportFailure> for StylesheetLoadError {
    fn from(error: cem_ml::css_imports::CssImportFailure) -> Self {
        Self::Import(Box::new(error))
    }
}
impl std::fmt::Display for StylesheetLoadError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Control(message) => f.write_str(message),
            Self::Import(error) => error.fmt(f),
        }
    }
}
impl std::error::Error for StylesheetLoadError {}

pub struct StylesheetLoadOptions {
    pub consumer: String,
    pub index: usize,
    pub scope: CssManagedScope,
    pub declaration_identity: String,
    pub base_url: String,
    pub capability: CemModuleUrlResolutionCapability,
    pub response_policy: CssImportResponsePolicy,
}

pub(super) struct StylesheetLoad {
    options: StylesheetLoadOptions,
    closure: CssImportClosure,
    abort: AbortSignal,
}

pub enum StylesheetLoadProgress {
    Pending(CssImportRequest),
    Ready(Arc<RetainedStylesheet>),
}

impl RetainedTemplate {
    pub fn begin_stylesheet_load(
        &mut self,
        options: StylesheetLoadOptions,
    ) -> Result<u32, StylesheetLoadError> {
        if options.consumer.is_empty() {
            return Err("stylesheet consumer identity must not be empty".into());
        }
        self.validate_stylesheet_scope(
            options.index,
            &options.scope,
            &options.declaration_identity,
        )?;
        let source = self
            .artifact
            .stylesheets
            .get(options.index)
            .ok_or("unknown stylesheet occurrence")?;
        let replaced: Vec<_> = self
            .loads
            .iter()
            .filter(|(_, load)| {
                load.options.consumer == options.consumer && load.options.index == options.index
            })
            .map(|(id, _)| *id)
            .collect();
        if self.loads.len() - replaced.len() >= 64 {
            return Err("too many pending stylesheet loads".into());
        }
        let id = self
            .next_load
            .checked_add(1)
            .ok_or("stylesheet load identity exhausted")?;
        let abort = AbortSignal::new();
        let closure = CssImportClosure::new(
            source.css_tree().clone(),
            &options.base_url,
            options.capability.clone(),
            CssImportLimits::default(),
            abort.clone(),
        )?;
        for id in replaced {
            self.cancel_stylesheet_load(id);
        }
        self.next_load = id;
        self.stylesheet_generations
            .insert((options.consumer.clone(), options.index), id);
        self.loads.insert(
            id,
            StylesheetLoad {
                options,
                closure,
                abort,
            },
        );
        Ok(id)
    }

    pub fn advance_stylesheet_load(
        &mut self,
        id: u32,
        consumer: &str,
    ) -> Result<StylesheetLoadProgress, StylesheetLoadError> {
        self.check_load_consumer(id, consumer)?;
        let result = self.loads.get_mut(&id).unwrap().closure.next_import();
        match result {
            Ok(Some(request)) => Ok(StylesheetLoadProgress::Pending(request)),
            Ok(None) => {
                let load = self.loads.remove(&id).unwrap();
                let result = self.retain_stylesheet(
                    &load.options.consumer,
                    load.options.index,
                    Arc::new(load.closure),
                    &load.options.scope,
                    &load.options.declaration_identity,
                );
                if result.is_err() {
                    load.abort.abort();
                }
                result
                    .map(StylesheetLoadProgress::Ready)
                    .map_err(Into::into)
            }
            Err(error) => {
                self.cancel_stylesheet_load(id);
                Err(error.into())
            }
        }
    }

    pub fn deliver_stylesheet_response(
        &mut self,
        id: u32,
        consumer: &str,
        request_id: u64,
        response: ResolvedRead,
    ) -> Result<StylesheetLoadProgress, StylesheetLoadError> {
        self.check_load_consumer(id, consumer)?;
        let load = self.loads.get_mut(&id).unwrap();
        if let Err(error) =
            load.closure
                .complete_response(request_id, response, &load.options.response_policy)
        {
            self.cancel_stylesheet_load(id);
            return Err(error.into());
        }
        self.advance_stylesheet_load(id, consumer)
    }

    /// Stop a pending import after its host transport fails, retaining native location.
    pub fn fail_stylesheet_load(
        &mut self,
        id: u32,
        consumer: &str,
        request_id: u64,
        message: &str,
    ) -> Result<StylesheetLoadProgress, StylesheetLoadError> {
        self.check_load_consumer(id, consumer)?;
        if message.len() > cem_ml::import::MAX_DOCUMENT_BYTES {
            return Err("stylesheet failure control exceeds byte limit".into());
        }
        let load = self.loads.get_mut(&id).unwrap();
        if let Err(error) = load
            .closure
            .fail_import(request_id, "cem.css.import_load_failed", message)
        {
            self.cancel_stylesheet_load(id);
            return Err(error.into());
        }
        self.advance_stylesheet_load(id, consumer)
    }

    /// Cancellation targets a generation so a late reply cannot release newer work.
    pub fn release_stylesheet_generation(&mut self, consumer: &str, id: u32) -> usize {
        let slot = self
            .stylesheet_generations
            .iter()
            .find(|((owner, _), current)| owner == consumer && **current == id)
            .map(|(slot, _)| slot.clone());
        let Some(slot) = slot else {
            return 0;
        };
        self.stylesheet_generations.remove(&slot);
        self.cancel_stylesheet_load(id);
        self.consumers.remove(&slot);
        self.prune();
        1
    }

    fn check_load_consumer(&self, id: u32, consumer: &str) -> Result<(), String> {
        let load = self
            .loads
            .get(&id)
            .ok_or("stylesheet load is stale or unknown")?;
        if load.options.consumer != consumer {
            return Err("stylesheet load belongs to another consumer".into());
        }
        Ok(())
    }
    fn cancel_stylesheet_load(&mut self, id: u32) {
        if let Some(load) = self.loads.remove(&id) {
            load.abort.abort();
        }
    }
    pub(super) fn release_stylesheet_loads(&mut self, consumer: &str) -> usize {
        let ids: Vec<_> = self
            .loads
            .iter()
            .filter(|(_, load)| load.options.consumer == consumer)
            .map(|(id, _)| *id)
            .collect();
        let count = ids.len();
        for id in ids {
            self.cancel_stylesheet_load(id);
        }
        count
    }
    pub(super) fn cancel_all_stylesheet_loads(&self) {
        for load in self.loads.values() {
            load.abort.abort();
        }
    }
}
