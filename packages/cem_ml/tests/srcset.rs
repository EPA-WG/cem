use cem_ml::import::srcset::*;
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
