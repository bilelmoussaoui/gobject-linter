fn main() {
    let mut c_config = cc::Build::new();
    c_config.std("c11").include("src");

    let parser_path = std::path::Path::new("src/parser.c");
    c_config.file(parser_path);
    println!("cargo:rerun-if-changed={}", parser_path.to_str().unwrap());

    let scanner_path = std::path::Path::new("src/scanner.c");
    c_config.file(scanner_path);
    println!("cargo:rerun-if-changed={}", scanner_path.to_str().unwrap());
    println!("cargo:rerun-if-changed=src/modifiers.h");

    c_config.compile("tree-sitter-c-gobject");
}
