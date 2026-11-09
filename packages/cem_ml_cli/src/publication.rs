use std::collections::BTreeSet;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use cem_ml::engine::EngineContext;
use cem_ml::resolver::{
    is_windows_drive_path, local_file_uri_to_path, uri_scheme, PreparedResolverWrite,
    PublicationCapability, ResolveDirection, ResolvePurpose, ResolveRequest, ResolvedWrite,
    ResolverDiagnostic,
};

static NEXT_STAGING_ID: AtomicU64 = AtomicU64::new(1);

#[derive(Debug)]
pub(crate) struct PublicationItem {
    pub(crate) destination: PathBuf,
    pub(crate) label: &'static str,
    pub(crate) purpose: ResolvePurpose,
    pub(crate) bytes: Vec<u8>,
}

#[derive(Debug, Default)]
pub(crate) struct PublicationBatch {
    items: Vec<PublicationItem>,
}

impl PublicationBatch {
    pub(crate) fn new() -> Self {
        Self::default()
    }

    pub(crate) fn push(
        &mut self,
        destination: impl Into<PathBuf>,
        label: &'static str,
        purpose: ResolvePurpose,
        bytes: impl Into<Vec<u8>>,
    ) {
        self.items.push(PublicationItem {
            destination: destination.into(),
            label,
            purpose,
            bytes: bytes.into(),
        });
    }

    pub(crate) fn commit(self, context: &EngineContext) -> io::Result<()> {
        if self.items.is_empty() {
            return Ok(());
        }
        ensure_active(context)?;
        validate_unique_destinations(&self.items)?;
        if self.items.len() == 1 {
            return publish_direct(context, &self.items[0]);
        }

        preflight_transactional_destinations(context, &self.items)?;
        let mut prepared = Vec::with_capacity(self.items.len());
        for item in &self.items {
            prepared.push(prepare(context, item)?);
        }
        ensure_active(context)?;

        let result = (|| {
            for participant in &mut prepared {
                ensure_active(context)?;
                participant.commit()?;
            }
            ensure_active(context)
        })();
        if result.is_err() {
            for participant in prepared.iter_mut().rev() {
                let _ = participant.rollback();
            }
        }
        result
    }
}

/// Check SSR's complete local output plan without staging any files.
pub(crate) fn preflight_ssr_destinations(
    destinations: &[PathBuf],
    protected_sources: &[PathBuf],
) -> io::Result<()> {
    let mut keys = BTreeSet::new();
    let mut sources = BTreeSet::new();
    for source in protected_sources {
        if let Some(path) = local_destination(source, "SSR source")? {
            sources.insert(path_key(fs::canonicalize(path)?));
        }
    }
    for destination in destinations {
        let path = local_destination(destination, "SSR output")?.ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::Unsupported,
                "SSR graph publication requires local file destinations",
            )
        })?;
        let key = path_key(preflight_local_file(&path)?);
        if sources.contains(&key) {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                format!(
                    "SSR publication destination `{}` aliases a source file",
                    destination.display()
                ),
            ));
        }
        if !keys.insert(key) {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                format!(
                    "SSR publication destination `{}` is duplicated",
                    destination.display()
                ),
            ));
        }
    }
    for key in &keys {
        if key.ancestors().skip(1).any(|parent| keys.contains(parent)) {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                format!(
                    "SSR publication destination `{}` has another output as its parent",
                    key.display()
                ),
            ));
        }
    }
    Ok(())
}

fn path_key(path: PathBuf) -> PathBuf {
    #[cfg(windows)]
    {
        PathBuf::from(path.to_string_lossy().to_lowercase())
    }
    #[cfg(not(windows))]
    {
        path
    }
}

fn preflight_local_file(path: &Path) -> io::Result<PathBuf> {
    let absolute = std::path::absolute(path)?;
    let invalid = || {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            format!(
                "SSR output `{}` requires a regular file with directory parents",
                path.display()
            ),
        )
    };
    match fs::symlink_metadata(&absolute) {
        Ok(metadata) if !metadata.is_file() => return Err(invalid()),
        Ok(_) => return fs::canonicalize(&absolute),
        Err(error) if error.kind() == io::ErrorKind::NotFound => {}
        Err(error) => return Err(error),
    }

    // Resolve existing directory aliases, then append the as-yet missing names.
    let mut missing = vec![absolute.file_name().ok_or_else(invalid)?];
    for parent in absolute.ancestors().skip(1) {
        match fs::metadata(parent) {
            Ok(metadata) if metadata.is_dir() => {
                let mut canonical = fs::canonicalize(parent)?;
                canonical.extend(missing.into_iter().rev());
                return Ok(canonical);
            }
            Ok(_) => return Err(invalid()),
            Err(error) if error.kind() == io::ErrorKind::NotFound => {
                if fs::symlink_metadata(parent).is_ok() {
                    return Err(invalid());
                }
                missing.push(parent.file_name().ok_or_else(invalid)?);
            }
            Err(error) => return Err(error),
        }
    }
    Err(invalid())
}

