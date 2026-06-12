//! Layer 7 — `BinaryAstEncoder` interface + Tier A debug body.
//!
//! Per AC-F-11 the *production* binary AST encoder body is Tier B. Tier A
//! ships:
//!
//! 1. The public boundary types (`BinaryAstPayload`, `ChunkMetadata`,
//!    `BinaryAstEncoder` trait) so consumers can program against the
//!    boundary today.
//! 2. A test-only `DebugBinaryEncoder` / `DebugBinaryDecoder` that round-trips
//!    a `CemDocument` through a deterministic uncompressed byte format. The
//!    encoder is exposed publicly (it's the only way to exercise the round
//!    trip in tests) but it is *not* a compatibility-stable format and is
//!    explicitly marked `#[doc(hidden)]`.
//!
//! Format sketch (see `format.rs` for the canonical declaration):
//!
//! ```text
//! Header:        magic b"CEMB" + u16 version + u16 flags
//! Dictionaries:  strings, source_ids, transform_kinds, source_map_frames
//! Chunks:        single root chunk + child chunks, each with chunk_metadata
//! Nodes:         flat table indexed by AstNodeId, kind tag + dict refs
//! Edges:         parent → ordered children list
//! Side tables:   id_table, unresolved_slots
//! Integrity:     FNV-1a u64 over every preceding byte
//! ```

pub mod decode;
pub mod encode;
pub mod format;

pub use decode::DebugBinaryDecoder;
pub use encode::DebugBinaryEncoder;
pub use format::{BinaryAstPayload, ChunkMetadata, MAGIC, VERSION};

use crate::parser::CemAstNode;

/// Boundary for the deferred Tier B binary AST encoder. The Tier A debug
/// encoder ([`DebugBinaryEncoder`]) is the only implementation today.
#[doc(hidden)]
pub trait BinaryAstEncoder: Send {
    fn encode(&self, nodes: &[CemAstNode]) -> BinaryAstPayload;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::events::cem::CemEventNormalizer;
    use crate::parser::builder::CemAstBuilder;
    use crate::parser::document::CemDocument;
    use crate::source::{BytesSource, SourceId};
    use crate::tokenizer::cem::CemTokenizer;

    fn parse(input: &str) -> CemDocument {
        let src = BytesSource::new(SourceId(1), input.as_bytes().to_vec());
        let tok = CemTokenizer::from_source(src);
        let normalizer = CemEventNormalizer::new(tok);
        CemAstBuilder::new(normalizer).build()
    }

    fn round_trip(doc: &CemDocument) -> (BinaryAstPayload, CemDocument) {
        let encoder = DebugBinaryEncoder::new();
        let payload = encoder.encode(doc);
        let decoded = DebugBinaryDecoder::new()
            .decode(&payload.bytes)
            .expect("decoded ok");
        (payload, decoded)
    }

    fn assert_documents_equivalent(a: &CemDocument, b: &CemDocument) {
        assert_eq!(a.nodes.len(), b.nodes.len(), "node count must match");
        for (left, right) in a.nodes.iter().zip(b.nodes.iter()) {
            assert_eq!(format!("{left:?}"), format!("{right:?}"), "node mismatch");
        }
        assert_eq!(a.id_table, b.id_table);
        assert_eq!(a.unresolved_slots.len(), b.unresolved_slots.len());
    }

    #[test]
    fn payload_starts_with_magic_and_version() {
        let doc = parse("{p Hello}");
        let (payload, _) = round_trip(&doc);
        assert!(payload.bytes.starts_with(&MAGIC));
        let version = u16::from_le_bytes([payload.bytes[4], payload.bytes[5]]);
        assert_eq!(version, VERSION);
    }

    #[test]
    fn empty_input_round_trips() {
        let doc = parse("");
        let (_, decoded) = round_trip(&doc);
        assert_documents_equivalent(&doc, &decoded);
    }

    #[test]
    fn simple_element_round_trips() {
        let doc = parse("{p Hello}");
        let (_, decoded) = round_trip(&doc);
        assert_documents_equivalent(&doc, &decoded);
    }

    #[test]
    fn nested_attributes_round_trip() {
        let doc = parse(
            r#"{form @method=post | {label @for=email | Email} {input @id=email @required}}"#,
        );
        let (_, decoded) = round_trip(&doc);
        assert_documents_equivalent(&doc, &decoded);
        // id_table content survives the round trip.
        assert!(decoded.id_table.contains_key("email"));
    }

    #[test]
    fn encoder_is_deterministic() {
        let doc = parse("{button @cem:action=primary | Save}");
        let a = DebugBinaryEncoder::new().encode(&doc).bytes;
        let b = DebugBinaryEncoder::new().encode(&doc).bytes;
        assert_eq!(a, b, "encoder must produce identical bytes for same input");
    }

    #[test]
    fn integrity_hash_protects_against_tampering() {
        let doc = parse("{p Hi}");
        let mut bytes = DebugBinaryEncoder::new().encode(&doc).bytes;
        // Flip a byte in the middle of the payload.
        let idx = bytes.len() / 2;
        bytes[idx] ^= 0xFF;
        let err = DebugBinaryDecoder::new().decode(&bytes).unwrap_err();
        assert!(matches!(
            err,
            crate::ast::decode::DecodeError::IntegrityMismatch { .. }
        ));
    }

    #[test]
    fn chunk_metadata_records_node_count_and_hash() {
        let doc = parse("{a | {b | {c | x}}}");
        let payload = DebugBinaryEncoder::new().encode(&doc);
        assert_eq!(payload.chunks.len(), 1);
        let chunk = &payload.chunks[0];
        assert_eq!(chunk.root_id, 0);
        assert!(chunk.parent_anchor.is_none());
        assert_eq!(chunk.local_node_start, 0);
        assert_eq!(chunk.local_node_count, doc.nodes.len() as u32);
        assert!(chunk.source_map_deltas.is_empty());
        assert!(chunk.child_links.is_empty());
        // Re-encode → same hash (determinism implies stable hash).
        let payload2 = DebugBinaryEncoder::new().encode(&doc);
        assert_eq!(chunk.integrity_hash, payload2.chunks[0].integrity_hash);
    }

    #[test]
    fn every_canonical_fixture_round_trips() {
        let dir =
            std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../examples/cem-ml");
        let mut checked = 0;
        for entry in std::fs::read_dir(&dir).unwrap() {
            let path = entry.unwrap().path();
            if path.extension().and_then(|e| e.to_str()) != Some("cem") {
                continue;
            }
            let input = std::fs::read_to_string(&path).unwrap();
            let doc = parse(&input);
            let (payload, decoded) = round_trip(&doc);
            assert_documents_equivalent(&doc, &decoded);
            // Determinism check per fixture: re-encode the decoded
            // document and confirm byte-identical payload.
            let reencoded = DebugBinaryEncoder::new().encode(&decoded).bytes;
            assert_eq!(
                payload.bytes,
                reencoded,
                "fixture `{}` round-trip is not byte-stable",
                path.display()
            );
            checked += 1;
        }
        assert!(checked >= 5);
    }
}
