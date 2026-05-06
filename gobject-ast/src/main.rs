use std::path::PathBuf;

use anyhow::{Result, bail};
use clap::{Parser, Subcommand, ValueEnum};
use gobject_ast::{
    Parser as AstParser, Project,
    model::top_level::{TopLevelItem, TopLevelItemKind},
};
use serde_json::json;
use tree_sitter::Node;

fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .with_target(false)
        .init();

    Cli::parse().command.run()
}

#[derive(Parser)]
#[command(
    name = "gobject-ast",
    about = "Debug and inspection tool for the gobject-ast parser"
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Parse files and display the gobject-ast model
    Model {
        /// File or directory to parse
        path: PathBuf,
        /// Output format
        #[arg(long, default_value = "pretty", value_name = "FORMAT")]
        format: Format,
        /// Filter to items whose name contains this string
        #[arg(long, value_name = "NAME")]
        find: Option<String>,
        /// Filter by item kind
        #[arg(long, value_name = "KIND")]
        kind: Option<TopLevelItemKind>,
    },
    /// Display the raw tree-sitter parse tree
    Tree {
        /// File to parse
        path: PathBuf,
        /// Output format
        #[arg(long, default_value = "pretty", value_name = "FORMAT")]
        format: Format,
        /// Show only subtrees rooted at nodes of this tree-sitter kind (e.g.
        /// function_definition)
        #[arg(long, value_name = "KIND")]
        kind: Option<String>,
        /// Show the ancestor spine leading to the deepest node at this 1-based
        /// line number
        #[arg(long, value_name = "LINE")]
        line: Option<usize>,
    },
}

impl Command {
    fn run(self) -> Result<()> {
        match self {
            Self::Model {
                path,
                format,
                find,
                kind,
            } => {
                let mut parser = AstParser::new()?;
                let project = if path.is_dir() {
                    parser.parse_directory(&path)?
                } else {
                    parser.parse_file(&path)?
                };
                format.print_model(&project, find.as_deref(), kind.as_ref())
            }
            Self::Tree {
                path,
                format,
                kind,
                line,
            } => {
                if path.is_dir() {
                    bail!("'tree' subcommand requires a single file, not a directory");
                }
                let source = std::fs::read(&path)?;
                let mut ts_parser = tree_sitter::Parser::new();
                ts_parser.set_language(&tree_sitter_c_gobject::LANGUAGE.into())?;
                let tree = ts_parser.parse(&source, None).unwrap();
                format.print_tree(tree.root_node(), &source, kind.as_deref(), line)
            }
        }
    }
}

#[derive(ValueEnum, Clone, Copy)]
enum Format {
    /// Human-readable text (debug model, indented tree)
    Pretty,
    /// JSON output
    Json,
}

impl Format {
    fn print_model(
        self,
        project: &Project,
        find: Option<&str>,
        kind: Option<&TopLevelItemKind>,
    ) -> Result<()> {
        let mut sorted: Vec<_> = project.files.iter().collect();
        sorted.sort_by_key(|(p, _)| *p);
        let filtering = find.is_some() || kind.is_some();

        match self {
            Self::Pretty => {
                for (path, file) in &sorted {
                    if filtering {
                        let matches: Vec<_> = file
                            .iter_all_items()
                            .filter(|item| item_passes_filters(item, find, kind))
                            .collect();
                        if matches.is_empty() {
                            continue;
                        }
                        let s = if matches.len() == 1 { "" } else { "es" };
                        println!(
                            "=== {} ({} match{}) ===\n",
                            path.display(),
                            matches.len(),
                            s
                        );
                        for item in matches {
                            println!("{:#?}\n", item);
                        }
                    } else {
                        println!("=== {} ===\n", path.display());
                        println!("{:#?}\n", file.top_level_items);
                    }
                }
            }
            Self::Json => {
                if !filtering {
                    println!("{}", serde_json::to_string_pretty(project)?);
                    return Ok(());
                }
                let results: Vec<_> = sorted
                    .iter()
                    .filter_map(|(path, file)| {
                        let items: Vec<_> = file
                            .iter_all_items()
                            .filter(|item| item_passes_filters(item, find, kind))
                            .collect();
                        if items.is_empty() {
                            None
                        } else {
                            Some(json!({ "file": path.display().to_string(), "items": items }))
                        }
                    })
                    .collect();
                println!("{}", serde_json::to_string_pretty(&results)?);
            }
        }
        Ok(())
    }

