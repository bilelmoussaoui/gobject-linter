use std::{env, path::PathBuf};

use anyhow::Result;
use gobject_ast::{Parser, Project};

fn main() -> Result<()> {
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .with_target(true)
        .with_line_number(true)
        .init();

    let args: Vec<String> = env::args().collect();

    if args.len() < 2 {
        eprintln!("Usage: gobject-ast <file.c|file.h|directory> [raw_tree]");
        eprintln!("\nParse a C/header file or directory and print the AST model");
        eprintln!("  raw_tree: Print raw tree-sitter AST instead of parsed model");
        std::process::exit(1);
    }

    let path = PathBuf::from(&args[1]);
    let show_raw = args.len() > 2 && args[2] == "raw_tree";

    if show_raw {
        // Print raw tree-sitter output
        let source = std::fs::read(&path)?;
        let mut ts_parser = tree_sitter::Parser::new();
        ts_parser.set_language(&tree_sitter_c_gobject::LANGUAGE.into())?;
        let tree = ts_parser.parse(&source, None).unwrap();
        print_raw_tree(tree.root_node(), &source, 0);
        return Ok(());
    }

    let mut parser = Parser::new()?;

    tracing::info!("Starting to parse: {}", path.display());

    let project = if path.is_dir() {
        eprintln!("Parsing directory: {}", path.display());
        parser.parse_directory(&path)?
    } else {
        eprintln!("Parsing file: {}", path.display());
        parser.parse_file(&path)?
    };

    tracing::info!("Finished parsing, found {} files", project.files.len());

    print_project(&project);

    Ok(())
}

fn print_raw_tree(node: tree_sitter::Node, source: &[u8], depth: usize) {
    let indent = "  ".repeat(depth);
    let text = if node.child_count() == 0 {
        let s = std::str::from_utf8(&source[node.byte_range()]).unwrap_or("");
        if s.len() > 50 {
            format!(" '{}'...", &s[..50])
        } else {
            format!(" '{}'", s)
        }
    } else {
        String::new()
    };
    println!(
        "{}{} [{}:{}->{}:{}]{}",
        indent,
        node.kind(),
        node.start_position().row,
        node.start_position().column,
        node.end_position().row,
        node.end_position().column,
        text
    );

    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        print_raw_tree(child, source, depth + 1);
    }
}

fn print_project(project: &Project) {
    println!("\n=== FILES ({}) ===\n", project.files.len());

    let mut sorted_files: Vec<_> = project.files.iter().collect();
    sorted_files.sort_by_key(|(path, _)| *path);

    for (path, file) in sorted_files {
        println!("{}:", path.display());

        // Print top-level items
        println!("  Top-level items ({}):", file.top_level_items.len());
        println!("{:#?}", file.top_level_items);

        println!();
    }
}
