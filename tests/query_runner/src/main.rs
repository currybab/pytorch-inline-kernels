//! Usage: query_runner <injections.scm> <source.py>
//!
//! Prints one line per injection match: `<language>\t<start>-<end>[,<start>-<end>...]`
//! with byte ranges of the `@injection.content` nodes, mirroring how Zed's
//! syntax map turns each match into one injection layer.

use std::process::ExitCode;

use streaming_iterator::StreamingIterator;
use tree_sitter::{Language, Parser, Query, QueryCursor};

fn run(query_path: &str, source_path: &str) -> Result<(), String> {
    let read = |path: &str| std::fs::read_to_string(path).map_err(|e| format!("{path}: {e}"));
    let query_source = read(query_path)?;
    let source = read(source_path)?;

    let language: Language = tree_sitter_python::LANGUAGE.into();
    let mut parser = Parser::new();
    parser.set_language(&language).map_err(|e| e.to_string())?;
    let tree = parser.parse(&source, None).ok_or("parse failed")?;
    let query = Query::new(&language, &query_source).map_err(|e| e.to_string())?;
    let content = query
        .capture_index_for_name("injection.content")
        .ok_or("query has no @injection.content")?;

    let mut cursor = QueryCursor::new();
    let mut matches = cursor.matches(&query, tree.root_node(), source.as_bytes());
    while let Some(m) = matches.next() {
        let language = query
            .property_settings(m.pattern_index)
            .iter()
            .find(|p| &*p.key == "injection.language")
            .and_then(|p| p.value.as_deref())
            .unwrap_or("");
        let ranges: Vec<String> = m
            .nodes_for_capture_index(content)
            .map(|n| format!("{}-{}", n.start_byte(), n.end_byte()))
            .collect();
        if !ranges.is_empty() {
            println!("{language}\t{}", ranges.join(","));
        }
    }
    Ok(())
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().collect();
    let [_, query, source] = args.as_slice() else {
        eprintln!("usage: query_runner <injections.scm> <source.py>");
        return ExitCode::from(2);
    };
    match run(query, source) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("query_runner: {error}");
            ExitCode::FAILURE
        }
    }
}
