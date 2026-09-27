//! Effective ownership and artifact keys over ready retained stylesheet closures.
use super::CssManagedScope;
use crate::{
    css_imports::{CssImportClosure, CssImportState},
    css_resources::CssResourceKind,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CssStylesheetIdentity {
    /// Shared by hosts using this effective resolver context, across declarations.
    pub context_marker: Option<String>,
    /// Stable namespace input; content changes do not rename the stylesheet owner.
    pub owner_key: String,
    /// Includes every imported occurrence's source fingerprint and final URL.
    pub cache_key: String,
    pub scope: CssManagedScope,
}

fn field(hash: &mut blake3::Hasher, value: &str) {
    hash.update(&(value.len() as u64).to_le_bytes());
    hash.update(value.as_bytes());
}

/// Derive identity before installation; this neither caches nor registers CSS.
/// `declaration` and `occurrence` are stable authoring/artifact identities, not
/// process counters. Instance callers include their persisted instance identity.
/// The input scope must be unqualified: this function owns context qualification.
/// Missing source/resolver fingerprints fail closed rather than using pointers.
/// A cache hit still requires current loader authorization/freshness and a live
/// consuming context. This key does not authorize skipping dependency revalidation.
pub fn derive_css_stylesheet_identity(
    closure: &CssImportClosure,
    scope: &CssManagedScope,
    declaration: &str,
    occurrence: &str,
) -> Result<CssStylesheetIdentity, &'static str> {
    if closure.state() != CssImportState::Ready || closure.failure().is_some() {
        return Err("stylesheet identity requires a ready, active import closure");
    }
    if declaration.is_empty() || occurrence.is_empty() {
        return Err("stylesheet identity requires stable declaration and occurrence identities");
    }
    let (kind, name) = match scope {
        CssManagedScope::Private { tag, context: None } => ("private", tag.as_str()),
        CssManagedScope::Shared {
            name,
            context: None,
        } => ("shared", name.as_str()),
        CssManagedScope::Instance => ("instance", ""),
        _ => return Err("stylesheet scope must not carry a caller-supplied context marker"),
    };
    super::emit_css_scope_wrapper(scope).map_err(|_| "invalid managed stylesheet scope")?;
    let dependent = closure.sheets().iter().any(|sheet| {
        sheet.resources.references.iter().any(|r| {
            matches!(r.kind, CssResourceKind::Import { .. })
                || !r.authored_specifier.starts_with('#')
        })
    });
    let context_marker = if dependent {
        Some(
            closure
                .stable_resolution_identity()
                .ok_or("resolver context changed or does not expose a stable fingerprint")?
                .to_owned(),
        )
    } else {
        None
    };
    let mut hash = blake3::Hasher::new_derive_key("cem effective stylesheet owner/1");
    let root = closure
        .sheets()
        .first()
        .ok_or("stylesheet closure has no root")?;
    for value in [
        declaration,
        occurrence,
        kind,
        name,
        root.url.as_str(),
        context_marker.as_deref().unwrap_or(""),
    ] {
        field(&mut hash, value);
    }
    let owner_key = format!("cem-style-1-{}", hash.finalize().to_hex());
    let mut hash = blake3::Hasher::new_derive_key("cem compiled stylesheet cache/1");
    field(&mut hash, &owner_key);
    hash.update(&(closure.sheets().len() as u64).to_le_bytes());
    for sheet in closure.sheets() {
        field(&mut hash, &sheet.url);
        field(
            &mut hash,
            &sheet
                .resources
                .tree
                .source_key(0)
                .ok_or("retained stylesheet has no import-owned source fingerprint")?,
        );
    }
    hash.update(&(closure.edges().len() as u64).to_le_bytes());
    for edge in closure.edges() {
        for value in [
            edge.parent_sheet as u64,
            edge.import_node as u64,
            edge.child_sheet as u64,
        ] {
            hash.update(&value.to_le_bytes());
        }
    }
    let cache_key = format!("cem-css-cache-1-{}", hash.finalize().to_hex());
    let scope = match scope {
        CssManagedScope::Private { tag, .. } => CssManagedScope::Private {
            tag: tag.clone(),
            context: context_marker.clone(),
        },
        CssManagedScope::Shared { name, .. } => CssManagedScope::Shared {
            name: name.clone(),
            context: context_marker.clone(),
        },
        CssManagedScope::Instance => CssManagedScope::Instance,
    };
    Ok(CssStylesheetIdentity {
        context_marker,
        owner_key,
        cache_key,
        scope,
    })
}
