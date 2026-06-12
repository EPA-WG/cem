//! Content-addressed transport loader for compiled CEM-QL artifacts.
//!
//! Implements the cem-ql binding of AC-CC-6 / AC-CC-7 (`cem-ml-ac.md` §14):
//! when an engine holds a cached compiled artifact it sends `If-CEM-Hash`;
//! a `304 Not Modified` confirms the cache and the loader skips the parser
//! by reloading the binary. A `200` body is compiled through
//! [`crate::api::compile_artifact`] and the recomputed hash MUST match the
//! server's `CEM-Hash` header. Verified by AC-QC-V-2 in
//! `tests/transport_protocol.rs`.

use std::collections::BTreeMap;
use std::sync::atomic::{AtomicUsize, Ordering};

use cem_ml::content_cache::{
    ArtifactContentType, CemHashRequest, CemHashResponse, ContentHash, InMemoryCemHashTransport,
};

use crate::api::{compile, compile_artifact, reload_artifact, CompileContext, CompileError};
use crate::artifact::CompiledArtifact;
use crate::ir::CompiledQuery;

/// Header-shaped request the engine sends to the transport. `if_cem_hash`
/// is populated when the engine already holds a cached compiled artifact
/// for this URI and would prefer to satisfy the load from cache.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LoaderRequest {
    pub uri: String,
    pub if_cem_hash: Option<ContentHash>,
}

/// Header-shaped response. `NotModified` carries the matching `CEM-Hash`
/// only; `Body` carries the source bytes plus the `CEM-Hash` the engine
/// MUST recompute and compare against per AC-CC-6.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LoaderResponse {
    NotModified {
        cem_hash: ContentHash,
    },
    Body {
        cem_hash: ContentHash,
        body: Vec<u8>,
    },
}

/// Transport boundary. The future `cem-ml-cli` HTTP(S) loader and any
/// build-pipeline / file-store loader implement this trait; tests use
/// [`InMemoryTransport`].
pub trait Transport {
    fn fetch(&self, request: &LoaderRequest) -> Result<LoaderResponse, TransportError>;
}

/// Outcome of a single load. The verification fixture asserts these to
/// prove the parser ran (or didn't) on each pass.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LoadOutcome {
    /// `200` response: loader entered the cem-ql parser via
    /// [`compile_artifact`] and populated its cache.
    Compiled,
    /// `304` response: loader resolved the request from its cache via
    /// [`reload_artifact`]; the parser was not entered.
    CacheHit,
}

/// Process-wide counters the verification fixture inspects. Each
/// counter is monotonic across the lifetime of an [`ArtifactLoader`].
#[derive(Debug, Default)]
pub struct LoaderTelemetry {
    compiled: AtomicUsize,
    cache_hits: AtomicUsize,
    conditional_requests: AtomicUsize,
}

impl LoaderTelemetry {
    pub fn compiled(&self) -> usize {
        self.compiled.load(Ordering::SeqCst)
    }
    pub fn cache_hits(&self) -> usize {
        self.cache_hits.load(Ordering::SeqCst)
    }
    pub fn conditional_requests(&self) -> usize {
        self.conditional_requests.load(Ordering::SeqCst)
    }
}

/// Engine-side artifact loader. Holds a `uri → CompiledArtifact` cache
/// keyed by source URI; the artifact carries the content hash the next
/// load sends in `If-CEM-Hash`.
pub struct ArtifactLoader<T: Transport> {
    transport: T,
    cache: BTreeMap<String, CompiledArtifact>,
    telemetry: LoaderTelemetry,
}

impl<T: Transport> ArtifactLoader<T> {
    pub fn new(transport: T) -> Self {
        Self {
            transport,
            cache: BTreeMap::new(),
            telemetry: LoaderTelemetry::default(),
        }
    }

    pub fn telemetry(&self) -> &LoaderTelemetry {
        &self.telemetry
    }

    pub fn transport(&self) -> &T {
        &self.transport
    }

    pub fn cached_hash(&self, uri: &str) -> Option<&ContentHash> {
        self.cache.get(uri).map(|artifact| &artifact.content_hash)
    }

