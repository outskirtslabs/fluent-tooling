fn main() {
    let grammar_root = std::path::Path::new("../..");
    let source_dir = grammar_root.join("src");

    println!(
        "cargo:rerun-if-changed={}",
        source_dir.join("parser.c").display()
    );
    println!(
        "cargo:rerun-if-changed={}",
        source_dir.join("scanner.c").display()
    );

    println!("cargo:rerun-if-env-changed=FLUENT_SCANNER_DEBUG");

    let mut build = cc::Build::new();
    build
        .include(&source_dir)
        .file(source_dir.join("parser.c"))
        .file(source_dir.join("scanner.c"))
        .warnings(true);
    if std::env::var_os("FLUENT_SCANNER_DEBUG").is_some() {
        build.define("FLUENT_IS_DEBUG", None);
    }
    build.compile("tree-sitter-fluent");
}
