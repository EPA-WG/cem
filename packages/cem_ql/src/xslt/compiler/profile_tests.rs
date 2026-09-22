//! Profile the actual compiler, retaining all validation and source maps.
use super::*;
use crate::{
    compile_profile::{measure, Stages},
    eval::{AtomValue, Item, ItemStream},
    render::{render_plan_to_html, TemplateData},
};
use std::{hint::black_box, time::Instant};

const BASE: &str = include_str!("../../../../cem-elements/demo/data-table-view.xslt");
const ASPECTS: &str = include_str!("../../../../cem-elements/demo/data-table-aspects.xslt");

fn input() -> TemplateData {
    let mut input = TemplateData::default();
    for (name, value) in [
        (
            "source",
            "<r><row qty='10'>🍒</row><row qty='2'>🍋</row><row qty='3'>🍌</row></r>",
        ),
        ("format", "xml"),
        ("column", "@qty"),
        ("mode", "number"),
        ("direction", "ascending"),
    ] {
        input.bindings.insert(
            name.into(),
            ItemStream::once(Item::Atomic(AtomValue::String(value.into()))),
        );
    }
    input
}

fn report(case: &str, samples: &[Stages]) {
    for (name, first) in &samples[0] {
        let mut times: Vec<_> = samples[1..]
            .iter()
            .map(|sample| {
                assert_eq!(sample[name].calls, first.calls, "{case}/{name}");
                sample[name].elapsed.as_secs_f64() * 1000.0
            })
            .collect();
        times.sort_by(f64::total_cmp);
        println!(
            "{case}\t{name}\tcalls={}\tfirst_ms={:.3}\tmedian_ms={:.3}\tmin_ms={:.3}\tmax_ms={:.3}",
            first.calls,
            first.elapsed.as_secs_f64() * 1000.0,
            times[2],
            times[0],
            times[4]
        );
    }
}

#[test]
#[ignore = "profiling fixture: --release --lib profile_xslt_compilation_stages -- --ignored --nocapture --test-threads=1"]
fn profile_xslt_compilation_stages() {
    // Every iteration compiles a fresh bundle. Only process-level schema caches
    // warm up. Nested stage totals are inclusive, not additional wall time.
    for (case, source, entry, programs) in [
        ("base", BASE, "viewer", 104),
        ("aspects", ASPECTS, "viewer-aspects", 127),
    ] {
        let uri = format!("memory:profile-{case}.xslt");
        let mut names = vec![
            "source",
            "initial",
            "format",
            "column",
            "direction",
            "mode",
            "selected",
        ];
        let modules = if case == "aspects" {
            names.extend(["aspects", "ipAddress", "ipAction"]);
            vec![XsltModuleSource {
                parent_uri: uri.clone(),
                href: "./data-table-view.xslt".into(),
                uri: "memory:profile-base.xslt".into(),
                source: BASE.into(),
                content_hash: ContentHash::from_blake3(BASE.as_bytes()),
            }]
        } else {
            vec![]
        };
        let options = XsltCompileOptions {
            entrypoint: Some(XPathExpandedName::unqualified(entry)),
            parameters: names
                .iter()
                .map(|name| (XPathExpandedName::unqualified(*name), (*name).into()))
                .collect(),
            modules,
        };
        let mut samples = Vec::new();
        let mut expected = None;
        for _ in 0..6 {
            let (compiled, stages) = measure(|| {
                let _total = crate::compile_profile::Span::new("total/compile-bundle");
                black_box(compile_xslt_bundle_with_options(source, &uri, &options).unwrap())
            });
            assert_eq!(stages["xpath/adapt-and-compile"].calls, programs);
            if let Some(bytes) = &expected {
                assert_eq!(&compiled.bytes, bytes);
            }
            expected = Some(compiled.bytes.clone());
            samples.push(stages);
        }
        report(case, &samples);
        // Compile without the recorder, then compare the complete portable
        // bytes: identity, source maps, diagnostics, host bindings and code.
        let start = Instant::now();
        let compiled = compile_xslt_bundle_with_options(source, &uri, &options).unwrap();
        println!(
            "{case}\tunprofiled_compile_ms={:.3}",
            start.elapsed().as_secs_f64() * 1000.0
        );
        assert_eq!(compiled.bytes, expected.unwrap());
        // Same binary, same inputs: compare against the unchanged standalone
        // checker path, including complete portable bundle bytes.
        let mut fresh_times = Vec::new();
        let mut prepared_times = Vec::new();
        for _ in 0..6 {
            let start = Instant::now();
            let prepared = compile_xslt_bundle_with_options(source, &uri, &options).unwrap();
            prepared_times.push(start.elapsed().as_secs_f64() * 1000.0);
            assert_eq!(compiled.bytes, prepared.bytes);
            let start = Instant::now();
            let fresh = crate::api::prepared_tests::without_prepared(|| {
                compile_xslt_bundle_with_options(source, &uri, &options).unwrap()
            });
            fresh_times.push(start.elapsed().as_secs_f64() * 1000.0);
            assert_eq!(compiled.bytes, fresh.bytes);
        }
        let first = fresh_times.remove(0);
        fresh_times.sort_by(f64::total_cmp);
        let prepared_first = prepared_times.remove(0);
        prepared_times.sort_by(f64::total_cmp);
        println!(
            "{case}\tfresh_checkers_first_ms={first:.3}\tfresh_checkers_median_ms={:.3}\tmin_ms={:.3}\tmax_ms={:.3}",
            fresh_times[2], fresh_times[0], fresh_times[4]
        );
        println!(
            "{case}\tprepared_first_ms={prepared_first:.3}\tprepared_median_ms={:.3}\tmin_ms={:.3}\tmax_ms={:.3}",
            prepared_times[2], prepared_times[0], prepared_times[4]
        );
        let start = Instant::now();
        let bundle = XsltBundle::from_bytes(
            &compiled.bytes,
            &compiled.content_hash,
            &compiled.source_hash,
        )
        .unwrap();
        println!(
            "{case}\treload_ms={:.3}\tbundle_bytes={}\tgenerated_cemt_bytes={}\txpath_programs={}",
            start.elapsed().as_secs_f64() * 1000.0,
            compiled.bytes.len(),
            compiled.generated_cemt.len(),
            bundle.expressions().len()
        );
        assert_eq!(bundle.expressions().len(), programs);
        let mut input = input();
        for binding in bundle.host_bindings() {
            input.bindings.entry(binding.clone()).or_default();
        }
        let output = bundle.render(&input);
        assert!(output.diagnostics.is_empty(), "{:?}", output.diagnostics);
        let html = render_plan_to_html(&output);
        let body = html.split_once("<tbody>").unwrap().1;
        assert!(body.find('🍋').unwrap() < body.find('🍌').unwrap());
        assert!(body.find('🍌').unwrap() < body.find('🍒').unwrap());
        let reloaded = XsltBundle::from_bytes(
            &compiled.bytes,
            &compiled.content_hash,
            &compiled.source_hash,
        )
        .unwrap();
        let repeated = reloaded.render(&input);
        assert!(repeated.diagnostics.is_empty());
        assert_eq!(html, render_plan_to_html(&repeated));
    }
}