    fn print_tree(
        self,
        root: Node<'_>,
        source: &[u8],
        kind: Option<&str>,
        line: Option<usize>,
    ) -> Result<()> {
        match (kind, line) {
            (None, None) => match self {
                Self::Pretty => print_tree_text(root, source, 0),
                Self::Json => {
                    println!(
                        "{}",
                        serde_json::to_string_pretty(&node_to_json(root, source))?
                    );
                }
            },

            (Some(k), None) => {
                let mut nodes = Vec::new();
                collect_nodes_of_kind(root, k, &mut nodes);
                if nodes.is_empty() {
                    eprintln!("No nodes of kind {:?} found", k);
                }
                match self {
                    Self::Pretty => {
                        for node in nodes {
                            println!(
                                "--- {} @ {}:{} ---",
                                node.kind(),
                                node.start_position().row + 1,
                                node.start_position().column
                            );
                            print_tree_text(node, source, 0);
                            println!();
                        }
                    }
                    Self::Json => {
                        let arr: Vec<_> = nodes.iter().map(|n| node_to_json(*n, source)).collect();
                        println!("{}", serde_json::to_string_pretty(&arr)?);
                    }
                }
            }

            (None, Some(line_num)) => {
                let spine = find_spine_to_line(root, line_num - 1);
                if spine.is_empty() {
                    eprintln!("No node found at line {}", line_num);
                    return Ok(());
                }
                let deepest = *spine.last().unwrap();
                match self {
                    Self::Pretty => {
                        for (depth, node) in spine.iter().enumerate() {
                            println!(
                                "{}↳ {} [{}:{} → {}:{}]",
                                "  ".repeat(depth),
                                node.kind(),
                                node.start_position().row + 1,
                                node.start_position().column,
                                node.end_position().row + 1,
                                node.end_position().column,
                            );
                        }
                        println!();
                        println!("--- deepest node at line {} ---", line_num);
                        print_tree_text(deepest, source, 0);
                    }
                    Self::Json => {
                        let spine_json: Vec<_> = spine
                            .iter()
                            .map(|n| {
                                json!({
                                    "kind": n.kind(),
                                    "start_line": n.start_position().row + 1,
                                    "start_col": n.start_position().column,
                                    "end_line": n.end_position().row + 1,
                                    "end_col": n.end_position().column,
                                })
                            })
                            .collect();
                        println!(
                            "{}",
                            serde_json::to_string_pretty(&json!({
                                "spine": spine_json,
                                "deepest": node_to_json(deepest, source),
                            }))?
                        );
                    }
                }
            }

            (Some(k), Some(line_num)) => {
                let spine = find_spine_to_line(root, line_num - 1);
                match spine.iter().rev().find(|n| n.kind() == k) {
                    None => eprintln!(
                        "No ancestor of kind {:?} found containing line {}",
                        k, line_num
                    ),
                    Some(node) => match self {
                        Self::Pretty => {
                            println!(
                                "--- {} @ {}:{} (containing line {}) ---",
                                node.kind(),
                                node.start_position().row + 1,
                                node.start_position().column,
                                line_num
                            );
                            print_tree_text(*node, source, 0);
                        }
                        Self::Json => {
                            println!(
                                "{}",
                                serde_json::to_string_pretty(&node_to_json(*node, source))?
                            );
                        }
                    },
                }
            }
        }
        Ok(())
    }
}

fn item_passes_filters(
    item: &TopLevelItem,
    find: Option<&str>,
    kind: Option<&TopLevelItemKind>,
) -> bool {
    if let Some(k) = kind
        && item.kind() != *k
    {
        return false;
    }
    if let Some(search) = find {
        match item.name() {
            Some(name) if name.contains(search) => {}
            _ => return false,
        }
    }
    true
}

fn print_tree_text(node: Node, source: &[u8], depth: usize) {
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
        "{}{} [{}:{} → {}:{}]{}",
        "  ".repeat(depth),
        node.kind(),
        node.start_position().row + 1,
        node.start_position().column,
        node.end_position().row + 1,
        node.end_position().column,
        text,
    );
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        print_tree_text(child, source, depth + 1);
    }
}

fn node_to_json(node: Node, source: &[u8]) -> serde_json::Value {
    let text = std::str::from_utf8(&source[node.byte_range()]).unwrap_or("");
    let text_preview = if text.len() < 100 && !text.contains('\n') {
        Some(text.to_string())
    } else {
        None
    };

    let mut children = Vec::new();
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        children.push(node_to_json(child, source));
    }

    json!({
        "kind": node.kind(),
        "start_line": node.start_position().row + 1,
        "start_col": node.start_position().column,
        "end_line": node.end_position().row + 1,
        "end_col": node.end_position().column,
        "text": text_preview,
        "children": if children.is_empty() { serde_json::Value::Null } else { json!(children) },
    })
}

fn collect_nodes_of_kind<'a>(node: Node<'a>, kind: &str, results: &mut Vec<Node<'a>>) {
    if node.kind() == kind {
        results.push(node);
        return;
    }
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        collect_nodes_of_kind(child, kind, results);
    }
}

fn find_spine_to_line<'a>(root: Node<'a>, target_row: usize) -> Vec<Node<'a>> {
    let mut spine = Vec::new();
    let mut current = root;
    loop {
        spine.push(current);
        let mut cursor = current.walk();
        let next = current
            .children(&mut cursor)
            .find(|c| c.start_position().row <= target_row && target_row <= c.end_position().row);
        match next {
            Some(child) => current = child,
            None => break,
        }
    }
    spine
}
