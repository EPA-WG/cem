use super::*;
pub(super) fn account(
    own: usize,
    artifacts: &[ProfileArtifact],
    work: &Work<'_>,
) -> Result<usize, ModuleVerificationError> {
    if artifacts.len() > 32 {
        return Err(refused("datatype-module-profile-limit"));
    }
    let mut total = own;
    let mut sources = 0usize;
    for a in artifacts {
        work.check()?;
        if a.bytes.len() > 64 * 1024 || a.sources.len() > 64 {
            return Err(refused("datatype-module-profile-limit"));
        }
        total = total
            .checked_add(a.bytes.len())
            .ok_or("profile bytes overflow")?;
        sources = sources
            .checked_add(a.sources.len())
            .ok_or("profile count overflow")?;
        for s in &a.sources {
            work.check()?;
            if s.bytes.len() > 64 * 1024 {
                return Err(refused("datatype-module-profile-limit"));
            }
            total = total
                .checked_add(s.bytes.len())
                .ok_or("profile bytes overflow")?;
        }
        if total > MAX_BYTES || sources > 64 {
            return Err(refused("datatype-module-profile-limit"));
        }
    }
    Ok(total)
}
pub(super) fn inspect(
    records: &BTreeMap<String, ModuleRecord>,
    modules: &BTreeMap<String, VerifiedDatatypeModule>,
    artifacts: &[ProfileArtifact],
    registry: &str,
    agreement: &mut SourceAgreement,
    work: &Work<'_>,
    control: &OperationControl,
) -> Result<
    BTreeMap<String, BTreeMap<AstNodeId, profiles::VerifiedOperationProfile>>,
    ModuleVerificationError,
> {
    let mut required = BTreeMap::new();
    let mut count = 0;
    for m in records.values() {
        for selection in m.profiles.values() {
            work.check()?;
            count += 1;
            if count > 64
                || selection.pin.registry_hash != registry
                || !valid_hash(&selection.pin.hash)
            {
                return Err(refused("datatype-module-profile-refused"));
            }
            required.insert(selection.pin.hash.clone(), &selection.pin);
        }
    }
    if required.len() != artifacts.len() {
        return Err(refused("datatype-module-profile-refused"));
    }
    let mut verified = BTreeMap::new();
    for a in artifacts {
        work.check()?;
        let pin = required
            .get(&a.pin.hash)
            .ok_or("unexpected profile artifact")?;
        if encode(*pin)? != encode(&a.pin)? || verified.contains_key(&a.pin.hash) {
            return Err(refused("datatype-module-profile-refused"));
        }
        let p = profiles::verify(&a.bytes, &a.pin, &a.sources, control)?;
        for id in p.profile_ids() {
            for r in p.profile(id).unwrap().requirements().values() {
                work.check()?;
                remember_source(agreement, r.capture(), &r.pin().source, work)?;
            }
        }
        verified.insert(a.pin.hash.clone(), p);
    }
    let mut used = BTreeSet::new();
    let mut result = BTreeMap::new();
    for (id, m) in records {
        let original = &modules[id];
        let mut profiles_out = BTreeMap::new();
        for (datatype, selection) in &m.profiles {
            work.check()?;
            if !original.sources.contains_key(datatype)
                || !used.insert((selection.pin.hash.clone(), selection.id.clone()))
            {
                return Err(refused("datatype-module-profile-subject-refused"));
            }
            let p = verified
                .get(&selection.pin.hash)
                .and_then(|p| p.profile(&selection.id))
                .ok_or("unknown profile selection")?;
            let expected = &original.sources[datatype];
            if p.source().scope().node_id() != expected.scope().node_id()
                || p.source().declaration().node_id() != *datatype
            {
                return Err(refused("datatype-module-profile-subject-refused"));
            }
            let bound = profiles::share_module_source(
                p,
                &original.identity,
                &original.capture,
                &original.source,
                control,
            )?;
            profiles_out.insert(*datatype, bound);
        }
        result.insert(id.clone(), profiles_out);
    }
    for (hash, p) in &verified {
        for id in p.profile_ids() {
            work.check()?;
            if !used.contains(&(hash.clone(), id.to_owned())) {
                return Err(refused("datatype-module-profile-unused"));
            }
        }
    }
    Ok(result)
}
