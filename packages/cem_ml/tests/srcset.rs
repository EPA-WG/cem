use cem_ml::import::srcset::*;
use cem_ml::{document_dependencies::inspect_document_dependencies, parser::tree::RetainedCemTree};
use std::sync::Arc;
fn tree(source: &str) -> Arc<RetainedCemTree> {
    cem_ml::import::import_data_bytes(
        source.as_bytes(),
        "application/xml",
        "cem",
        "studio://p/source.xhtml",
    )
    .unwrap()
}
#[test]
fn parser_preserves_url_commas_ascii_whitespace_and_decoded_byte_ranges() {
    let input = " , data:image/svg+xml,%3Csvg%3E 1x, a,b.svg 2x, café.svg, \u{a0}other.svg .5x";
    let report = parse_srcset(input, SrcsetLimits::default()).unwrap();
    assert!(report.complete);
    assert_eq!(
        report
            .candidates
            .iter()
            .map(|c| &*c.url)
            .collect::<Vec<_>>(),
        [
            "data:image/svg+xml,%3Csvg%3E",
            "a,b.svg",
            "café.svg",
            "\u{a0}other.svg"
        ]
    );
    assert_eq!(
        report.candidates[0].descriptor,
        SrcsetDescriptor::Density(1.0)
    );
    assert_eq!(
        &input[report.candidates[0].descriptor_ranges[0].clone()],
        "1x"
    );
    assert_eq!(report.candidates[2].descriptor, SrcsetDescriptor::None);
    assert_eq!(
        report.candidates[3].descriptor,
        SrcsetDescriptor::Density(0.5)
    );
    for candidate in &report.candidates {
        assert_eq!(&input[candidate.url_range.clone()], candidate.url);
    }
    assert!(!report.issues.is_empty());
    assert_eq!(
        parse_srcset("a,b", Default::default()).unwrap().candidates[0].url,
        "a,b"
    );
    assert!(parse_srcset(" \t\n\r\u{c}", Default::default())
        .unwrap()
        .candidates
        .is_empty());
}
#[test]
fn parser_validates_descriptors_and_recovers_at_the_correct_delimiter() {
    let report = parse_srcset("bad.svg 2x 3x, bad2.svg foo(a,b), width.svg 40w 20h, good.svg 2e0x, zero.svg 0x, tail.svg,,", Default::default()).unwrap();
    assert!(report.complete);
    assert_eq!(
        report
            .candidates
            .iter()
            .map(|c| &*c.url)
            .collect::<Vec<_>>(),
        ["width.svg", "good.svg", "zero.svg", "tail.svg"]
    );
    assert_eq!(
        report.candidates[0].descriptor,
        SrcsetDescriptor::Width {
            width: 40,
            future_height: Some(20)
        }
    );
    for descriptor in [
        "0w",
        "-1w",
        "+1x",
        "1.x",
        "1e+x",
        "-2x",
        "1h",
        "1w 1x",
        "1h 2h 3w",
        "NaNx",
        "Infinityx",
    ] {
        let input = format!("bad {descriptor}, good 2x");
        let parsed = parse_srcset(&input, Default::default()).unwrap();
        assert_eq!(
            parsed.candidates.len(),
            1,
            "{input}: {:?}",
            parsed.candidates
        );
        assert_eq!(parsed.candidates[0].url, "good");
        assert!(!parsed.issues.is_empty());
    }
}
#[test]
fn parser_bounds_bytes_candidate_attempts_and_unsupported_numbers() {
    assert!(parse_srcset(
        "long.svg",
        SrcsetLimits {
            max_bytes: 3,
            ..Default::default()
        }
    )
    .is_err());
    assert!(parse_srcset(
        "bad 0w, also-bad -1x, good 2x",
        SrcsetLimits {
            max_candidates: 2,
            ..Default::default()
        }
    )
    .is_err());
    for input in ["huge 999999999999999999999w", "huge 1e999x"] {
        assert!(!parse_srcset(input, Default::default()).unwrap().complete);
    }
}
#[test]
fn retained_import_preserves_entity_decoding_owners_and_slot_guards() {
    let tree = tree(
        r#"<html xmlns="http://www.w3.org/1999/xhtml"><img srcset="café.svg?x=1&amp;y=2 1x, b.svg 2x"/><p srcset="unknown.svg"/><g xmlns="urn:other" srcset="foreign.svg"/></html>"#,
    );
    let report = inspect_document_dependencies(tree.clone(), 500);
    assert_eq!(report.srcsets.len(), 1);
    let srcset = &report.srcsets[0];
    assert!(Arc::ptr_eq(srcset.tree(), &tree));
    let candidate = &srcset.parsed().candidates[0];
    assert_eq!(candidate.url, "café.svg?x=1&y=2");
    assert_eq!(&srcset.value()[candidate.url_range.clone()], candidate.url);
    assert!(!srcset.host_source().frames.is_empty());
    assert!(srcset.source_key().is_some());
    assert!(
        import_srcset_attribute(tree.clone(), 0, srcset.attribute(), Default::default()).is_err()
    );
    assert!(report
        .blockers
        .iter()
        .any(|b| b == "srcset-analysis-required")); // unsupported p slot
}
#[test]
fn retained_inventory_covers_img_source_and_preload_without_splitting_direct_references() {
    let report = inspect_document_dependencies(
        tree(
            r#"<html xmlns="http://www.w3.org/1999/xhtml"><head><link rel="preload" as="image" imagesrcset="a.svg 1x, b.svg 2x"/></head><body><picture><source srcset="c.svg 320w"/><img src="fallback.svg" srcset="d.svg 1x"/></picture></body></html>"#,
        ),
        500,
    );
    assert_eq!(report.srcsets.len(), 3);
    assert_eq!(report.references.len(), 1);
    assert!(!report
        .blockers
        .iter()
        .any(|b| b == "srcset-analysis-required"));
    assert!(report
        .blockers
        .iter()
        .any(|b| b == "srcset-selection-and-load-history-required"));
    let bounded = inspect_document_dependencies(
        tree(r#"<img xmlns="http://www.w3.org/1999/xhtml" srcset="a.svg 1x, b.svg 2x"/>"#),
        8,
    );
    assert!(bounded
        .blockers
        .iter()
        .any(|b| b == "inspection-budget-exceeded"));
    assert!(bounded.srcsets.is_empty());
}
