//! Bounded portable resource identities. The browser must provide fresh bytes.
use cem_ml::{content_cache::ContentHash, resolver::ssr_resource_url};

pub const DECLARATION_ATTRIBUTE: &str = "data-cem-ssr-declaration";
pub const PAYLOAD_ATTRIBUTE: &str = "data-cem-ssr-payload";
pub const STYLESHEETS_ATTRIBUTE: &str = "data-cem-ssr-stylesheets";
pub const MAX_DECLARATION_BYTES: usize = 64 * 1024;
pub const MAX_STYLESHEET_BYTES: usize = 256 * 1024;
pub const MAX_STYLESHEET_TOTAL_BYTES: usize = 1024 * 1024;
pub const MAX_STYLESHEET_MANIFEST_BYTES: usize = 64 * 1024;

/// Only public binary URLs in the managed stylesheet profile need a read.
pub(crate) fn stylesheet_asset(
    reference: &cem_ml::css_resources::CssResourceReference,
) -> Result<Option<(&str, &'static str)>, &'static str> {
    use cem_ml::css_resources::CssResourceKind;
    if reference.kind != CssResourceKind::Url || reference.authored_specifier.starts_with('#') {
        return Ok(None);
    }
    let resolved = reference
        .resolution
        .as_ref()
        .map_err(|_| "CSS asset resolution refused")?;
    let uri = resolved.resolved_url.as_str();
    if uri.starts_with("data:") {
        return Ok(None);
    }
    let url = url::Url::parse(uri).map_err(|_| "Invalid CSS asset URL")?;
    if uri.len() > 4096 || url.query().is_some() || ssr_resource_url(uri).as_deref() != Ok(uri) {
        return Err("CSS asset reads require an exact public URL without a query or fragment");
    }
    let extension = url
        .path()
        .rsplit_once('.')
        .map(|(_, ext)| ext.to_ascii_lowercase());
    let mime = match extension.as_deref() {
        Some("png") => "image/png",
        Some("jpg" | "jpeg") => "image/jpeg",
        Some("gif") => "image/gif",
        Some("webp") => "image/webp",
        Some("avif") => "image/avif",
        Some("woff") => "font/woff",
        Some("woff2") => "font/woff2",
        Some("otf") => "font/otf",
        Some("ttf") => "font/ttf",
        _ => return Err("Unsupported CSS asset media type"),
    };
    if resolved
        .content_type_hint
        .as_deref()
        .is_some_and(|hint| cem_ml::schema::registry::content_type_essence(hint) != mime)
    {
        return Err("CSS asset media type disagrees with its resolver hint");
    }
    Ok(Some((uri, mime)))
}

#[derive(serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SourceReceipt {
    pub url: String,
    pub content_type: String,
    pub byte_length: usize,
    pub hash: String,
}

impl SourceReceipt {
    pub(crate) fn new(url: &str, content_type: &str, length: usize, hash: &ContentHash) -> Self {
        Self {
            url: url.into(),
            content_type: content_type.into(),
            byte_length: length,
            hash: hash.hex.clone(),
        }
    }

    pub fn verify(
        raw: &str,
        bytes: &[u8],
        content_type: &str,
        limit: usize,
    ) -> Result<Self, String> {
        let fail = || "cem.ssr.resume_resource_identity".to_owned();
        if raw.len() > 8192 || bytes.len() > limit {
            return Err(fail());
        }
        let receipt: Self = serde_json::from_str(raw).map_err(|_| fail())?;
        receipt.check_bytes(bytes, content_type, limit)?;
        Ok(receipt)
    }

    pub(crate) fn check_bytes(
        &self,
        bytes: &[u8],
        content_type: &str,
        limit: usize,
    ) -> Result<(), String> {
        if self.url.len() > 4096
            || ssr_resource_url(&self.url).as_deref() != Ok(self.url.as_str())
            || self.content_type != content_type
            || self.byte_length > limit
            || self.byte_length != bytes.len()
            || self.hash != ContentHash::from_blake3(bytes).hex
        {
            return Err("cem.ssr.resume_resource_identity".into());
        }
        Ok(())
    }
}

#[derive(serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct StylesheetReceipt {
    pub owner_key: String,
    pub context: String,
    pub sources: Vec<SourceReceipt>,
}

pub fn declaration_content_type(uri: &str) -> &'static str {
    use cem_ml::schema::registry::{HTML_CONTENT_TYPE, XHTML_CONTENT_TYPE};
    if uri
        .rsplit_once('.')
        .is_some_and(|(_, suffix)| suffix.eq_ignore_ascii_case("xhtml"))
    {
        XHTML_CONTENT_TYPE
    } else {
        HTML_CONTENT_TYPE
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn receipt_requires_exact_bytes_length_mime_and_canonical_url() {
        let bytes = b"{item | Expected}";
        let mut receipt = SourceReceipt::new(
            "https://ssr.test/payload.cem",
            "text/cem-ml",
            bytes.len(),
            &ContentHash::from_blake3(bytes),
        );
        let raw = |receipt: &SourceReceipt| serde_json::to_string(receipt).unwrap();
        assert!(SourceReceipt::verify(&raw(&receipt), bytes, "text/cem-ml", 64).is_ok());
        assert!(
            SourceReceipt::verify(&raw(&receipt), b"{item | Modified}", "text/cem-ml", 64).is_err()
        );
        assert!(SourceReceipt::verify(&raw(&receipt), bytes, "text/html", 64).is_err());
        assert!(SourceReceipt::verify(&raw(&receipt), bytes, "text/cem-ml", 8).is_err());
        receipt.url.push_str("#foreign");
        assert!(SourceReceipt::verify(&raw(&receipt), bytes, "text/cem-ml", 64).is_err());
    }
}
