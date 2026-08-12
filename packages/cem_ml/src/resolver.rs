//! Shared URI/resource resolver boundary.
//!
//! The first built-in resolver behavior is intentionally local-only: plain
//! filesystem paths and local `file://` URIs map to paths, while remote or
//! custom schemes require a host-registered resolver.

use std::borrow::Cow;
use std::collections::BTreeMap;
use std::fmt;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use crate::scheduler::AbortSignal;
use crate::source_map::SourceMapStack;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ResolvePurpose {
    Config,
    Input,
    Query,
    Template,
    ModuleMap,
    Output,
    Report,
    ObserveEvents,
}

impl ResolvePurpose {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Config => "config",
            Self::Input => "input",
            Self::Query => "query",
            Self::Template => "template",
            Self::ModuleMap => "moduleMap",
            Self::Output => "output",
            Self::Report => "report",
            Self::ObserveEvents => "observeEvents",
        }
    }
}

impl fmt::Display for ResolvePurpose {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ResolveDirection {
    Read,
    Write,
    List,
}

impl ResolveDirection {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Read => "read",
            Self::Write => "write",
            Self::List => "list",
        }
    }
}

impl fmt::Display for ResolveDirection {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolveRequest {
    pub uri: String,
    pub base_uri: Option<String>,
    pub purpose: ResolvePurpose,
    pub direction: ResolveDirection,
    pub content_type_hint: Option<String>,
}

impl ResolveRequest {
    pub fn new(
        uri: impl Into<String>,
        purpose: ResolvePurpose,
        direction: ResolveDirection,
    ) -> Self {
        Self {
            uri: uri.into(),
            base_uri: None,
            purpose,
            direction,
            content_type_hint: None,
        }
    }

    pub fn with_base_uri(mut self, base_uri: impl Into<String>) -> Self {
        self.base_uri = Some(base_uri.into());
        self
    }

