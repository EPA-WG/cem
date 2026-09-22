//! Test-only type-surface candidates. Production seeding remains unchanged.
use super::*;
use crate::stdlib::{ModuleRegistry, StdlibFunction};
use std::{hint::black_box, time::Instant};

fn register(checker: &mut TypeChecker, name: QNameKey, function: &StdlibFunction) {
    if function.module == "cem:stdlib/modules" {
        for arity in function.min_arity..=function.max_arity {
            checker.register_function(FunctionSignature {
                name: name.clone(),
                params: vec![Type::Any; arity as usize],
                ret: Type::atom(AtomType::AnyUri),
            });
        }
    } else {
        checker.register_any_function(name, function.min_arity, function.max_arity);
    }
}

fn alias(checker: &mut TypeChecker, registry: &ModuleRegistry, alias: &str, module: &str) {
    checker.imported_prefixes.insert(alias.into());
    for function in registry
        .functions
        .iter()
        .filter(|function| function.module == module)
    {
        register(
            checker,
            QNameKey::new(Some(alias.into()), function.name),
            function,
        );
    }
}

fn seed_candidate(checker: &mut TypeChecker, module: &SurfaceModule) {
    let registry = ModuleRegistry::with_all_known();
    // Deliberately mirrors the existing surface. Complete-map equality below
    // detects drift; this is not a second production alias registry.
    for (prefix, name) in [
        ("seq", "sequence"),
        ("str", "strings"),
        ("num", "numbers"),
        ("dt", "datetime"),
        ("dom", "dom"),
        ("cemt", "cemt"),
        ("item", "items"),
        ("module", "modules"),
        ("record", "records"),
        ("report", "report"),
        ("state", "state"),
        ("tpl", "template"),
        ("cemml", "cemml"),
        ("data", "data"),
        ("native", "native"),
        ("ct", "content-types"),
        ("user", "user"),
    ] {
        alias(checker, &registry, prefix, &format!("cem:stdlib/{name}"));
    }
    checker.register_function(FunctionSignature {
        name: QNameKey::new(None, "same_node"),
        params: vec![Type::Any, Type::Any],
        ret: boolean_type(),
    });
    for function in &registry.functions {
        if function.module == "cem:stdlib/sequence"
            || (function.module == "cem:stdlib/content-types" && function.name == "read")
            || (function.module == "cem:stdlib/modules" && function.name == "module_url")
        {
            register(checker, QNameKey::new(None, function.name), function);
        }
    }
    for node in &module.nodes {
        if let SurfaceNode::Import(import) = node {
            if let Some(prefix) = &import.alias {
                alias(checker, &registry, prefix, &import.uri);
            }
        }
    }
}

#[test]
#[ignore = "profiling fixture: --release --lib profile_local_type_registry -- --ignored --nocapture --test-threads=1"]
fn profile_local_type_registry() {
    let empty = crate::api::parse("1").module;
    let mut builtins = TypeChecker::new();
    builtins.seed_runtime_import_surface(&empty);
    println!(
        "type-surface\tfunctions={}\tparameter_slots={}",
        builtins.functions.len(),
        builtins
            .functions
            .values()
            .map(|signature| signature.params.len())
            .sum::<usize>()
    );
    for source in [
        "1",
        "seq:map((1, 2), fn(n) => n + 1)",
        "import \"cem:stdlib/sequence\" as other\nother:count((1, 2))",
        "import \"memory:opaque\" as host\nhost:custom(1)",
        "module:module_url(\"demo\")",
        "native:call(\"fixture\", 1)",
        "unknown:missing(1)",
        "1 + true",
    ] {
        let parsed = crate::api::parse(source);
        assert!(
            parsed.diagnostics.is_empty(),
            "{source}: {:?}",
            parsed.diagnostics
        );
        for config in [TyConfig::strict(), TyConfig::dev_profile()] {
            let mut baseline = TypeChecker::with_config(config.clone());
            baseline.declare_variable(
                QNameKey::new(None, "host_variable"),
                Type::atom(AtomType::Integer),
            );
            baseline.register_function(FunctionSignature {
                name: QNameKey::new(Some("host".into()), "keep"),
                params: vec![],
                ret: boolean_type(),
            });
            let mut candidate = baseline.clone();
            let mut prepared = baseline.clone();
            baseline.seed_runtime_import_surface(&parsed.module);
            seed_candidate(&mut candidate, &parsed.module);
            prepared.functions.extend(builtins.functions.clone());
            prepared
                .imported_prefixes
                .extend(builtins.imported_prefixes.clone());
            for node in &parsed.module.nodes {
                if let SurfaceNode::Import(import) = node {
                    prepared.register_import_surface(import);
                }
            }
            assert_eq!(baseline.functions, candidate.functions, "{source}");
            assert_eq!(baseline.imported_prefixes, candidate.imported_prefixes);
            assert_eq!(baseline.scopes, candidate.scopes);
            assert_eq!(baseline.functions, prepared.functions, "{source}");
            assert_eq!(baseline.imported_prefixes, prepared.imported_prefixes);
            assert_eq!(baseline.scopes, prepared.scopes);
            let before = baseline.check_surface_module(&parsed.module);
            let after = candidate.check_surface_module(&parsed.module);
            let reused = prepared.check_surface_module(&parsed.module);
            assert_eq!(before.root_type, after.root_type, "{source}");
            assert_eq!(before.diagnostics, after.diagnostics, "{source}");
            assert_eq!(before.root_type, reused.root_type, "{source}");
            assert_eq!(before.diagnostics, reused.diagnostics, "{source}");
        }
    }
    let module = crate::api::parse("1").module;
    for (label, seed) in [
        (
            "current",
            TypeChecker::seed_runtime_import_surface as fn(&mut TypeChecker, &SurfaceModule),
        ),
        ("local-registry-candidate", seed_candidate),
    ] {
        let mut times = Vec::new();
        // Count separately so every timed candidate runs with recording off.
        let (_, stages) = crate::compile_profile::measure(|| {
            let mut checker = TypeChecker::new();
            seed(&mut checker, &module);
            black_box(checker);
        });
        let assemblies = stages["stdlib/assemble-registry"].calls * 128;
        for _ in 0..6 {
            let start = Instant::now();
            for _ in 0..128 {
                let mut checker = TypeChecker::new();
                seed(&mut checker, &module);
                black_box(checker);
            }
            times.push(start.elapsed().as_secs_f64() * 1000.0);
        }
        let first = times.remove(0);
        times.sort_by(f64::total_cmp);
        println!("type-surface/{label}\titerations=128\tregistry_assemblies={assemblies}\tfirst_ms={first:.3}\tmedian_ms={:.3}\tmin_ms={:.3}\tmax_ms={:.3}", times[2], times[0], times[4]);
    }
    let mut times = Vec::new();
    for _ in 0..6 {
        let start = Instant::now();
        // One owned baseline per compilation session, no process-global cache.
        let mut prepared = TypeChecker::new();
        prepared.seed_runtime_import_surface(&module);
        for _ in 0..128 {
            black_box(prepared.clone());
        }
        drop(prepared);
        times.push(start.elapsed().as_secs_f64() * 1000.0);
    }
    let first = times.remove(0);
    times.sort_by(f64::total_cmp);
    println!("type-surface/prepared-baseline-candidate\titerations=128\tfirst_ms={first:.3}\tmedian_ms={:.3}\tmin_ms={:.3}\tmax_ms={:.3}", times[2], times[0], times[4]);
}
