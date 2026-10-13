//! Bounded HTML image-candidate parsing over a retained, decoded attribute.
//! https://html.spec.whatwg.org/multipage/images.html#parsing-a-srcset-attribute
use crate::{
    parser::{
        tree::{CemTreeNodeKind, RetainedCemTree},
        AstNodeId,
    },
    source_map::SourceMapStack,
};
use std::{ops::Range, sync::Arc};

#[derive(Clone, Copy, Debug)]
pub struct SrcsetLimits {
    pub max_bytes: usize,
    /// Counts attempted candidates, including discarded malformed candidates.
    pub max_candidates: usize,
}
impl Default for SrcsetLimits {
    fn default() -> Self {
        Self {
            max_bytes: 65_536,
            max_candidates: 4096,
        }
    }
}
#[derive(Clone, Debug, PartialEq)]
pub enum SrcsetDescriptor {
    None,
    Density(f64),
    Width {
        width: u32,
        future_height: Option<u32>,
    },
}
#[derive(Clone, Debug, PartialEq)]
pub struct SrcsetCandidate {
    pub url: String,
    /// UTF-8 byte range in the decoded attribute, never an original-file edit range.
    pub url_range: Range<usize>,
    pub descriptor_ranges: Vec<Range<usize>>,
    pub descriptor: SrcsetDescriptor,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SrcsetIssueKind {
    UnexpectedComma,
    InvalidDescriptor,
    FutureHeight,
    UnsupportedNumber,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SrcsetIssue {
    pub kind: SrcsetIssueKind,
    pub range: Range<usize>,
}
#[derive(Clone, Debug)]
pub struct SrcsetParse {
    pub candidates: Vec<SrcsetCandidate>,
    pub issues: Vec<SrcsetIssue>,
    /// False when valid numeric syntax exceeds this implementation's profile.
    pub complete: bool,
}
impl SrcsetParse {
    fn issue(&mut self, kind: SrcsetIssueKind, range: Range<usize>) {
        self.issues.push(SrcsetIssue { kind, range });
    }
}
fn space(b: u8) -> bool {
    matches!(b, b'\t' | b'\n' | b'\x0c' | b'\r' | b' ')
}
fn digits(s: &str) -> bool {
    !s.is_empty() && s.bytes().all(|b| b.is_ascii_digit())
}
// HTML's valid floating-point syntax excludes leading '+', trailing '.', NaN/Infinity.
fn float_syntax(s: &str) -> bool {
    let b = s.as_bytes();
    let mut i = usize::from(b.first() == Some(&b'-'));
    let start = i;
    while i < b.len() && b[i].is_ascii_digit() {
        i += 1;
    }
    let integral = i > start;
    if b.get(i) == Some(&b'.') {
        i += 1;
        let start = i;
        while i < b.len() && b[i].is_ascii_digit() {
            i += 1;
        }
        if start == i {
            return false;
        }
    } else if !integral {
        return false;
    }
    if matches!(b.get(i), Some(b'e' | b'E')) {
        i += 1;
        if matches!(b.get(i), Some(b'+' | b'-')) {
            i += 1;
        }
        let start = i;
        while i < b.len() && b[i].is_ascii_digit() {
            i += 1;
        }
        if start == i {
            return false;
        }
    }
    i == b.len()
}

/// Inventory potential sources only. Media/sizes/DPR selection and fetching are
/// separate operations. Limits fail atomically rather than publishing a prefix.
pub fn parse_srcset(value: &str, limits: SrcsetLimits) -> Result<SrcsetParse, String> {
    if value.len() > limits.max_bytes {
        return Err("srcset byte budget exceeded".into());
    }
    let b = value.as_bytes();
    let mut i = 0;
    let mut attempts = 0;
    let mut result = SrcsetParse {
        candidates: vec![],
        issues: vec![],
        complete: true,
    };
    while i < b.len() {
        let skipped = i;
        while i < b.len() && (space(b[i]) || b[i] == b',') {
            i += 1;
        }
        if b[skipped..i].contains(&b',') {
            result.issue(SrcsetIssueKind::UnexpectedComma, skipped..i);
        }
        if i == b.len() {
            break;
        }
        if attempts == limits.max_candidates {
            return Err("srcset candidate budget exceeded".into());
        }
        attempts += 1;
        let start = i;
        while i < b.len() && !space(b[i]) {
            i += 1;
        }
        let mut end = i;
        let mut ranges = Vec::new();
        if b[end - 1] == b',' {
            while end > start && b[end - 1] == b',' {
                end -= 1;
            }
            if i - end > 1 {
                result.issue(SrcsetIssueKind::UnexpectedComma, end..i);
            }
        } else {
            // Descriptor tokenizer: parentheses protect commas and whitespace.
            // The standard has a single in-parentheses state, not nesting depth.
            let mut token = None;
            let mut parens = false;
            while i < b.len() {
                let c = b[i];
                if !parens && (space(c) || c == b',') {
                    if let Some(start) = token.take() {
                        ranges.push(start..i);
                    }
                    i += 1;
                    if c == b',' {
                        break;
                    }
                    continue;
                }
                token.get_or_insert(i);
                if c == b'(' {
                    parens = true;
                }
                if c == b')' {
                    parens = false;
                }
                i += 1;
            }
            if let Some(start) = token {
                ranges.push(start..i);
            }
        }
        let (mut width, mut height, mut density) = (None, None, None);
        let mut valid = true;
        for range in &ranges {
            let d = &value[range.clone()];
            let (number, suffix) = d.split_at(d.len() - d.chars().last().unwrap().len_utf8());
            let kind = match suffix {
                "w" | "h" if digits(number) => {
                    if density.is_some()
                        || (suffix == "w" && width.is_some())
                        || (suffix == "h" && height.is_some())
                    {
                        Some(SrcsetIssueKind::InvalidDescriptor)
                    } else {
                        match number.parse::<u32>() {
                            Ok(0) => Some(SrcsetIssueKind::InvalidDescriptor),
                            Ok(n) => {
                                if suffix == "w" {
                                    width = Some(n);
                                } else {
                                    height = Some(n);
                                    result.issue(SrcsetIssueKind::FutureHeight, range.clone());
                                }
                                None
                            }
                            Err(_) => Some(SrcsetIssueKind::UnsupportedNumber),
                        }
                    }
                }
                "x" if float_syntax(number) => {
                    if width.is_some() || height.is_some() || density.is_some() {
                        Some(SrcsetIssueKind::InvalidDescriptor)
                    } else {
                        match number.parse::<f64>() {
                            Ok(n) if n < 0.0 => Some(SrcsetIssueKind::InvalidDescriptor),
                            Ok(n) if n.is_finite() => {
                                density = Some(n);
                                None
                            }
                            _ => Some(SrcsetIssueKind::UnsupportedNumber),
                        }
                    }
                }
                _ => Some(SrcsetIssueKind::InvalidDescriptor),
            };
            if let Some(kind) = kind {
                if kind == SrcsetIssueKind::UnsupportedNumber {
                    result.complete = false;
                }
                result.issue(kind, range.clone());
                valid = false;
            }
        }
        if height.is_some() && width.is_none() {
            result.issue(SrcsetIssueKind::InvalidDescriptor, start..i);
            valid = false;
        }
        if valid {
            let descriptor = if let Some(width) = width {
                SrcsetDescriptor::Width {
                    width,
                    future_height: height,
                }
            } else if let Some(density) = density {
                SrcsetDescriptor::Density(density)
            } else {
                SrcsetDescriptor::None
            };
            result.candidates.push(SrcsetCandidate {
                url: value[start..end].into(),
                url_range: start..end,
                descriptor_ranges: ranges,
                descriptor,
            });
        }
    }
    Ok(result)
}

#[derive(Clone)]
pub struct RetainedSrcsetAttribute {
    tree: Arc<RetainedCemTree>,
    owner: AstNodeId,
    attribute: AstNodeId,
    parsed: SrcsetParse,
}
impl RetainedSrcsetAttribute {
    pub fn tree(&self) -> &Arc<RetainedCemTree> {
        &self.tree
    }
    pub fn owner(&self) -> AstNodeId {
        self.owner
    }
    pub fn attribute(&self) -> AstNodeId {
        self.attribute
    }
    pub fn parsed(&self) -> &SrcsetParse {
        &self.parsed
    }
    pub fn value(&self) -> &str {
        &self.tree.node(self.attribute).unwrap().value
    }
    pub fn host_source(&self) -> &SourceMapStack {
        &self.tree.node(self.attribute).unwrap().source
    }
    pub fn source_key(&self) -> Option<String> {
        self.tree.source_key(self.attribute)
    }
    pub fn source_uri(&self) -> Option<&str> {
        self.tree.node_source_uri(self.attribute)
    }
}
pub fn srcset_slot(namespace: &str, local: &str, attribute_namespace: &str, field: &str) -> bool {
    namespace == "http://www.w3.org/1999/xhtml"
        && attribute_namespace.is_empty()
        && matches!(
            (local, field),
            ("img" | "source", "srcset") | ("link", "imagesrcset")
        )
}
pub fn import_srcset_attribute(
    tree: Arc<RetainedCemTree>,
    owner: AstNodeId,
    attribute: AstNodeId,
    limits: SrcsetLimits,
) -> Result<RetainedSrcsetAttribute, String> {
    let node = tree.node(owner).ok_or("missing srcset owner")?;
    let attr = tree.node(attribute).ok_or("missing srcset attribute")?;
    let name = node.name.as_ref().ok_or("missing srcset owner name")?;
    let field = attr.name.as_ref().ok_or("missing srcset attribute name")?;
    if node.kind != CemTreeNodeKind::Element
        || attr.kind != CemTreeNodeKind::Attribute
        || !node.attributes.contains(&attribute)
        || !srcset_slot(
            &name.namespace_uri,
            &name.local_name,
            &field.namespace_uri,
            &field.local_name,
        )
    {
        return Err("unsupported srcset owner/attribute slot".into());
    }
    let parsed = parse_srcset(&attr.value, limits)?;
    Ok(RetainedSrcsetAttribute {
        tree,
        owner,
        attribute,
        parsed,
    })
}