    pub fn load(
        &mut self,
        uri: &str,
        context: &CompileContext,
    ) -> Result<(CompiledQuery, LoadOutcome), LoaderError> {
        let cached_hash = self.cached_hash(uri).cloned();
        let request = LoaderRequest {
            uri: uri.to_owned(),
            if_cem_hash: cached_hash.clone(),
        };
        if request.if_cem_hash.is_some() {
            self.telemetry
                .conditional_requests
                .fetch_add(1, Ordering::SeqCst);
        }

        let response = self
            .transport
            .fetch(&request)
            .map_err(LoaderError::transport)?;

        match response {
            LoaderResponse::NotModified { cem_hash } => {
                let cached = cached_hash.ok_or_else(|| {
                    LoaderError::protocol("server returned 304 without a prior cached hash")
                })?;
                if cached != cem_hash {
                    return Err(LoaderError::hash_mismatch(
                        "304 CEM-Hash does not match cached artifact hash",
                    ));
                }
                let artifact = self.cache.get(uri).expect("cache populated").clone();
                let query = reload_artifact(&artifact).map_err(LoaderError::reload)?;
                self.telemetry.cache_hits.fetch_add(1, Ordering::SeqCst);
                Ok((query, LoadOutcome::CacheHit))
            }
            LoaderResponse::Body { cem_hash, body } => {
                let source = std::str::from_utf8(&body)
                    .map_err(|_| LoaderError::protocol("response body is not valid UTF-8"))?;
                let query = compile(source, context).map_err(LoaderError::compile)?;
                let artifact = CompiledArtifact::from_query(&query);
                if artifact.content_hash != cem_hash {
                    return Err(LoaderError::hash_mismatch(
                        "recomputed artifact hash does not match server CEM-Hash",
                    ));
                }
                self.cache.insert(uri.to_owned(), artifact);
                self.telemetry.compiled.fetch_add(1, Ordering::SeqCst);
                Ok((query, LoadOutcome::Compiled))
            }
        }
    }
}

/// Wraps [`compile_artifact`] for callers that hold a source string and
/// want to publish it to a transport-backed store under its content
/// hash. Used by [`InMemoryTransport::publish`].
pub fn artifact_from_source(
    source: &str,
    context: &CompileContext,
) -> Result<CompiledArtifact, CompileError> {
    compile_artifact(source, context)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TransportError {
    pub code: &'static str,
    pub message: String,
}

impl TransportError {
    pub fn not_found(message: impl Into<String>) -> Self {
        Self {
            code: "cem.cc.not_found",
            message: message.into(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LoaderError {
    pub code: &'static str,
    pub message: String,
}

impl LoaderError {
    fn transport(err: TransportError) -> Self {
        Self {
            code: err.code,
            message: err.message,
        }
    }

    fn protocol(message: impl Into<String>) -> Self {
        Self {
            code: "cem.cc.protocol",
            message: message.into(),
        }
    }

    fn hash_mismatch(message: impl Into<String>) -> Self {
        Self {
            code: "cem.cc.hash_mismatch",
            message: message.into(),
        }
    }

    fn compile(err: CompileError) -> Self {
        Self {
            code: err.code,
            message: err.message,
        }
    }

    fn reload(err: crate::api::LoadError) -> Self {
        Self {
            code: err.code,
            message: err.message,
        }
    }
}

/// Mock transport backed by an in-process `uri → (CEM-Hash, body)`
/// table. Mirrors the server side of AC-CC-6: responds `304` when the
/// `If-CEM-Hash` matches the stored hash and `200` with the body
/// otherwise. Tracks the request log so the fixture can assert which
/// header the loader sent.
#[derive(Debug, Default)]
pub struct InMemoryTransport {
    shared: InMemoryCemHashTransport,
    request_log: std::sync::Mutex<Vec<LoaderRequest>>,
}

impl InMemoryTransport {
    pub fn new() -> Self {
        Self::default()
    }

    /// Publish a source body under `uri`. The transport stamps it with
    /// the same `cem-bin/1+blake3` hash a compiled artifact would carry
    /// when its source text round-trips through `compile_artifact`. The
    /// build pipeline would compute this hash up front.
    pub fn publish(&mut self, uri: &str, body: Vec<u8>, cem_hash: ContentHash) {
        self.shared.publish_with_hash(
            uri.to_owned(),
            ArtifactContentType::CemQlModule,
            body,
            cem_hash,
        );
    }

    pub fn requests(&self) -> Vec<LoaderRequest> {
        self.request_log.lock().expect("request log").clone()
    }
}

impl Transport for InMemoryTransport {
    fn fetch(&self, request: &LoaderRequest) -> Result<LoaderResponse, TransportError> {
        self.request_log
            .lock()
            .expect("request log")
            .push(request.clone());
        match self.shared.fetch(&CemHashRequest {
            uri: request.uri.clone(),
            if_cem_hash: request.if_cem_hash.clone(),
        }) {
            Ok(CemHashResponse::NotModified { cem_hash, .. }) => {
                Ok(LoaderResponse::NotModified { cem_hash })
            }
            Ok(CemHashResponse::Body { cem_hash, body, .. }) => {
                Ok(LoaderResponse::Body { cem_hash, body })
            }
            Err(err) => Err(TransportError {
                code: err.code,
                message: err.message,
            }),
        }
    }
}