fn replaceable_destination(path: &Path) -> io::Result<bool> {
    match fs::symlink_metadata(path) {
        Ok(metadata) if metadata.is_file() || metadata.file_type().is_symlink() => Ok(true),
        Ok(_) => Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!(
                "publication destination `{}` is not a regular file",
                path.display()
            ),
        )),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(false),
        Err(error) => Err(error),
    }
}

fn validate_unique_destinations(items: &[PublicationItem]) -> io::Result<()> {
    let mut destinations = BTreeSet::new();
    for item in items {
        let destination = item.destination.to_string_lossy().into_owned();
        if !destinations.insert(destination.clone()) {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                format!("transactional publication destination `{destination}` is duplicated"),
            ));
        }
    }
    Ok(())
}

fn preflight_transactional_destinations(
    context: &EngineContext,
    items: &[PublicationItem],
) -> io::Result<()> {
    for item in items {
        if local_destination(&item.destination, item.label)?.is_some() {
            continue;
        }
        let request = publication_request(item);
        let capability = context
            .resolver_registry
            .publication_capability(&request)
            .map_err(resolver_io_error)?;
        if capability != PublicationCapability::Transactional {
            return Err(resolver_io_error(
                ResolverDiagnostic::TransactionUnsupported {
                    uri: request.uri,
                    purpose: request.purpose,
                },
            ));
        }
    }
    Ok(())
}

fn prepare(context: &EngineContext, item: &PublicationItem) -> io::Result<PreparedPublication> {
    if let Some(path) = local_destination(&item.destination, item.label)? {
        return PreparedLocalWrite::prepare(path.as_path(), &item.bytes)
            .map(PreparedPublication::Local);
    }
    let request = publication_request(item);
    let prepared = context
        .resolver_registry
        .prepare_write_with_control(
            &request,
            &item.bytes,
            &context.operation_control,
            cem_ml::operation_control::ROOT_EXECUTION_SCOPE_ID,
        )
        .map_err(resolver_io_error)?;
    Ok(PreparedPublication::Resolver(prepared))
}

fn publish_direct(context: &EngineContext, item: &PublicationItem) -> io::Result<()> {
    if let Some(path) = local_destination(&item.destination, item.label)? {
        if let Some(parent) = path.parent() {
            if !parent.as_os_str().is_empty() {
                fs::create_dir_all(parent)?;
            }
        }
        ensure_active(context)?;
        return fs::write(path, &item.bytes);
    }
    let request = publication_request(item);
    context
        .resolver_registry
        .write_with_control(
            &request,
            &item.bytes,
            &context.operation_control,
            cem_ml::operation_control::ROOT_EXECUTION_SCOPE_ID,
        )
        .map(|_| ())
        .map_err(resolver_io_error)
}

fn publication_request(item: &PublicationItem) -> ResolveRequest {
    ResolveRequest::new(
        item.destination.to_string_lossy().into_owned(),
        item.purpose,
        ResolveDirection::Write,
    )
}

fn local_destination(path: &Path, label: &str) -> io::Result<Option<PathBuf>> {
    let raw = path.to_string_lossy();
    if raw.starts_with("file://") {
        return local_file_uri_to_path(&raw).map(Some).ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidInput,
                format!("unsupported {label} `{raw}`; only local file:// URIs are supported"),
            )
        });
    }
    if uri_scheme(&raw).is_some() && !is_windows_drive_path(&raw) {
        return Ok(None);
    }
    Ok(Some(path.to_path_buf()))
}

fn ensure_active(context: &EngineContext) -> io::Result<()> {
    context
        .ensure_active()
        .map_err(|error| io::Error::new(io::ErrorKind::Interrupted, error))
}