    pub fn with_content_type_hint(mut self, content_type_hint: impl Into<String>) -> Self {
        self.content_type_hint = Some(content_type_hint.into());
        self
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedRead {
    pub uri: String,
    pub bytes: Vec<u8>,
    pub content_type: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvePolicyDecision {
    pub requested_uri: String,
    pub normalized_uri: String,
    pub effective_uri: String,
    pub substituted_uri: Option<String>,
    pub reason: Option<String>,
    pub policy_stamp: String,
}

impl ResolvePolicyDecision {
    fn from_request(request: &ResolveRequest, policy_stamp: String) -> Self {
        let normalized_uri = normalize_policy_uri(&request.uri);
        Self {
            requested_uri: request.uri.clone(),
            effective_uri: normalized_uri.clone(),
            normalized_uri,
            substituted_uri: None,
            reason: None,
            policy_stamp,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvePolicyDiagnostic {
    pub request: ResolveRequest,
    pub reason: String,
    pub policy_stamp: String,
}

impl fmt::Display for ResolvePolicyDiagnostic {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "resolver policy denied {} {} URI `{}`: {}",
            self.request.direction, self.request.purpose, self.request.uri, self.reason
        )
    }
}

impl std::error::Error for ResolvePolicyDiagnostic {}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct ResolvePolicyRequestKey {
    pub uri: String,
    pub base_uri: Option<String>,
    pub purpose: ResolvePurpose,
    pub direction: ResolveDirection,
    pub content_type_hint: Option<String>,
}

impl ResolvePolicyRequestKey {
    pub fn new(
        uri: impl Into<String>,
        purpose: ResolvePurpose,
        direction: ResolveDirection,
    ) -> Self {
        let uri = uri.into();
        Self {
            uri: normalize_policy_uri(&uri),
            base_uri: None,
            purpose,
            direction,
            content_type_hint: None,
        }
    }

    pub fn with_base_uri(mut self, base_uri: impl Into<String>) -> Self {
        self.base_uri = Some(normalize_policy_uri(&base_uri.into()));
        self
    }

    pub fn with_content_type_hint(mut self, content_type_hint: impl Into<String>) -> Self {
        self.content_type_hint = Some(content_type_hint.into());
        self
    }

    fn from_request(request: &ResolveRequest) -> Self {
        let mut key = Self::new(request.uri.as_str(), request.purpose, request.direction);
        if let Some(base_uri) = &request.base_uri {
            key = key.with_base_uri(base_uri.as_str());
        }
        if let Some(content_type_hint) = &request.content_type_hint {
            key = key.with_content_type_hint(content_type_hint.as_str());
        }
        key
    }

    fn candidates_from_request(request: &ResolveRequest) -> Vec<Self> {
        let exact = Self::from_request(request);
        let mut candidates = vec![exact.clone()];
        if exact.content_type_hint.is_some() {
            let mut without_content_type = exact.clone();
            without_content_type.content_type_hint = None;
            candidates.push(without_content_type);
        }
        if exact.base_uri.is_some() {
            let mut without_base = exact.clone();
            without_base.base_uri = None;
            candidates.push(without_base.clone());
            if without_base.content_type_hint.is_some() {
                without_base.content_type_hint = None;
                candidates.push(without_base);
            }
        }
        candidates
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvePolicySubstitution {
    pub substituted_uri: String,
    pub reason: String,
}

impl ResolvePolicySubstitution {
    pub fn new(substituted_uri: impl Into<String>, reason: impl Into<String>) -> Self {
        Self {
            substituted_uri: normalize_policy_uri(&substituted_uri.into()),
            reason: reason.into(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvePolicyDenial {
    pub reason: String,
}

impl ResolvePolicyDenial {
    pub fn new(reason: impl Into<String>) -> Self {
        Self {
            reason: reason.into(),
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ResolverPolicy {
    substitutions: BTreeMap<ResolvePolicyRequestKey, ResolvePolicySubstitution>,
    denials: BTreeMap<ResolvePolicyRequestKey, ResolvePolicyDenial>,
}

impl ResolverPolicy {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn register_substitution(
        &mut self,
        key: ResolvePolicyRequestKey,
        substitution: ResolvePolicySubstitution,
    ) -> Option<ResolvePolicySubstitution> {
        self.substitutions.insert(key, substitution)
    }

    pub fn with_substitution(
        mut self,
        key: ResolvePolicyRequestKey,
        substitution: ResolvePolicySubstitution,
    ) -> Self {
        self.register_substitution(key, substitution);
        self
    }

    pub fn register_denial(
        &mut self,
        key: ResolvePolicyRequestKey,
        denial: ResolvePolicyDenial,
    ) -> Option<ResolvePolicyDenial> {
        self.denials.insert(key, denial)
    }

    pub fn with_denial(
        mut self,
        key: ResolvePolicyRequestKey,
        denial: ResolvePolicyDenial,
    ) -> Self {
        self.register_denial(key, denial);
        self
    }

    pub fn decide(
        &self,
        request: &ResolveRequest,
    ) -> Result<ResolvePolicyDecision, ResolvePolicyDiagnostic> {
        let keys = ResolvePolicyRequestKey::candidates_from_request(request);
        let policy_stamp = self.cache_stamp();
        for key in &keys {
            if let Some(denial) = self.denials.get(key) {
                return Err(ResolvePolicyDiagnostic {
                    request: request.clone(),
                    reason: denial.reason.clone(),
                    policy_stamp,
                });
            }
        }
        for key in &keys {
            if let Some(substitution) = self.substitutions.get(key) {
                let normalized_uri = normalize_policy_uri(&request.uri);
                return Ok(ResolvePolicyDecision {
                    requested_uri: request.uri.clone(),
                    normalized_uri,
                    effective_uri: substitution.substituted_uri.clone(),
                    substituted_uri: Some(substitution.substituted_uri.clone()),
                    reason: Some(substitution.reason.clone()),
                    policy_stamp,
                });
            }
        }

        Ok(ResolvePolicyDecision::from_request(request, policy_stamp))
    }

    pub fn cache_stamp(&self) -> String {
        let substitutions =
            stamped_policy_map(self.substitutions.iter().map(|(key, substitution)| {
                (
                    key,
                    substitution.substituted_uri.as_str(),
                    substitution.reason.as_str(),
                )
            }));
        let denials = stamped_policy_map(
            self.denials
                .iter()
                .map(|(key, denial)| (key, "", denial.reason.as_str())),
        );
        format!("resolver-policy/1;substitutions={substitutions};denials={denials}")
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolveListRequest {
    pub uri: String,
    pub base_uri: Option<String>,
    pub purpose: ResolvePurpose,
    pub content_type_hint: Option<String>,
    pub max_entries: Option<usize>,
}

impl ResolveListRequest {
    pub fn new(uri: impl Into<String>, purpose: ResolvePurpose) -> Self {
        Self {
            uri: uri.into(),
            base_uri: None,
            purpose,
            content_type_hint: None,
            max_entries: None,
        }
    }

    pub fn with_base_uri(mut self, base_uri: impl Into<String>) -> Self {
        self.base_uri = Some(base_uri.into());
        self
    }

    pub fn with_content_type_hint(mut self, content_type_hint: impl Into<String>) -> Self {
        self.content_type_hint = Some(content_type_hint.into());
        self
    }

    pub fn with_max_entries(mut self, max_entries: usize) -> Self {
        self.max_entries = Some(max_entries);
        self
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedListEntry {
    pub uri: String,
    pub content_type: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedWrite {
    pub uri: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PublicationCapability {
    DirectOnly,
    Transactional,
}

pub trait PreparedResolverWrite: Send {
    fn commit(&mut self) -> Result<ResolvedWrite, ResolverDiagnostic>;
    fn rollback(&mut self) -> Result<(), ResolverDiagnostic>;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ResolverDiagnostic {
    Cancelled {
        uri: String,
        purpose: ResolvePurpose,
        direction: ResolveDirection,
        source_map: Option<SourceMapStack>,
    },
    UnsupportedResolver {
        uri: String,
        purpose: ResolvePurpose,
        direction: ResolveDirection,
    },
    NonLocalFileUri {
        uri: String,
    },
    InvalidFileUri {
        uri: String,
        message: String,
    },
    Io {
        uri: String,
        message: String,
    },
    TransactionUnsupported {
        uri: String,
        purpose: ResolvePurpose,
    },
    TransactionState {
        uri: String,
        message: String,
    },
}

impl ResolverDiagnostic {
    pub fn code(&self) -> &'static str {
        match self {
            Self::Cancelled { .. } => "cem.resolver.cancelled",
            Self::UnsupportedResolver { .. } => "cem.resolver.unsupported",
            Self::NonLocalFileUri { .. } => "cem.resolver.file_uri_non_local",
            Self::InvalidFileUri { .. } => "cem.resolver.file_uri_invalid",
            Self::Io { .. } => "cem.resolver.io",
            Self::TransactionUnsupported { .. } => "cem.resolver.transaction_unsupported",
            Self::TransactionState { .. } => "cem.resolver.transaction_state",
        }
    }
}

impl fmt::Display for ResolverDiagnostic {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Cancelled {
                uri,
                purpose,
                direction,
                ..
            } => write!(
                f,
                "resolver {direction} {purpose} URI `{uri}` cancelled by host"
            ),
            Self::UnsupportedResolver {
                uri,
                purpose,
                direction,
            } => write!(
                f,
                "unsupported resolver for {direction} {purpose} URI `{uri}`"
            ),
            Self::NonLocalFileUri { uri } => {
                write!(
                    f,
                    "unsupported file URI `{uri}`; only local file:// URIs are supported"
                )
            }
            Self::InvalidFileUri { uri, message } => {
                write!(f, "invalid file URI `{uri}`: {message}")
            }
            Self::Io { uri, message } => write!(f, "I/O error for `{uri}`: {message}"),
            Self::TransactionUnsupported { uri, purpose } => write!(
                f,
                "resolver for {purpose} URI `{uri}` does not support transactional publication"
            ),
            Self::TransactionState { uri, message } => {
                write!(f, "transactional publication error for `{uri}`: {message}")
            }
        }
    }
}

impl std::error::Error for ResolverDiagnostic {}

pub trait ResourceResolver: Send + Sync {
    fn read(&self, request: &ResolveRequest) -> Result<ResolvedRead, ResolverDiagnostic>;

    fn read_with_abort(
        &self,
        request: &ResolveRequest,
        abort: &AbortSignal,
    ) -> Result<ResolvedRead, ResolverDiagnostic> {
        ensure_resolver_active(abort, &request.uri, request.purpose, ResolveDirection::Read)?;
        let resolved = self.read(request)?;
        ensure_resolver_active(abort, &request.uri, request.purpose, ResolveDirection::Read)?;
        Ok(resolved)
    }

    fn write(
        &self,
        request: &ResolveRequest,
        bytes: &[u8],
    ) -> Result<ResolvedWrite, ResolverDiagnostic>;

    fn write_with_abort(
        &self,
        request: &ResolveRequest,
        bytes: &[u8],
        abort: &AbortSignal,
    ) -> Result<ResolvedWrite, ResolverDiagnostic> {
        ensure_resolver_active(
            abort,
            &request.uri,
            request.purpose,
            ResolveDirection::Write,
        )?;
        self.write(request, bytes)
    }

    fn publication_capability(&self) -> PublicationCapability {
        PublicationCapability::DirectOnly
    }

    fn prepare_write(
        &self,
        request: &ResolveRequest,
        _bytes: &[u8],
    ) -> Result<Box<dyn PreparedResolverWrite>, ResolverDiagnostic> {
        Err(ResolverDiagnostic::TransactionUnsupported {
            uri: request.uri.clone(),
            purpose: request.purpose,
        })
    }

    fn prepare_write_with_abort(
        &self,
        request: &ResolveRequest,
        bytes: &[u8],
        abort: &AbortSignal,
    ) -> Result<Box<dyn PreparedResolverWrite>, ResolverDiagnostic> {
        ensure_resolver_active(
            abort,
            &request.uri,
            request.purpose,
            ResolveDirection::Write,
        )?;
        let prepared = self.prepare_write(request, bytes)?;
        ensure_resolver_active(
            abort,
            &request.uri,
            request.purpose,
            ResolveDirection::Write,
        )?;
        Ok(prepared)
    }

    fn list(
        &self,
        request: &ResolveListRequest,
    ) -> Result<Vec<ResolvedListEntry>, ResolverDiagnostic> {
        Err(ResolverDiagnostic::UnsupportedResolver {
            uri: request.uri.clone(),
            purpose: request.purpose,
            direction: ResolveDirection::List,
        })
    }

    fn list_with_abort(
        &self,
        request: &ResolveListRequest,
        abort: &AbortSignal,
    ) -> Result<Vec<ResolvedListEntry>, ResolverDiagnostic> {
        ensure_resolver_active(abort, &request.uri, request.purpose, ResolveDirection::List)?;
        let entries = self.list(request)?;
        ensure_resolver_active(abort, &request.uri, request.purpose, ResolveDirection::List)?;
        Ok(entries)
    }
}

fn ensure_resolver_active(
    abort: &AbortSignal,
    uri: &str,
    purpose: ResolvePurpose,
    direction: ResolveDirection,
) -> Result<(), ResolverDiagnostic> {
    if abort.is_aborted() {
        Err(ResolverDiagnostic::Cancelled {
            uri: uri.to_owned(),
            purpose,
            direction,
            source_map: abort.source_map(),
        })
    } else {
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ResolverKey {
    pub scheme: String,
    pub purpose: ResolvePurpose,
    pub direction: ResolveDirection,
}

#[derive(Clone, Default)]
pub struct ResolverRegistry {
    resolvers: BTreeMap<ResolverKey, Arc<dyn ResourceResolver>>,
}

impl fmt::Debug for ResolverRegistry {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ResolverRegistry")
            .field("resolver_count", &self.resolvers.len())
            .finish()
    }
}

impl ResolverRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn register<R>(
        &mut self,
        scheme: impl Into<String>,
        purpose: ResolvePurpose,
        direction: ResolveDirection,
        resolver: R,
    ) -> Option<Arc<dyn ResourceResolver>>
    where
        R: ResourceResolver + 'static,
    {
        self.register_arc(scheme, purpose, direction, Arc::new(resolver))
    }

    pub fn register_arc(
        &mut self,
        scheme: impl Into<String>,
        purpose: ResolvePurpose,
        direction: ResolveDirection,
        resolver: Arc<dyn ResourceResolver>,
    ) -> Option<Arc<dyn ResourceResolver>> {
        self.resolvers.insert(
            ResolverKey {
                scheme: normalize_scheme(scheme.into()),
                purpose,
                direction,
            },
            resolver,
        )
    }

    pub fn resolver_for(
        &self,
        scheme: &str,
        purpose: ResolvePurpose,
        direction: ResolveDirection,
    ) -> Option<Arc<dyn ResourceResolver>> {
        self.resolvers
            .get(&ResolverKey {
                scheme: normalize_scheme(scheme),
                purpose,
                direction,
            })
            .cloned()
    }

    pub fn read(&self, request: &ResolveRequest) -> Result<ResolvedRead, ResolverDiagnostic> {
        self.read_with_abort(request, &AbortSignal::new())
    }

    pub fn read_with_abort(
        &self,
        request: &ResolveRequest,
        abort: &AbortSignal,
    ) -> Result<ResolvedRead, ResolverDiagnostic> {
        ensure_resolver_active(abort, &request.uri, request.purpose, ResolveDirection::Read)?;
        let scheme = request_scheme(request)?;
        let Some(resolver) = self.resolver_for(scheme, request.purpose, ResolveDirection::Read)
        else {
            return Err(ResolverDiagnostic::UnsupportedResolver {
                uri: request.uri.clone(),
                purpose: request.purpose,
                direction: ResolveDirection::Read,
            });
        };
        resolver.read_with_abort(request, abort)
    }

    pub fn write(
        &self,
        request: &ResolveRequest,
        bytes: &[u8],
    ) -> Result<ResolvedWrite, ResolverDiagnostic> {
        self.write_with_abort(request, bytes, &AbortSignal::new())
    }

    pub fn write_with_abort(
        &self,
        request: &ResolveRequest,
        bytes: &[u8],
        abort: &AbortSignal,
    ) -> Result<ResolvedWrite, ResolverDiagnostic> {
        ensure_resolver_active(
            abort,
            &request.uri,
            request.purpose,
            ResolveDirection::Write,
        )?;
        let scheme = request_scheme(request)?;
        let Some(resolver) = self.resolver_for(scheme, request.purpose, ResolveDirection::Write)
        else {
            return Err(ResolverDiagnostic::UnsupportedResolver {
                uri: request.uri.clone(),
                purpose: request.purpose,
                direction: ResolveDirection::Write,
            });
        };
        resolver.write_with_abort(request, bytes, abort)
    }

    pub fn publication_capability(
        &self,
        request: &ResolveRequest,
    ) -> Result<PublicationCapability, ResolverDiagnostic> {
        let scheme = request_scheme(request)?;
        let Some(resolver) = self.resolver_for(scheme, request.purpose, ResolveDirection::Write)
        else {
            return Err(ResolverDiagnostic::UnsupportedResolver {
                uri: request.uri.clone(),
                purpose: request.purpose,
                direction: ResolveDirection::Write,
            });
        };
        Ok(resolver.publication_capability())
    }

    pub fn prepare_write_with_abort(
        &self,
        request: &ResolveRequest,
        bytes: &[u8],
        abort: &AbortSignal,
    ) -> Result<Box<dyn PreparedResolverWrite>, ResolverDiagnostic> {
        ensure_resolver_active(
            abort,
            &request.uri,
            request.purpose,
            ResolveDirection::Write,
        )?;
        let scheme = request_scheme(request)?;
        let Some(resolver) = self.resolver_for(scheme, request.purpose, ResolveDirection::Write)
        else {
            return Err(ResolverDiagnostic::UnsupportedResolver {
                uri: request.uri.clone(),
                purpose: request.purpose,
                direction: ResolveDirection::Write,
            });
        };
        resolver.prepare_write_with_abort(request, bytes, abort)
    }

    pub fn list(
        &self,
        request: &ResolveListRequest,
    ) -> Result<Vec<ResolvedListEntry>, ResolverDiagnostic> {
        self.list_with_abort(request, &AbortSignal::new())
    }

    pub fn list_with_abort(
        &self,
        request: &ResolveListRequest,
        abort: &AbortSignal,
    ) -> Result<Vec<ResolvedListEntry>, ResolverDiagnostic> {
        ensure_resolver_active(abort, &request.uri, request.purpose, ResolveDirection::List)?;
        let scheme = list_request_scheme(request)?;
        let Some(resolver) = self.resolver_for(scheme, request.purpose, ResolveDirection::List)
        else {
            return Err(ResolverDiagnostic::UnsupportedResolver {
                uri: request.uri.clone(),
                purpose: request.purpose,
                direction: ResolveDirection::List,
            });
        };
        resolver.list_with_abort(request, abort)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LocalFileUriError {
    NonLocalFileUri,
    InvalidPercentEscape,
}

impl LocalFileUriError {
    pub fn message(self) -> &'static str {
        match self {
            Self::NonLocalFileUri => "only local file:// URIs are supported",
            Self::InvalidPercentEscape => "file:// URI contains an invalid percent escape",
        }
    }
}

impl fmt::Display for LocalFileUriError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.message())
    }
}

impl std::error::Error for LocalFileUriError {}

pub fn local_path_or_file_uri<'a>(path: &'a Path, label: &str) -> io::Result<Cow<'a, Path>> {
    let raw = path.to_string_lossy();
    if raw.starts_with("file://") {
        return match parse_local_file_uri(&raw) {
            Some(Ok(path)) => Ok(Cow::Owned(path)),
            _ => Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                format!("unsupported {label} `{raw}`; only local file:// URIs are supported"),
            )),
        };
    }

    if uri_scheme(&raw).is_some() && !is_windows_drive_path(&raw) {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("unsupported {label} `{raw}`; remote/custom URI resolvers are not implemented"),
        ));
    }

    Ok(Cow::Borrowed(path))
}

pub fn local_file_uri_to_path(uri: &str) -> Option<PathBuf> {
    parse_local_file_uri(uri).and_then(Result::ok)
}

pub fn parse_local_file_uri(uri: &str) -> Option<Result<PathBuf, LocalFileUriError>> {
    let rest = uri.strip_prefix("file://")?;
    let path = if let Some(localhost_path) = rest.strip_prefix("localhost/") {
        format!("/{localhost_path}")
    } else if rest.starts_with('/') {
        rest.to_owned()
    } else {
        return Some(Err(LocalFileUriError::NonLocalFileUri));
    };

    Some(
        percent_decode_uri_path(&path)
            .map(PathBuf::from)
            .ok_or(LocalFileUriError::InvalidPercentEscape),
    )
}

pub fn has_uri_scheme(value: &str) -> bool {
    uri_scheme(value).is_some()
}

pub fn uri_scheme(value: &str) -> Option<&str> {
    let (scheme, _) = value.split_once(':')?;
    if !scheme.is_empty()
        && scheme
            .chars()
            .next()
            .is_some_and(|ch| ch.is_ascii_alphabetic())
        && scheme
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '+' | '-' | '.'))
    {
        Some(scheme)
    } else {
        None
    }
}

pub fn is_windows_drive_path(value: &str) -> bool {
    let bytes = value.as_bytes();
    bytes.len() >= 3
        && bytes[0].is_ascii_alphabetic()
        && bytes[1] == b':'
        && matches!(bytes[2], b'/' | b'\\')
}

fn request_scheme(request: &ResolveRequest) -> Result<&str, ResolverDiagnostic> {
    uri_scheme(&request.uri)
        .or_else(|| request.base_uri.as_deref().and_then(uri_scheme))
        .ok_or_else(|| ResolverDiagnostic::UnsupportedResolver {
            uri: request.uri.clone(),
            purpose: request.purpose,
            direction: request.direction,
        })
}

fn list_request_scheme(request: &ResolveListRequest) -> Result<&str, ResolverDiagnostic> {
    uri_scheme(&request.uri)
        .or_else(|| request.base_uri.as_deref().and_then(uri_scheme))
        .ok_or_else(|| ResolverDiagnostic::UnsupportedResolver {
            uri: request.uri.clone(),
            purpose: request.purpose,
            direction: ResolveDirection::List,
        })
}

fn normalize_policy_uri(value: &str) -> String {
    value.trim().to_owned()
}

fn stamped_policy_map<'a, I>(entries: I) -> String
where
    I: Iterator<Item = (&'a ResolvePolicyRequestKey, &'a str, &'a str)>,
{
    entries
        .map(|(key, target, reason)| {
            format!(
                "{}:{}|{}|{}|{}|{}=>{}:{}:{}",
                key.uri.len(),
                key.uri,
                key.base_uri.as_deref().unwrap_or(""),
                key.content_type_hint.as_deref().unwrap_or(""),
                key.purpose,
                key.direction,
                target.len(),
                target,
                reason
            )
        })
        .collect::<Vec<_>>()
        .join(",")
}

fn normalize_scheme(scheme: impl AsRef<str>) -> String {
    scheme
        .as_ref()
        .trim()
        .trim_end_matches(':')
        .to_ascii_lowercase()
}

fn percent_decode_uri_path(path: &str) -> Option<String> {
    let bytes = path.as_bytes();
    let mut decoded = Vec::with_capacity(bytes.len());
    let mut index = 0;
    while index < bytes.len() {
        if bytes[index] == b'%' {
            let high = *bytes.get(index + 1)?;
            let low = *bytes.get(index + 2)?;
            decoded.push((hex_value(high)? << 4) | hex_value(low)?);
            index += 3;
        } else {
            decoded.push(bytes[index]);
            index += 1;
        }
    }
    String::from_utf8(decoded).ok()
}

fn hex_value(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'a'..=b'f' => Some(byte - b'a' + 10),
        b'A'..=b'F' => Some(byte - b'A' + 10),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug)]
    struct EchoResolver;

    impl ResourceResolver for EchoResolver {
        fn read(&self, request: &ResolveRequest) -> Result<ResolvedRead, ResolverDiagnostic> {
            Ok(ResolvedRead {
                uri: request.uri.clone(),
                bytes: request.uri.as_bytes().to_vec(),
                content_type: request.content_type_hint.clone(),
            })
        }

        fn write(
            &self,
            request: &ResolveRequest,
            bytes: &[u8],
        ) -> Result<ResolvedWrite, ResolverDiagnostic> {
            Ok(ResolvedWrite {
                uri: format!("{}#{}b", request.uri, bytes.len()),
            })
        }

        fn list(
            &self,
            request: &ResolveListRequest,
        ) -> Result<Vec<ResolvedListEntry>, ResolverDiagnostic> {
            Ok(vec![ResolvedListEntry {
                uri: request.uri.replace('*', "a"),
                content_type: request.content_type_hint.clone(),
            }])
        }
    }

    #[test]
    fn local_file_uri_decodes_percent_escaped_paths() {
        assert_eq!(
            local_file_uri_to_path("file:///tmp/cem%20ml/out.json").unwrap(),
            PathBuf::from("/tmp/cem ml/out.json")
        );
        assert_eq!(
            local_file_uri_to_path("file://localhost/tmp/cem%23ml/out.json").unwrap(),
            PathBuf::from("/tmp/cem#ml/out.json")
        );
    }

    #[test]
    fn local_file_uri_rejects_non_local_hosts() {
        assert_eq!(
            parse_local_file_uri("file://example.test/tmp/out.json"),
            Some(Err(LocalFileUriError::NonLocalFileUri))
        );
        assert_eq!(
            local_file_uri_to_path("file://example.test/tmp/out.json"),
            None
        );
    }

    #[test]
    fn local_file_uri_rejects_malformed_percent_escapes() {
        assert_eq!(
            parse_local_file_uri("file:///tmp/cem%2/out.json"),
            Some(Err(LocalFileUriError::InvalidPercentEscape))
        );
        assert_eq!(
            parse_local_file_uri("file:///tmp/cem%zz/out.json"),
            Some(Err(LocalFileUriError::InvalidPercentEscape))
        );
    }

    #[test]
    fn local_path_or_file_uri_rejects_remote_or_custom_uri_schemes() {
        let err = local_path_or_file_uri(
            Path::new("https://example.test/out.json"),
            "output destination",
        )
        .unwrap_err();
        assert_eq!(err.kind(), io::ErrorKind::InvalidInput);
        assert!(err
            .to_string()
            .contains("remote/custom URI resolvers are not implemented"));

        assert!(
            local_path_or_file_uri(Path::new("relative/out.json"), "output destination").is_ok()
        );
    }

    #[test]
    fn local_path_or_file_uri_rejects_non_local_file_uri_hosts() {
        let err = local_path_or_file_uri(
            Path::new("file://example.test/out.json"),
            "output destination",
        )
        .unwrap_err();

        assert_eq!(err.kind(), io::ErrorKind::InvalidInput);
        assert!(err
            .to_string()
            .contains("only local file:// URIs are supported"));
    }

    #[test]
    fn windows_drive_paths_are_not_uri_schemes_for_local_path_checks() {
        assert_eq!(uri_scheme("C:/tmp/out.json"), Some("C"));
        assert!(is_windows_drive_path("C:/tmp/out.json"));
        assert!(local_path_or_file_uri(Path::new("C:/tmp/out.json"), "input URI").is_ok());
    }

    #[test]
    fn resolve_request_builder_preserves_purpose_and_hints() {
        let request = ResolveRequest::new(
            "cem+vfs://root/input.cem",
            ResolvePurpose::Input,
            ResolveDirection::Read,
        )
        .with_base_uri("file:///tmp/root/")
        .with_content_type_hint("application/cem+xml");

        assert_eq!(request.purpose.as_str(), "input");
        assert_eq!(request.direction.as_str(), "read");
        assert_eq!(request.base_uri.as_deref(), Some("file:///tmp/root/"));
        assert_eq!(
            request.content_type_hint.as_deref(),
            Some("application/cem+xml")
        );
        assert_eq!(ResolvePurpose::Template.as_str(), "template");
    }

    #[test]
    fn resolver_policy_defaults_to_passthrough_decisions() {
        let request =
            ResolveRequest::new("ui.cem", ResolvePurpose::Template, ResolveDirection::Read)
                .with_base_uri("cem+vfs://templates/main.cem")
                .with_content_type_hint("text/cem-ml");

        let decision = ResolverPolicy::new()
            .decide(&request)
            .expect("default policy allows pass-through");

        assert_eq!(decision.requested_uri, "ui.cem");
        assert_eq!(decision.normalized_uri, "ui.cem");
        assert_eq!(decision.effective_uri, "ui.cem");
        assert_eq!(decision.substituted_uri, None);
        assert_eq!(decision.reason, None);
        assert!(decision
            .policy_stamp
            .starts_with("resolver-policy/1;substitutions="));
    }

    #[test]
    fn resolver_policy_applies_exact_substitution_before_resolver_read() {
        let request =
            ResolveRequest::new(" ui.cem ", ResolvePurpose::Template, ResolveDirection::Read)
                .with_base_uri("cem+vfs://templates/main.cem")
                .with_content_type_hint("text/cem-ml");
        let policy = ResolverPolicy::new().with_substitution(
            ResolvePolicyRequestKey::new(
                "ui.cem",
                ResolvePurpose::Template,
                ResolveDirection::Read,
            )
            .with_base_uri("cem+vfs://templates/main.cem")
            .with_content_type_hint("text/cem-ml"),
            ResolvePolicySubstitution::new("substitute.cem", "fixture-substitution"),
        );

        let decision = policy
            .decide(&request)
            .expect("policy applies matching substitution");

        assert_eq!(decision.requested_uri, " ui.cem ");
        assert_eq!(decision.normalized_uri, "ui.cem");
        assert_eq!(decision.effective_uri, "substitute.cem");
        assert_eq!(decision.substituted_uri.as_deref(), Some("substitute.cem"));
        assert_eq!(decision.reason.as_deref(), Some("fixture-substitution"));
        assert!(decision.policy_stamp.contains("substitute.cem"));
    }

    #[test]
    fn resolver_policy_allows_broad_substitution_keys_without_base_uri() {
        let request =
            ResolveRequest::new("ui.cem", ResolvePurpose::Template, ResolveDirection::Read)
                .with_base_uri("cem+vfs://templates/main.cem");
        let policy = ResolverPolicy::new().with_substitution(
            ResolvePolicyRequestKey::new(
                "ui.cem",
                ResolvePurpose::Template,
                ResolveDirection::Read,
            ),
            ResolvePolicySubstitution::new("fallback.cem", "global-fixture-substitution"),
        );

        let decision = policy
            .decide(&request)
            .expect("base-less policy key applies broadly");

        assert_eq!(decision.effective_uri, "fallback.cem");
        assert_eq!(decision.substituted_uri.as_deref(), Some("fallback.cem"));
    }

    #[test]
    fn resolver_policy_reports_exact_denial_before_resolver_read() {
        let request = ResolveRequest::new(
            "https://example.test/template.cem",
            ResolvePurpose::Template,
            ResolveDirection::Read,
        );
        let policy = ResolverPolicy::new().with_denial(
            ResolvePolicyRequestKey::new(
                "https://example.test/template.cem",
                ResolvePurpose::Template,
                ResolveDirection::Read,
            ),
            ResolvePolicyDenial::new("network-disabled"),
        );

        let diagnostic = policy
            .decide(&request)
            .expect_err("policy denial blocks read before resolver dispatch");

        assert_eq!(diagnostic.request.uri, "https://example.test/template.cem");
        assert_eq!(diagnostic.reason, "network-disabled");
        assert!(diagnostic.policy_stamp.contains("network-disabled"));
    }

    #[test]
    fn resolver_registry_dispatches_by_scheme_purpose_and_direction() {
        let mut registry = ResolverRegistry::new();
        registry.register(
            "CEM+VFS:",
            ResolvePurpose::Input,
            ResolveDirection::Read,
            EchoResolver,
        );
        registry.register(
            "cem+vfs",
            ResolvePurpose::Output,
            ResolveDirection::Write,
            EchoResolver,
        );

        let read = registry
            .read(
                &ResolveRequest::new(
                    "cem+vfs://root/input.cem",
                    ResolvePurpose::Input,
                    ResolveDirection::Read,
                )
                .with_content_type_hint("application/cem+xml"),
            )
            .unwrap();
        assert_eq!(read.uri, "cem+vfs://root/input.cem");
        assert_eq!(read.content_type.as_deref(), Some("application/cem+xml"));

        let write = registry
            .write(
                &ResolveRequest::new(
                    "cem+vfs://root/out.json",
                    ResolvePurpose::Output,
                    ResolveDirection::Write,
                ),
                b"{}",
            )
            .unwrap();
        assert_eq!(write.uri, "cem+vfs://root/out.json#2b");

        registry.register(
            "cem+vfs",
            ResolvePurpose::Input,
            ResolveDirection::List,
            EchoResolver,
        );
        let listed = registry
            .list(
                &ResolveListRequest::new("cem+vfs://root/*.cem", ResolvePurpose::Input)
                    .with_content_type_hint("text/cem-ml")
                    .with_max_entries(10),
            )
            .unwrap();
        assert_eq!(listed[0].uri, "cem+vfs://root/a.cem");
        assert_eq!(listed[0].content_type.as_deref(), Some("text/cem-ml"));

        let err = registry
            .write(
                &ResolveRequest::new(
                    "cem+vfs://root/input.cem",
                    ResolvePurpose::Input,
                    ResolveDirection::Write,
                ),
                b"",
            )
            .unwrap_err();
        assert_eq!(err.code(), "cem.resolver.unsupported");
    }

    #[test]
    fn pre_cancelled_resolver_request_never_invokes_host_io() {
        #[derive(Debug)]
        struct PanicResolver;

        impl ResourceResolver for PanicResolver {
            fn read(&self, _: &ResolveRequest) -> Result<ResolvedRead, ResolverDiagnostic> {
                panic!("pre-cancelled resolver read must not reach host I/O")
            }

            fn write(
                &self,
                _: &ResolveRequest,
                _: &[u8],
            ) -> Result<ResolvedWrite, ResolverDiagnostic> {
                panic!("pre-cancelled resolver write must not reach host I/O")
            }

            fn list(
                &self,
                _: &ResolveListRequest,
            ) -> Result<Vec<ResolvedListEntry>, ResolverDiagnostic> {
                panic!("pre-cancelled resolver list must not reach host I/O")
            }
        }

        let mut registry = ResolverRegistry::new();
        registry.register(
            "cem+vfs",
            ResolvePurpose::Input,
            ResolveDirection::Read,
            PanicResolver,
        );
        registry.register(
            "cem+vfs",
            ResolvePurpose::Output,
            ResolveDirection::Write,
            PanicResolver,
        );
        registry.register(
            "cem+vfs",
            ResolvePurpose::Input,
            ResolveDirection::List,
            PanicResolver,
        );
        let abort = crate::scheduler::AbortSignal::new();
        abort.abort();
        let request = ResolveRequest::new(
            "cem+vfs://root/input.cem",
            ResolvePurpose::Input,
            ResolveDirection::Read,
        );

        let error = registry
            .read_with_abort(&request, &abort)
            .expect_err("pre-cancelled read must fail");
        assert_eq!(error.code(), "cem.resolver.cancelled");

        let write_request = ResolveRequest::new(
            "cem+vfs://root/output.cem",
            ResolvePurpose::Output,
            ResolveDirection::Write,
        );
        let error = registry
            .write_with_abort(&write_request, b"must not commit", &abort)
            .expect_err("pre-cancelled write must fail");
        assert_eq!(error.code(), "cem.resolver.cancelled");

        let list_request = ResolveListRequest::new("cem+vfs://root/*.cem", ResolvePurpose::Input);
        let error = registry
            .list_with_abort(&list_request, &abort)
            .expect_err("pre-cancelled list must fail");
        assert_eq!(error.code(), "cem.resolver.cancelled");
    }

    #[test]
    fn cancellation_during_resolver_read_suppresses_the_resolved_value() {
        #[derive(Debug)]
        struct CancellingResolver(AbortSignal);

        impl ResourceResolver for CancellingResolver {
            fn read(&self, request: &ResolveRequest) -> Result<ResolvedRead, ResolverDiagnostic> {
                self.0.abort();
                Ok(ResolvedRead {
                    uri: request.uri.clone(),
                    bytes: b"partial".to_vec(),
                    content_type: None,
                })
            }

            fn write(
                &self,
                request: &ResolveRequest,
                _: &[u8],
            ) -> Result<ResolvedWrite, ResolverDiagnostic> {
                Ok(ResolvedWrite {
                    uri: request.uri.clone(),
                })
            }
        }

        let abort = AbortSignal::new();
        let mut registry = ResolverRegistry::new();
        registry.register(
            "cem+vfs",
            ResolvePurpose::Input,
            ResolveDirection::Read,
            CancellingResolver(abort.clone()),
        );
        let request = ResolveRequest::new(
            "cem+vfs://root/input.cem",
            ResolvePurpose::Input,
            ResolveDirection::Read,
        );

        let error = registry
            .read_with_abort(&request, &abort)
            .expect_err("cancelled read result must be suppressed");
        assert_eq!(error.code(), "cem.resolver.cancelled");
    }
}
