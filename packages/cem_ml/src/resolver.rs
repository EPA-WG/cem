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

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ResolvePurpose {
    Config,
    Input,
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ResolverDiagnostic {
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
}

impl ResolverDiagnostic {
    pub fn code(&self) -> &'static str {
        match self {
            Self::UnsupportedResolver { .. } => "cem.resolver.unsupported",
            Self::NonLocalFileUri { .. } => "cem.resolver.file_uri_non_local",
            Self::InvalidFileUri { .. } => "cem.resolver.file_uri_invalid",
            Self::Io { .. } => "cem.resolver.io",
        }
    }
}

impl fmt::Display for ResolverDiagnostic {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
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
        }
    }
}

impl std::error::Error for ResolverDiagnostic {}

pub trait ResourceResolver: Send + Sync {
    fn read(&self, request: &ResolveRequest) -> Result<ResolvedRead, ResolverDiagnostic>;
    fn write(
        &self,
        request: &ResolveRequest,
        bytes: &[u8],
    ) -> Result<ResolvedWrite, ResolverDiagnostic>;
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
        let scheme = request_scheme(request)?;
        let Some(resolver) = self.resolver_for(scheme, request.purpose, ResolveDirection::Read)
        else {
            return Err(ResolverDiagnostic::UnsupportedResolver {
                uri: request.uri.clone(),
                purpose: request.purpose,
                direction: ResolveDirection::Read,
            });
        };
        resolver.read(request)
    }

    pub fn write(
        &self,
        request: &ResolveRequest,
        bytes: &[u8],
    ) -> Result<ResolvedWrite, ResolverDiagnostic> {
        let scheme = request_scheme(request)?;
        let Some(resolver) = self.resolver_for(scheme, request.purpose, ResolveDirection::Write)
        else {
            return Err(ResolverDiagnostic::UnsupportedResolver {
                uri: request.uri.clone(),
                purpose: request.purpose,
                direction: ResolveDirection::Write,
            });
        };
        resolver.write(request, bytes)
    }

    pub fn list(
        &self,
        request: &ResolveListRequest,
    ) -> Result<Vec<ResolvedListEntry>, ResolverDiagnostic> {
        let scheme = list_request_scheme(request)?;
        let Some(resolver) = self.resolver_for(scheme, request.purpose, ResolveDirection::List)
        else {
            return Err(ResolverDiagnostic::UnsupportedResolver {
                uri: request.uri.clone(),
                purpose: request.purpose,
                direction: ResolveDirection::List,
            });
        };
        resolver.list(request)
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
}