fn resolver_io_error(error: ResolverDiagnostic) -> io::Error {
    if let ResolverDiagnostic::UnsupportedResolver { uri, .. } = &error {
        return io::Error::new(
            io::ErrorKind::Unsupported,
            format!(
                "unsupported publication destination `{uri}`; remote/custom URI resolvers are not implemented"
            ),
        );
    }
    let kind = if matches!(error, ResolverDiagnostic::Cancelled { .. }) {
        io::ErrorKind::Interrupted
    } else if matches!(
        error,
        ResolverDiagnostic::TransactionUnsupported { .. }
            | ResolverDiagnostic::UnsupportedResolver { .. }
    ) {
        io::ErrorKind::Unsupported
    } else {
        io::ErrorKind::Other
    };
    io::Error::new(kind, error)
}

enum PreparedPublication {
    Local(PreparedLocalWrite),
    Resolver(Box<dyn PreparedResolverWrite>),
}

impl PreparedPublication {
    fn commit(&mut self) -> io::Result<()> {
        match self {
            Self::Local(local) => local.commit().map(|_| ()),
            Self::Resolver(resolver) => resolver.commit().map(|_| ()).map_err(resolver_io_error),
        }
    }

    fn rollback(&mut self) -> io::Result<()> {
        match self {
            Self::Local(local) => local.rollback(),
            Self::Resolver(resolver) => resolver.rollback().map_err(resolver_io_error),
        }
    }
}

#[derive(Debug)]
pub(crate) struct PreparedLocalWrite {
    destination: PathBuf,
    staged: PathBuf,
    backup: PathBuf,
    had_original: bool,
    committed: bool,
}

impl PreparedLocalWrite {
    pub(crate) fn prepare(destination: &Path, bytes: &[u8]) -> io::Result<Self> {
        let parent = destination.parent().unwrap_or_else(|| Path::new("."));
        if !parent.as_os_str().is_empty() {
            fs::create_dir_all(parent)?;
        }
        let file_name = destination
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("cem-publication");
        let staging_id = NEXT_STAGING_ID.fetch_add(1, Ordering::Relaxed);
        let staged = parent.join(format!(
            ".{file_name}.cem-stage-{}-{staging_id}",
            std::process::id()
        ));
        let backup = parent.join(format!(
            ".{file_name}.cem-backup-{}-{staging_id}",
            std::process::id()
        ));
        let had_original = replaceable_destination(destination)?;
        fs::write(&staged, bytes)?;
        Ok(Self {
            destination: destination.to_path_buf(),
            staged,
            backup,
            had_original,
            committed: false,
        })
    }

    pub(crate) fn commit(&mut self) -> io::Result<ResolvedWrite> {
        if self.committed {
            return Err(io::Error::other("publication write was already committed"));
        }
        if self.had_original != replaceable_destination(&self.destination)? {
            return Err(io::Error::new(
                io::ErrorKind::AlreadyExists,
                format!(
                    "publication destination `{}` changed after transaction preflight",
                    self.destination.display()
                ),
            ));
        }
        if self.had_original {
            fs::rename(&self.destination, &self.backup)?;
        }
        if let Err(error) = fs::rename(&self.staged, &self.destination) {
            if self.had_original {
                let _ = fs::rename(&self.backup, &self.destination);
            }
            return Err(error);
        }
        self.committed = true;
        Ok(ResolvedWrite {
            uri: self.destination.display().to_string(),
        })
    }

    pub(crate) fn rollback(&mut self) -> io::Result<()> {
        if self.committed {
            // From this point on, Drop must preserve or retry restoration of the
            // backup instead of treating the transaction as successfully finalized.
            self.committed = false;
            if fs::symlink_metadata(&self.destination).is_ok() {
                fs::remove_file(&self.destination)?;
            }
            if self.had_original && fs::symlink_metadata(&self.backup).is_ok() {
                fs::rename(&self.backup, &self.destination)?;
            }
        } else if self.staged.exists() {
            fs::remove_file(&self.staged)?;
        }
        Ok(())
    }
}

