//! Native writer lineage bound to an admitted input, never recovered from debug JSON.
use crate::{
    interpreter::OutputSpan,
    source_map::{FrameSpan, SourceMapFrame, SourceMapStack, TransformKind},
};

/// Native-only witness. Source bytes and runtime handles are not serialized into
/// command results or checkpoints. The digests bind the immutable original input and final writer bytes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PrimaryProvenance {
    source_uri: String,
    source_length: usize,
    source_hash: blake3::Hash,
    output_hash: blake3::Hash,
    output_spans: Vec<OutputSpan>,
}
impl PrimaryProvenance {
    pub(crate) fn from_writer(
        uri: &str,
        source: &[u8],
        output: &[u8],
        output_spans: Vec<OutputSpan>,
    ) -> Self {
        Self {
            source_uri: uri.into(),
            source_length: source.len(),
            source_hash: blake3::hash(source),
            output_hash: blake3::hash(output),
            output_spans,
        }
    }
    pub fn output_spans(&self) -> &[OutputSpan] {
        &self.output_spans
    }
    pub(crate) fn source_uri(&self) -> &str {
        &self.source_uri
    }

    /// Admit only explicit native boundaries belonging to this original buffer.
    /// Anonymous IDs, formatter origins and serialized user fields cannot supply
    /// an endpoint. Keep each output span's actual native range without inventing
    /// linear/character equality through formatting or escaping.
    pub(crate) fn admitted_spans(&self, source: &[u8], output: &[u8]) -> Vec<OutputSpan> {
        if source.len() != self.source_length
            || blake3::hash(source) != self.source_hash
            || blake3::hash(output) != self.output_hash
        {
            return Vec::new();
        }
        let (Ok(text), Ok(output_text)) =
            (std::str::from_utf8(source), std::str::from_utf8(output))
        else {
            return Vec::new();
        };
        let valid = |frame: &SourceMapFrame| {
            if !matches!(&frame.transform, TransformKind::SourceBoundary {uri} if uri == &self.source_uri)
            {
                return false;
            }
            let FrameSpan::Single(range) = frame.span else {
                return false;
            };
            let Some(end) = range.start.checked_add(u64::from(range.len)) else {
                return false;
            };
            end <= source.len() as u64
                && text.is_char_boundary(range.start as usize)
                && text.is_char_boundary(end as usize)
        };
        self.output_spans
            .iter()
            .filter_map(|span| {
                let end = span
                    .output_range
                    .start
                    .checked_add(u64::from(span.output_range.len))?;
                if span.output_range.len == 0
                    || end > output.len() as u64
                    || !output_text.is_char_boundary(span.output_range.start as usize)
                    || !output_text.is_char_boundary(end as usize)
                {
                    return None;
                }
                let frames = span
                    .origin
                    .frames
                    .iter()
                    .filter(|frame| valid(frame))
                    .cloned()
                    .collect::<Vec<_>>();
                (!frames.is_empty()).then(|| OutputSpan {
                    output_range: span.output_range,
                    origin: SourceMapStack { frames },
                })
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::source::{ByteRange, SourceId};

    fn span(uri: &str, start: u64, len: u32) -> OutputSpan {
        OutputSpan {
            output_range: ByteRange::new(2, 3),
            origin: SourceMapStack {
                frames: vec![SourceMapFrame {
                    source_id: SourceId(7),
                    span: FrameSpan::Single(ByteRange::new(start, len)),
                    transform: TransformKind::SourceBoundary { uri: uri.into() },
                }],
            },
        }
    }

    #[test]
    fn original_buffer_and_explicit_native_boundary_are_required() {
        let text = "aé\r\nz".as_bytes();
        let mut anonymous = span("studio://original", 1, 2);
        anonymous.origin.frames[0].transform = TransformKind::CemAstBuilder;
        let proof = PrimaryProvenance::from_writer(
            "studio://original",
            text,
            b"outxx",
            vec![
                span("studio://original", 1, 2),
                span("studio://foreign", 1, 2),
                anonymous,
                span("studio://original", 2, 1),
                span("studio://original", 5, 2),
                span("studio://original", u64::MAX, 2),
            ],
        );
        let opaque = PrimaryProvenance::from_writer(
            "studio://original",
            &[255],
            b"outxx",
            vec![span("studio://original", 0, 1)],
        );
        assert!(
            opaque.admitted_spans(&[255], b"outxx").is_empty(),
            "binary buffers gain no text navigation grant"
        );
        let split_output = PrimaryProvenance::from_writer(
            "studio://original",
            text,
            "xéxx".as_bytes(),
            vec![span("studio://original", 1, 2)],
        );
        assert!(
            split_output
                .admitted_spans(text, "xéxx".as_bytes())
                .is_empty(),
            "writer spans cannot split a UTF-8 scalar"
        );
        let admitted = proof.admitted_spans(text, b"outxx");
        assert_eq!(admitted.len(), 1);
        assert_eq!(
            admitted[0].origin.frames[0].span,
            FrameSpan::Single(ByteRange::new(1, 2))
        );
        assert!(
            proof.admitted_spans(b"abcdef", b"outxx").is_empty(),
            "same length and URI cannot replace original bytes"
        );
        assert!(
            proof.admitted_spans(text, b"outx").is_empty(),
            "writer output ranges must fit the published bytes"
        );
        assert!(
            proof.admitted_spans(text, b"other").is_empty(),
            "equal length output cannot replace the original writer bytes"
        );
        assert_eq!(
            proof.output_spans().len(),
            6,
            "raw native evidence remains unchanged"
        );
    }
}
