//! Native-owned freshness for the capability-free retained JSON boundary.
//! Hash actual parsed input bytes, never a JS digest/revision claim. Explicit
//! native/resource handles remain opaque, even if their JSON envelopes repeat.
use super::*;
use crate::eval::{DataReaderCache, Item};
use cem_ml::{
    content_cache::ContentHash,
    dependency_identity::{DependencyVersion, RENDER_DEPENDENCY_INPUTS},
    scheduler::ScopePolicy,
};

#[derive(Debug, Clone, serde::Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RetainedJsonRenderContext {
    pub owner: String,
    pub scope_policy_stamp: String,
}
#[derive(Default)]
pub struct RetainedJsonRenderCache {
    cache: PureRenderCache,
    revision: u64,
    admissions: u64,
}
impl RetainedJsonRenderCache {
    pub fn evaluations(&self) -> u64 {
        self.cache.evaluations()
    }
    pub fn admissions(&self) -> u64 {
        self.admissions
    }
    pub fn stats(&self) -> ContentCacheStats {
        self.cache.stats()
    }
    pub fn reset(&mut self) -> Result<(), CacheError> {
        self.cache.reset()
    }
    /// The WASM caller must exclude all non-JSON capabilities. This method owns
    /// parsing, input fingerprinting, request identity and synchronous admission;
    /// projection/resource controls/publication still execute outside this cache.
    pub fn render(
        &mut self,
        artifact: &TemplateArtifact,
        json: &str,
        context: &RetainedJsonRenderContext,
        readers: &DataReaderCache,
    ) -> Result<CheckedPureRender, CacheError> {
        if [&context.owner, &context.scope_policy_stamp]
            .iter()
            .any(|s| s.is_empty() || s.len() > 8192)
        {
            return Err(CacheError {
                code: "cem.render.context",
                message: "Invalid retained render owner/scope".into(),
            });
        }
        let mut data = crate::api::parse_template_data(json).map_err(|message| CacheError {
            code: "cem.ql.wasm.invalid_data",
            message,
        })?;
        data.data_readers = readers.clone();
        let mut owned = json.len() <= crate::eval::dependencies::MAX_EVALUATION_RECEIPT_BYTES
            && data.bindings.len() <= 65536;
        let mut pending = Vec::new();
        if owned {
            for stream in data.bindings.values() {
                if pending.len() + stream.items.len() > 65536 {
                    owned = false;
                    break;
                }
                pending.extend(&stream.items);
            }
        }
        let mut count = 0usize;
        while owned {
            let Some(item) = pending.pop() else {
                break;
            };
            count += 1;
            if count > 65536 {
                owned = false;
                break;
            }
            match item {
                Item::Atomic(_) => (),
                Item::Array(items) => {
                    if pending.len() + items.len() > 65536 {
                        owned = false;
                    } else {
                        pending.extend(items);
                    }
                }
                Item::Record(fields) => {
                    for values in fields.values() {
                        if pending.len() + values.len() > 65536 {
                            owned = false;
                            break;
                        }
                        pending.extend(values);
                    }
                }
                _ => owned = false,
            }
        }
        let freshness = if owned {
            let hash = DependencyVersion::Content(ContentHash::from_blake3(json.as_bytes()));
            RenderFreshness {
                environment: DependencyVersion::Content(
                    crate::eval::dependencies::bounded_identity(
                        b"cem.ql.retained-json/1\n",
                        &(crate::VERSION, cem_ml::VERSION, &context.scope_policy_stamp),
                    )
                    .map_err(|_| CacheError {
                        code: "cem.render.context",
                        message: "Retained context exceeds bounds".into(),
                    })?,
                ),
                inputs: RENDER_DEPENDENCY_INPUTS
                    .iter()
                    .map(|axis| (*axis, hash.clone()))
                    .collect(),
            }
        } else {
            RenderFreshness::default()
        };
        self.revision = self.revision.checked_add(1).ok_or_else(|| CacheError {
            code: "cem.render.exhausted",
            message: "Retained render admission exhausted".into(),
        })?;
        let state = RenderAdmission {
            owner: DependencyAdmission {
                owner: context.owner.clone(),
                runtime_generation: self.cache.stats().runtime_generation,
                revision: self.revision,
            },
            freshness,
        };
        let ticket = self.cache.begin(state.owner.clone());
        let control = OperationControl::with_root_policy(
            Default::default(),
            ScopePolicy::host_root().with_queue_size(128),
        )
        .map_err(|error| CacheError {
            code: "cem.render.control",
            message: error.to_string(),
        })?;
        let admissions = &mut self.admissions;
        self.cache.render(
            artifact,
            &data,
            &ticket,
            &control,
            cem_ml::operation_control::ROOT_EXECUTION_SCOPE_ID,
            || {
                *admissions = admissions.saturating_add(1);
                Ok(state.clone())
            },
            || Some(state.clone()),
        )
    }
}
