//! Build-owned Rust selection, never supplied by runtime input or an artifact.
use std::{env, fs, path::PathBuf};
fn main() {
    println!("cargo:rerun-if-env-changed=CEM_APPLICATION_REGISTRY_RS");
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
