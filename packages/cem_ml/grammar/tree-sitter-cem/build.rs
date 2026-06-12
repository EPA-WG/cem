fn main() {
    let src_dir = std::path::Path::new("src");
    let mut build = cc::Build::new();
    build.include(src_dir);
    build.file(src_dir.join("parser.c"));
    build.flag_if_supported("-Wno-unused-but-set-variable");
    build.flag_if_supported("-Wno-unused-parameter");
    build.flag_if_supported("-Wno-unused-value");
    build.compile("tree-sitter-cem");
    println!("cargo:rerun-if-changed=src/parser.c");
}