impl Drop for PreparedLocalWrite {
    fn drop(&mut self) {
        if self.committed {
            if self.had_original {
                let _ = fs::remove_file(&self.backup);
            }
        } else {
            let _ = fs::remove_file(&self.staged);
            if self.had_original
                && fs::symlink_metadata(&self.backup).is_ok()
                && fs::symlink_metadata(&self.destination)
                    .is_err_and(|error| error.kind() == io::ErrorKind::NotFound)
            {
                let _ = fs::rename(&self.backup, &self.destination);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cem_ml::resolver::{ResolvedRead, ResourceResolver};

    #[derive(Debug)]
    struct DirectOnlyResolver;

    impl ResourceResolver for DirectOnlyResolver {
        fn read(&self, request: &ResolveRequest) -> Result<ResolvedRead, ResolverDiagnostic> {
            Err(ResolverDiagnostic::UnsupportedResolver {
                uri: request.uri.clone(),
                purpose: request.purpose,
                direction: ResolveDirection::Read,
            })
        }

        fn write(
            &self,
            request: &ResolveRequest,
            _bytes: &[u8],
        ) -> Result<ResolvedWrite, ResolverDiagnostic> {
            Ok(ResolvedWrite {
                uri: request.uri.clone(),
            })
        }
    }

    #[derive(Debug, Clone)]
    enum FixtureCommit {
        Fail,
        Abort(cem_ml::scheduler::AbortSignal),
    }

    #[derive(Debug)]
    struct TransactionalResolver(FixtureCommit);

    #[derive(Debug)]
    struct PreparedFixtureWrite {
        uri: String,
        behavior: FixtureCommit,
    }

    impl PreparedResolverWrite for PreparedFixtureWrite {
        fn commit(&mut self) -> Result<ResolvedWrite, ResolverDiagnostic> {
            match &self.behavior {
                FixtureCommit::Fail => Err(ResolverDiagnostic::TransactionState {
                    uri: self.uri.clone(),
                    message: "fixture commit failure".to_owned(),
                }),
                FixtureCommit::Abort(signal) => {
                    signal.abort();
                    Ok(ResolvedWrite {
                        uri: self.uri.clone(),
                    })
                }
            }
        }

        fn rollback(&mut self) -> Result<(), ResolverDiagnostic> {
            Ok(())
        }
    }

    impl ResourceResolver for TransactionalResolver {
        fn read(&self, request: &ResolveRequest) -> Result<ResolvedRead, ResolverDiagnostic> {
            Err(ResolverDiagnostic::UnsupportedResolver {
                uri: request.uri.clone(),
                purpose: request.purpose,
                direction: ResolveDirection::Read,
            })
        }

        fn write(
            &self,
            request: &ResolveRequest,
            _bytes: &[u8],
        ) -> Result<ResolvedWrite, ResolverDiagnostic> {
            Ok(ResolvedWrite {
                uri: request.uri.clone(),
            })
        }

        fn publication_capability(&self) -> PublicationCapability {
            PublicationCapability::Transactional
        }

        fn prepare_write(
            &self,
            request: &ResolveRequest,
            _bytes: &[u8],
        ) -> Result<Box<dyn PreparedResolverWrite>, ResolverDiagnostic> {
            Ok(Box::new(PreparedFixtureWrite {
                uri: request.uri.clone(),
                behavior: self.0.clone(),
            }))
        }
    }

    fn fixture_dir(name: &str) -> PathBuf {
        let root = std::env::temp_dir().join(format!(
            "cem-ml-publication-{name}-{}-{}",
            std::process::id(),
            NEXT_STAGING_ID.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir_all(&root).unwrap();
        root
    }

    #[test]
    fn local_batch_commits_all_destinations() {
        let root = fixture_dir("commit");
        let first = root.join("first.txt");
        let second = root.join("second.txt");
        let mut batch = PublicationBatch::new();
        batch.push(&first, "output", ResolvePurpose::Output, b"first".to_vec());
        batch.push(
            &second,
            "report",
            ResolvePurpose::Report,
            b"second".to_vec(),
        );
        batch.commit(&EngineContext::default()).unwrap();
        assert_eq!(fs::read(first).unwrap(), b"first");
        assert_eq!(fs::read(second).unwrap(), b"second");
    }

    #[test]
    fn rollback_restores_preexisting_destination() {
        let root = fixture_dir("rollback");
        let destination = root.join("result.txt");
        fs::write(&destination, b"original").unwrap();
        let mut prepared = PreparedLocalWrite::prepare(&destination, b"replacement").unwrap();
        prepared.commit().unwrap();
        assert_eq!(fs::read(&destination).unwrap(), b"replacement");
        prepared.rollback().unwrap();
        assert_eq!(fs::read(destination).unwrap(), b"original");
    }

    #[test]
    fn rollback_error_preserves_backup_for_drop_retry() {
        let root = fixture_dir("rollback-retry");
        let destination = root.join("result.txt");
        fs::write(&destination, b"original").unwrap();
        let mut prepared = PreparedLocalWrite::prepare(&destination, b"replacement").unwrap();
        prepared.commit().unwrap();
        fs::remove_file(&destination).unwrap();
        fs::create_dir(&destination).unwrap();

        assert!(prepared.rollback().is_err());
        assert_eq!(fs::read(&prepared.backup).unwrap(), b"original");

        fs::remove_dir(&destination).unwrap();
        drop(prepared);
        assert_eq!(fs::read(destination).unwrap(), b"original");
    }

    #[test]
    fn destination_appearing_after_prepare_is_not_overwritten() {
        let root = fixture_dir("appeared-after-prepare");
        let destination = root.join("result.txt");
        let mut prepared = PreparedLocalWrite::prepare(&destination, b"staged").unwrap();
        fs::write(&destination, b"concurrent").unwrap();

        assert_eq!(
            prepared.commit().unwrap_err().kind(),
            io::ErrorKind::AlreadyExists
        );
        assert_eq!(fs::read(destination).unwrap(), b"concurrent");
    }

    #[test]
    fn duplicate_destination_fails_before_publication() {
        let root = fixture_dir("duplicate");
        let destination = root.join("same.txt");
        let mut batch = PublicationBatch::new();
        batch.push(
            &destination,
            "output",
            ResolvePurpose::Output,
            b"one".to_vec(),
        );
        batch.push(
            &destination,
            "report",
            ResolvePurpose::Report,
            b"two".to_vec(),
        );
        assert_eq!(
            batch.commit(&EngineContext::default()).unwrap_err().kind(),
            io::ErrorKind::InvalidInput
        );
        assert!(!destination.exists());
    }

    #[test]
    fn direct_only_resolver_is_rejected_before_local_staging() {
        let root = fixture_dir("direct-only");
        let local = root.join("local.txt");
        let mut context = EngineContext::default();
        context.resolver_registry.register(
            "fixture",
            ResolvePurpose::Output,
            ResolveDirection::Write,
            DirectOnlyResolver,
        );
        let mut batch = PublicationBatch::new();
        batch.push(&local, "output", ResolvePurpose::Output, b"local".to_vec());
        batch.push(
            "fixture://output/remote.txt",
            "output",
            ResolvePurpose::Output,
            b"remote".to_vec(),
        );
        let error = batch.commit(&context).unwrap_err();
        assert_eq!(error.kind(), io::ErrorKind::Unsupported);
        assert!(!local.exists());
    }

    #[test]
    fn later_participant_failure_rolls_back_committed_local_replacement() {
        let root = fixture_dir("participant-failure");
        let local = root.join("local.txt");
        fs::write(&local, b"original").unwrap();
        let mut context = EngineContext::default();
        context.resolver_registry.register(
            "fixture",
            ResolvePurpose::Output,
            ResolveDirection::Write,
            TransactionalResolver(FixtureCommit::Fail),
        );
        let mut batch = PublicationBatch::new();
        batch.push(
            &local,
            "output",
            ResolvePurpose::Output,
            b"replacement".to_vec(),
        );
        batch.push(
            "fixture://output/remote.txt",
            "output",
            ResolvePurpose::Output,
            b"remote".to_vec(),
        );
        assert!(batch.commit(&context).is_err());
        assert_eq!(fs::read(local).unwrap(), b"original");
    }
    #[test]
    fn cancellation_during_last_commit_restores_previous_destinations() {
        let root = fixture_dir("late-abort");
        let first = root.join("first.html");
        let second = root.join("second.json");
        fs::write(&first, b"original html").unwrap();
        fs::write(&second, b"original report").unwrap();
        let mut context = EngineContext::default();
        let signal = context.abort_signal().clone();
        context.resolver_registry.register(
            "fixture",
            ResolvePurpose::Output,
            ResolveDirection::Write,
            TransactionalResolver(FixtureCommit::Abort(signal)),
        );
        let mut batch = PublicationBatch::new();
        batch.push(
            &first,
            "html",
            ResolvePurpose::Output,
            b"replacement".to_vec(),
        );
        batch.push(
            &second,
            "report",
            ResolvePurpose::Output,
            b"replacement".to_vec(),
        );
        batch.push(
            "fixture://output/last",
            "output",
            ResolvePurpose::Output,
            Vec::new(),
        );
        assert_eq!(
            batch.commit(&context).unwrap_err().kind(),
            io::ErrorKind::Interrupted
        );
        assert_eq!(fs::read(&first).unwrap(), b"original html");
        assert_eq!(fs::read(&second).unwrap(), b"original report");
        assert_eq!(fs::read_dir(&root).unwrap().count(), 2);
        fs::remove_dir_all(root).unwrap();
    }
}
