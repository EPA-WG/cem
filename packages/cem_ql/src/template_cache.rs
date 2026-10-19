//! Checked compilation reuse. Only serialized immutable IR enters the shared
//! bounded store; each hit reloads an independent per-consumer template owner.
//! Resolver freshness is supplied anew on each call. This cache never skips
//! render evaluation, provider/event work, or stylesheet installation.
use crate::{
    render::{
        compilation_identity, compile_template, compile_template_module_closure,
        CompileTemplateOptions, TemplateArtifact, TemplateModuleClosure,
    },
    template_artifact::{
        encode_template_artifact, CompiledTemplateArtifact, TemplateArtifactLoadContext,
        TemplateArtifactSourceMapMode,
    },
};
use cem_ml::{
    content_cache::{
        ArtifactContentType, CacheArtifact, CacheError, CacheMode, ContentCache,
        ContentCacheLimits, ContentCacheRuntime, ContentCacheStats, ContentHash, PolicyStamps,
    },
    dependency_identity::DependencyAdmission,
};

pub struct TemplateCompilationCache {
    store: ContentCache,
    compilations: u64,
}
/// Capture before work; a portable dependency key does not grant publication.
pub struct TemplateCompilationRequest {
    runtime: ContentCacheRuntime,
    admission: DependencyAdmission,
}
pub struct CheckedTemplateCompilation {
    pub artifact: TemplateArtifact,
    pub hit: bool,
}
impl Default for TemplateCompilationCache {
    fn default() -> Self {
        Self {
            store: ContentCache::default(),
            compilations: 0,
        }
    }
}
impl TemplateCompilationCache {
    pub fn new(limits: ContentCacheLimits) -> Result<Self, CacheError> {
        Ok(Self {
            store: ContentCache::with_limits(limits)?,
            compilations: 0,
        })
    }
    pub fn begin(&self, admission: DependencyAdmission) -> TemplateCompilationRequest {
        TemplateCompilationRequest {
            runtime: self.store.runtime(),
            admission,
        }
    }
    pub fn reset(&mut self) -> Result<(), CacheError> {
        self.store.reset_runtime()
    }
    pub fn stats(&self) -> ContentCacheStats {
        self.store.stats()
    }
    /// Actual source compiler invocations, excluding binary reloads.
    pub fn compilations(&self) -> u64 {
        self.compilations
    }

    pub fn compile(
        &mut self,
        source: &str,
        closure: Option<&TemplateModuleClosure>,
        options: &CompileTemplateOptions,
        mode: TemplateArtifactSourceMapMode,
        request: &TemplateCompilationRequest,
        current: Option<&DependencyAdmission>,
    ) -> Result<CheckedTemplateCompilation, CacheError> {
        if !current.is_some_and(|c| request.admission.is_current(c)) {
            return Err(CacheError {
                code: "cem.cc.compilation_superseded",
                message: "compilation owner was cancelled, superseded or released".into(),
            });
        }
        // Even ineligible requests check their runtime before invoking a compiler.
        let identity = checked_type(source, closure, options);
        let policy = PolicyStamps::new("cem-template-compilation/checked/1");
        let cache_mode = match mode {
            TemplateArtifactSourceMapMode::Dev => CacheMode::Dev,
            TemplateArtifactSourceMapMode::Prod => CacheMode::Prod,
        };
        let lookup_type = identity
            .clone()
            .unwrap_or_else(|| ArtifactContentType::Other("cem-template/ineligible".into()));
        if let Some(lease) = self.store.lease_by_content_type_for_runtime(
            &request.runtime,
            &lookup_type,
            cache_mode,
            &policy,
        )? {
            let bytes = lease.artifact(&policy)?.bytes.clone();
            // Binary validation stays on the ordinary native reload boundary.
            if let Ok(artifact) = CompiledTemplateArtifact::from_bytes(bytes).and_then(|binary| {
                binary.reload(&TemplateArtifactLoadContext {
                    expected_source_hash: Some(ContentHash::from_blake3(source.as_bytes())),
                    host_bindings: options.host_bindings.clone(),
                    source_map_mode: mode,
                })
            }) {
                return Ok(CheckedTemplateCompilation {
                    artifact,
                    hit: true,
                });
            }
        }
        self.compilations = self.compilations.saturating_add(1);
        let mut artifact = match closure {
            Some(closure) => compile_template_module_closure(source, closure, options),
            None => compile_template(source, options),
        };
        // Preserve normal errors; invalid or opaque closures never populate cache.
        let valid = !artifact
            .diagnostics
            .iter()
            .any(|d| d.severity.is_hard_violation());
        if let Some(content_type) = identity.filter(|_| valid) {
            let binary = encode_template_artifact(source, options, mode, &mut artifact);
            let (cached, sidecar) =
                CacheArtifact::new(content_type, cache_mode, binary.bytes, policy, None);
            // Capacity refusal is an optimization miss, not a compilation error.
            // Runtime admission errors remain fatal rather than publishing stale work.
            if let Err(error) = self
                .store
                .insert_for_runtime(&request.runtime, cached, sidecar)
            {
                if error.code == "cem.cc.runtime_replaced" {
                    return Err(error);
                }
            }
        }
        if mode == TemplateArtifactSourceMapMode::Prod {
            crate::template_artifact::strip_template_source_maps(&mut artifact);
        }
        Ok(CheckedTemplateCompilation {
            artifact,
            hit: false,
        })
    }
}

fn checked_type(
    source: &str,
    closure: Option<&TemplateModuleClosure>,
    options: &CompileTemplateOptions,
) -> Option<ArtifactContentType> {
    let standalone;
    let (kind, closure) = match closure {
        Some(closure) => ("closure", closure),
        None => {
            standalone = TemplateModuleClosure {
                root_uri: options
                    .source_uri
                    .clone()
                    .unwrap_or_else(|| "cem://template/anonymous".into()),
                root_content_hash: ContentHash::from_blake3(source.as_bytes()).header_value(),
                resolver_policy_stamp: "cem-template/source-only/1".into(),
                ..Default::default()
            };
            ("source", &standalone)
        }
    };
    // Check bounded metadata before walking/hashing source payloads. The old
    // post-compilation receipt alone cannot authorize a pre-compilation hit.
    let identity = compilation_identity(closure, options).ok()?;
    if closure.root_content_hash != ContentHash::from_blake3(source.as_bytes()).header_value()
        || closure
            .modules
            .iter()
            .any(|m| m.content_hash != ContentHash::from_blake3(m.source.as_bytes()).header_value())
        || (!closure.cem_ml_version.is_empty() && closure.cem_ml_version != cem_ml::VERSION)
        || (!closure.cem_ql_version.is_empty() && closure.cem_ql_version != crate::VERSION)
        || (!closure.entrypoint.is_empty() && closure.entrypoint != "body")
    {
        return None;
    }
    let mut bindings = options.host_bindings.clone();
    bindings.sort();
    bindings.dedup();
    if !closure.parameter_contract.is_empty() && closure.parameter_contract != bindings {
        return None;
    }
    // Different compiler entrypoints and absent/explicit source URI remain
    // distinct even when their effective closure root text happens to match.
    let qualified = serde_json::to_vec(&(
        kind,
        identity.hash,
        if kind == "source" {
            options.source_uri.as_deref()
        } else {
            None
        },
    ))
    .ok()?;
    Some(ArtifactContentType::Other(format!(
        "cem-template/checked/{}",
        ContentHash::from_blake3(&qualified).header_value()
    )))
}
