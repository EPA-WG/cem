//! Portable origin strings without a host blob-URL store.
//!
//! Opaque origins serialize as `null`; these strings are not origin identities.
//! This core does not register query functions or perform resource resolution.
use url::Url;

/// Serialize the accepted pure origin profile without modifying the URL.
///
/// A blob URL may derive an origin only from one HTTP/HTTPS inner URL. In
/// particular, do not delegate a blob directly to `url::Url::origin`, which
/// recursively accepts nested blobs and FTP/WS/WSS inner origins in 2.5.8.
pub fn serialize_origin(url: &Url) -> String {
    match url.scheme() {
        "http" | "https" | "ftp" | "ws" | "wss" => url.origin().ascii_serialization(),
        "blob" => match Url::parse(url.path()) {
            Ok(inner) if matches!(inner.scheme(), "http" | "https") => {
                inner.origin().ascii_serialization()
            }
            _ => "null".to_owned(),
        },
        _ => "null".to_owned(),
    }
}
