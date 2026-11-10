//! Build-owned Rust selection, never supplied by runtime input or an artifact.
use std::{env, fs, path::PathBuf};
fn main() {
    println!("cargo:rustc-check-cfg=cfg(cem_selected_application)");
    println!("cargo:rerun-if-env-changed=CEM_APPLICATION_REGISTRY_RS");
    println!("cargo:rerun-if-env-changed=CEM_SELECTED_APPLICATION_DIR");
    if let Some(path) = env::var_os("CEM_SELECTED_APPLICATION_DIR") {
        assert!(
            env::var_os("CARGO_FEATURE_COMPONENT_RUNTIME").is_some(),
            "CEM_SELECTED_APPLICATION_DIR requires component-runtime"
        );
        assert!(
            env::var("CARGO_CFG_TARGET_ARCH").as_deref() != Ok("wasm32")
                || env::var_os("CARGO_FEATURE_SELECTED_APPLICATION_RUNTIME").is_some(),
            "selected application WASM requires selected-application-runtime"
        );
        assert!(
            env::var_os("CEM_APPLICATION_REGISTRY_RS").is_none(),
            "the selected application supplies its own registry"
        );
        let directory = PathBuf::from(path)
            .canonicalize()
            .expect("selected application build directory");
        for file in [
            "selected_application.rs",
            "application-entry.bin",
            "graphql-contracts.bin",
            "graphql-operations.graphql",
        ] {
            let path = directory.join(file);
            assert!(
                path.is_file(),
                "missing selected application input: {}",
                path.display()
            );
            println!("cargo:rerun-if-changed={}", path.display());
        }
        println!("cargo:rustc-cfg=cem_selected_application");
        println!(
            "cargo:rustc-env=CEM_SELECTED_APPLICATION_PATH={}",
            directory.join("selected_application.rs").display()
        );
        return;
    }
    if env::var_os("CARGO_FEATURE_COMPONENT_RUNTIME").is_none() {
        return;
    }
    let source = if let Some(path) = env::var_os("CEM_APPLICATION_REGISTRY_RS") {
        let path = PathBuf::from(path)
            .canonicalize()
            .expect("selected registry Rust path");
        println!("cargo:rerun-if-changed={}", path.display());
        fs::read_to_string(path).expect("read generated selected registry Rust")
    } else {
        // Building without an application recipe grants no function calls.
        "pub fn selected_registry() -> crate::selected_registry::SelectedFunctionRegistry { crate::selected_registry::SelectedFunctionRegistry::new([]).unwrap() }".into()
    };
    fs::write(
        PathBuf::from(env::var_os("OUT_DIR").unwrap()).join("application_registry.rs"),
        source,
    )
    .unwrap();
}
