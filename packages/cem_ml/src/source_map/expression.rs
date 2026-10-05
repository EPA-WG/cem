//! Format-neutral mapping from retained expression bytes to authored spans.
use super::{FrameSpan, SourceMapFrame, SourceMapStack, TransformKind};
use crate::source::{ByteRange, SourceId};

fn overlap(expression: ByteRange, query: ByteRange) -> Option<ByteRange> {
    let start = expression.start.max(query.start);
    let end = expression.end().min(query.end());
    (start < end
        || (query.len == 0 && query.start >= expression.start && query.start < expression.end()))
    .then(|| ByteRange::new(start, end.saturating_sub(start) as u32))
}
fn authored_segment(expression: ByteRange, source: ByteRange, selected: ByteRange) -> ByteRange {
    if expression.len == source.len {
        ByteRange::new(
            source.start + selected.start - expression.start,
            selected.len,
        )
    } else {
        source
    }
}

/// Preserve discontiguous source segments and full entity/newline provenance.
/// Unknown mappings return an empty list rather than guessing a document offset.
pub fn map_expression_range(
    stack: &SourceMapStack,
    query: ByteRange,
) -> Vec<(SourceId, ByteRange)> {
    let mut result = vec![];
    for frame in &stack.frames {
        let (TransformKind::ExpressionEmbedding { expression }, FrameSpan::Single(source)) =
            (&frame.transform, &frame.span)
        else {
            continue;
        };
        if let Some(selected) = overlap(*expression, query) {
            result.push((
                frame.source_id,
                authored_segment(*expression, *source, selected),
            ));
        }
    }
    // An EOF point belongs to the end of the last mapped payload, not to a
    // following XML delimiter. Prefer a following segment at internal boundaries.
    if result.is_empty() && query.len == 0 {
        if let Some(frame) = stack.frames.iter().rev().find(|frame| matches!(&frame.transform, TransformKind::ExpressionEmbedding { expression } if expression.end() == query.start)) {
            if let FrameSpan::Single(source) = frame.span {
                result.push((frame.source_id, ByteRange::new(source.end(), 0)));
            }
        }
    }
    result
}

/// Shift import-produced segments after trimming decoded expression whitespace.
pub fn trim_expression_frames(
    frames: Vec<SourceMapFrame>,
    leading: u64,
    length: u32,
) -> Vec<SourceMapFrame> {
    let query = ByteRange::new(leading, length);
    frames
        .into_iter()
        .filter_map(|frame| {
            let (TransformKind::ExpressionEmbedding { expression }, FrameSpan::Single(source)) =
                (&frame.transform, &frame.span)
            else {
                return None;
            };
            let selected = overlap(*expression, query)?;
            Some(SourceMapFrame {
                source_id: frame.source_id,
                span: FrameSpan::Single(authored_segment(*expression, *source, selected)),
                transform: TransformKind::ExpressionEmbedding {
                    expression: ByteRange::new(selected.start - leading, selected.len),
                },
            })
        })
        .collect()
}
