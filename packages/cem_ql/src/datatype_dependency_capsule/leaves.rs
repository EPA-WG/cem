//! Shared bounded leaf verification and complete original-source agreement.
use super::*;
pub(crate) type SourceAgreement = BTreeMap<String, (SourceIdentity, String)>;
pub(crate) fn remember_source(
    snapshots: &mut SourceAgreement,
    capture: &LexicallyScopedDocument,
    identity: &SourceIdentity,
    work: &Work<'_>,
) -> Result<(), String> {
    let snapshot =
        NativeDependencySourceSnapshot::capture(capture, identity, &mut || work.check())?;
    let entry = (identity.clone(), hash(&encode(&snapshot)?));
    if snapshots
        .get(&identity.uri)
        .is_some_and(|previous| previous != &entry)
    {
        return Err(INVALID.into());
    }
    snapshots.insert(identity.uri.clone(), entry);
    Ok(())
}
pub(crate) fn verify_leaf_artifacts(
    artifacts: &[DependencyArtifact],
    required: &BTreeSet<String>,
    registry: &str,
    own_bytes: usize,
    snapshots: &mut SourceAgreement,
    work: &Work<'_>,
    control: &OperationControl,
) -> Result<BTreeMap<String, VerifiedArtifact>, String> {
    if artifacts.len() > 64
        || artifacts.len() != required.len()
        || artifacts
            .iter()
            .map(|a| a.bytes.len())
            .try_fold(own_bytes, |sum, n| sum.checked_add(n))
            .is_none_or(|n| n > 4 * 1024 * 1024)
    {
        return Err(INVALID.into());
    }
    let mut dependencies = BTreeMap::new();
    for artifact in artifacts {
        work.check()?;
        let id = artifact.id();
        if !required.contains(id)
            || dependencies.contains_key(id)
            || artifact.pin.registry() != registry
        {
            return Err(INVALID.into());
        }
        let verified = match &artifact.pin {
            ArtifactPin::Datatype(p) => {
                VerifiedArtifact::Datatype(datatype::verify(&artifact.bytes, p, control)?)
            }
            ArtifactPin::Query(p) => {
                VerifiedArtifact::Query(query::verify(&artifact.bytes, p, control)?)
            }
            ArtifactPin::Native(p) => {
                VerifiedArtifact::Native(native::verify(&artifact.bytes, p, control)?)
            }
        };
        remember_source(snapshots, verified.capture(), verified.source(), work)?;
        dependencies.insert(id.to_owned(), verified);
    }
    Ok(dependencies)
}
