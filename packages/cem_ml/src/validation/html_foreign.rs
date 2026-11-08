use super::{HtmlFactKind, MATHML_NAMESPACE_URI, SVG_NAMESPACE_URI};
/// Bounded passive foreign-DOM profile. Namespace/case remains semantic input;
/// this never promotes a foreign node to HTML or authorizes resource loading.
pub fn validate_passive_foreign_projection(
    namespace: &str,
    name: &str,
    attributes: &[(&str, &str, &str)],
    has_markup_children: bool,
) -> Result<(), HtmlFactKind> {
    let admitted = match namespace {
        SVG_NAMESPACE_URI => matches!(
            name,
            "svg"
                | "g"
                | "defs"
                | "title"
                | "desc"
                | "path"
                | "circle"
                | "ellipse"
                | "line"
                | "polyline"
                | "polygon"
                | "rect"
                | "text"
                | "tspan"
                | "textPath"
                | "use"
                | "symbol"
                | "linearGradient"
                | "radialGradient"
                | "stop"
                | "clipPath"
                | "mask"
                | "pattern"
                | "marker"
        ),
        MATHML_NAMESPACE_URI => matches!(
            name,
            "math"
                | "mrow"
                | "mi"
                | "mn"
                | "mo"
                | "mtext"
                | "ms"
                | "mspace"
                | "mfrac"
                | "msqrt"
                | "mroot"
                | "msub"
                | "msup"
                | "msubsup"
                | "munder"
                | "mover"
                | "munderover"
                | "mmultiscripts"
                | "mprescripts"
                | "none"
                | "mtable"
                | "mtr"
                | "mtd"
                | "mstyle"
                | "merror"
                | "mpadded"
                | "mphantom"
                | "menclose"
                | "semantics"
        ),
        _ => false,
    };
    // HTML foreign text integration points change child token processing.
    // Until that source profile is admitted, retain only markup-free text/comments.
    // This also refuses markup-shaped RCDATA from the current HTML tokenizer.
    let text_container = namespace == SVG_NAMESPACE_URI && matches!(name, "title" | "desc")
        || namespace == MATHML_NAMESPACE_URI && matches!(name, "mi" | "mo" | "mn" | "ms" | "mtext");
    if !admitted || text_container && has_markup_children {
        return Err(HtmlFactKind::ForeignContentUnregistered);
    }
    for &(ns, attr, value) in attributes {
        if !matches!(
            ns,
            "" | "http://www.w3.org/XML/1998/namespace"
                | "http://www.w3.org/1999/xlink"
                | "http://www.w3.org/2000/xmlns/"
        ) || attr.is_empty()
            || !attr
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_' | b'.'))
            || ns == "http://www.w3.org/1999/xlink" && attr != "href"
            || ns == "http://www.w3.org/XML/1998/namespace" && !matches!(attr, "lang" | "space")
        {
            return Err(HtmlFactKind::ForeignContentUnregistered);
        }
        let lower = attr.to_ascii_lowercase();
        if lower.starts_with("on") || matches!(lower.as_str(), "is" | "srcdoc") {
            return Err(HtmlFactKind::EventHandlerRejected);
        }
        // Foreign presentation attributes can also contain URL references. The
        // conservative profile allows local fragments only in URI-valued attrs.
        if matches!(lower.as_str(), "href" | "src") && !(value.is_empty() || value.starts_with('#'))
            || value.to_ascii_lowercase().contains("url(")
            || lower == "style"
            || matches!(
                lower.as_str(),
                "fill"
                    | "stroke"
                    | "filter"
                    | "clip-path"
                    | "mask"
                    | "marker"
                    | "marker-start"
                    | "marker-mid"
                    | "marker-end"
                    | "cursor"
            ) && (value.contains('(') || value.contains('\\'))
        {
            return Err(HtmlFactKind::ExternalResourceRejected);
        }
    }
    Ok(())
}
