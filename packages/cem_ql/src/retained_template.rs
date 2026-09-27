//! Native template owner shared by processing-host handles and native tests.
//! Derived CSS retains its source closure; no CSS tree crosses a serialized boundary.
use crate::{eval::DataReaderCache, render::TemplateArtifact};
use cem_ml::{
    css_emission::{
        derive_css_stylesheet_identity, emit_css_import_closure, CssClosureEmission,
        CssManagedScope, CssStylesheetIdentity,
    },
    css_imports::{CssImportClosure, CssImportState},
    scheduler::AbortSignal,
};
use std::{
    collections::{BTreeMap, BTreeSet},
    sync::Arc,
};

pub struct RetainedStylesheet {
    identity: CssStylesheetIdentity,
    closure: Arc<CssImportClosure>,
    emission: CssClosureEmission,
    released: AbortSignal,
}
impl RetainedStylesheet {
    pub fn identity(&self) -> &CssStylesheetIdentity {
        &self.identity
    }
    fn active(&self) -> bool {
        !self.released.is_aborted()
            && self.closure.state() == CssImportState::Ready
            && self.closure.failure().is_none()
    }
    /// Check liveness immediately before using output, including on cache hits.
    /// The browser host still owns its final context/marker commit check.
    pub fn emission(&self) -> Result<&CssClosureEmission, &'static str> {
        self.active()
            .then_some(&self.emission)
            .ok_or("derived stylesheet closure is no longer active")
    }
}

pub struct RetainedTemplate {
    artifact: TemplateArtifact,
    data_readers: DataReaderCache,
    // A consumer can own multiple style occurrences without replacing siblings.
    consumers: BTreeMap<(String, usize), String>,
    stylesheets: BTreeMap<String, Arc<RetainedStylesheet>>,
}
impl RetainedTemplate {
    pub fn new(artifact: TemplateArtifact) -> Self {
        Self {
            artifact,
            data_readers: Default::default(),
            consumers: BTreeMap::new(),
            stylesheets: BTreeMap::new(),
        }
    }
    pub fn data_readers(&self) -> &DataReaderCache {
        &self.data_readers
    }

    pub fn artifact(&self) -> &TemplateArtifact {
        &self.artifact
    }

    /// Retain one fully loaded stylesheet from this template's native source tree.
    /// `consumer` identifies a host/context lease, not a CSS selector or cache key.
    /// The caller admits the effective declaration scope and revalidates imports
    /// before providing a ready closure. Pending loading is not a cache shortcut.
    pub fn retain_stylesheet(
        &mut self,
        consumer: &str,
        index: usize,
        closure: Arc<CssImportClosure>,
        scope: &CssManagedScope,
        declaration_identity: &str,
    ) -> Result<Arc<RetainedStylesheet>, String> {
        if consumer.is_empty() {
            return Err("stylesheet consumer identity must not be empty".into());
        }
        let source = self
            .artifact
            .stylesheets
            .get(index)
            .ok_or("unknown template stylesheet occurrence")?;
        let root = closure
            .sheets()
            .first()
            .ok_or("stylesheet closure has no root")?;
        if !Arc::ptr_eq(source.css_tree(), &root.resources.tree) {
            return Err(
                "stylesheet closure does not belong to the retained template source".into(),
            );
        }
        if matches!(scope, CssManagedScope::Instance) {
            return Err(
                "declaration stylesheet artifacts require a private or shared scope".into(),
            );
        }
        if let Some(authored) = &source.scope {
            if !matches!(scope, CssManagedScope::Shared { name, .. } if name == authored) {
                return Err("effective stylesheet scope disagrees with its authored scope".into());
            }
        }
        let identity = derive_css_stylesheet_identity(
            &closure,
            scope,
            declaration_identity,
            &format!("style:{index}"),
        )?;
        let key = identity.cache_key.clone();
        let retained = match self.stylesheets.get(&key).filter(|entry| entry.active()) {
            Some(entry) => entry.clone(),
            None => {
                let emission =
                    emit_css_import_closure(&closure, &identity.scope, &identity.owner_key)
                        .map_err(|error| {
                            format!("{}: {}", error.diagnostic.code, error.diagnostic.message)
                        })?;
                Arc::new(RetainedStylesheet {
                    identity,
                    closure: closure.clone(),
                    emission,
                    released: AbortSignal::new(),
                })
            }
        };
        // Recheck after emission and before changing the consumer's current set.
        retained.emission()?;
        if closure.state() != CssImportState::Ready || closure.failure().is_some() {
            return Err("incoming stylesheet closure is no longer active".into());
        }
        self.stylesheets.insert(key.clone(), retained.clone());
        self.consumers.insert((consumer.to_owned(), index), key);
        self.prune();
        Ok(retained)
    }

    pub fn stylesheet(&self, consumer: &str, index: usize) -> Option<Arc<RetainedStylesheet>> {
        let key = self.consumers.get(&(consumer.to_owned(), index))?;
        self.stylesheets
            .get(key)
            .filter(|entry| entry.active())
            .cloned()
    }

    /// Release every occurrence owned by this consumer. Removing the last
    /// consumer invalidates outstanding output handles without cancelling sources.
    pub fn release_stylesheet_consumer(&mut self, consumer: &str) -> usize {
        let before = self.consumers.len();
        self.consumers.retain(|(owner, _), _| owner != consumer);
        self.prune();
        before - self.consumers.len()
    }
    fn prune(&mut self) {
        let live: BTreeSet<_> = self.consumers.values().collect();
        self.stylesheets.retain(|key, entry| {
            if live.contains(key) {
                true
            } else {
                entry.released.abort();
                false
            }
        });
    }
}

impl Drop for RetainedTemplate {
    fn drop(&mut self) {
        for entry in self.stylesheets.values() {
            entry.released.abort();
        }
    }
}
