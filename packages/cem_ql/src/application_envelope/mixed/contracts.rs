//! Reuse the deployed closed scalar policy and the parent's exact sealed native
//! contracts. No source/profile reinterpretation or executable owner is created.
use super::*;
pub(super) fn shape(
    receivers: &BTreeMap<String, ReceiverContract>,
    work: &Work<'_>,
) -> Result<(), String> {
    if receivers.is_empty() || receivers.len() > MAX_RECEIVERS {
        return Err(INVALID.into());
    }
    let mut scalar_bytes = 0usize;
    for (name, receiver) in receivers {
        work.check()?;
        if name.is_empty()
            || name.len() > 256
            || application_declaration::reserved(name)
            || !name
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_' | b':'))
        {
            return Err(INVALID.into());
        }
        if let ReceiverContract::Scalar(value) = receiver {
            if value.name != *name || !application_declaration::valid_type(value) {
                return Err(INVALID.into());
            }
            scalar_bytes = scalar_bytes
                .checked_add(value.value.accounted_bytes())
                .ok_or(INVALID)?;
            if scalar_bytes > MAX_BYTES {
                return Err(INVALID.into());
            }
        }
    }
    Ok(())
}
pub(super) fn check(
    receivers: &BTreeMap<String, ReceiverContract>,
    application: &super::super::v3::VerifiedApplicationEnvelope,
    work: &Work<'_>,
) -> Result<(), String> {
    let mut sources = BTreeMap::new();
    let mut remember = |uri: &str, hash: &str| -> Result<(), String> {
        work.check()?;
        if sources
            .insert(uri.to_string(), hash.to_string())
            .is_some_and(|old| old != hash)
        {
            return Err(INVALID.into());
        }
        Ok(())
    };
    for contract in application.application().contracts.values() {
        let source = match contract {
            super::super::VerifiedContract::Structural(v) => &v.pin().source,
            super::super::VerifiedContract::Query(v) => &v.pin().program.source,
            super::super::VerifiedContract::Native(v) => &v.pin().source,
        };
        remember(&source.uri, &source.hash)?;
    }
    for id in application.assembly_ids() {
        let graph = application.assembly(id).ok_or(INVALID)?;
        for id in graph.module_ids() {
            let source = graph.module(id).ok_or(INVALID)?.identity();
            remember(&source.uri, &source.hash)?;
        }
        for id in graph.leaf_ids() {
            let source = graph.leaf(id).ok_or(INVALID)?.source();
            remember(&source.uri, &source.hash)?;
        }
    }
    for receiver in receivers.values() {
        work.check()?;
        match receiver {
            ReceiverContract::Node(pin) => {
                let native = application
                    .application()
                    .structural(&pin.hash)
                    .ok_or(INVALID)?;
                if encode(pin)? != encode(native.pin())? {
                    return Err(INVALID.into());
                }
            }
            ReceiverContract::Scalar(value) => remember(&value.source_uri, &value.source_hash)?,
            ReceiverContract::Executable { root, target } => {
                if application.roots().get(root) != Some(target) {
                    return Err(INVALID.into());
                }
                let graph = application.assembly(&target.assembly).ok_or(INVALID)?;
                if !graph
                    .module(&graph.pin().root)
                    .ok_or(INVALID)?
                    .roots()
                    .contains(&target.datatype)
                {
                    return Err(INVALID.into());
                }
            }
        }
    }
    Ok(())
}
