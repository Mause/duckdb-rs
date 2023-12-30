#!/usr/bin/env rust-script
//! This is a regular crate doc comment, but it also contains a partial
//! Cargo manifest.  Note the use of a *fenced* code block, and the
//! `cargo` "language".
//!
//! ```cargo
//! [dependencies]
//! clang = { version = "*", features = ["clang_10_0"] }
//! shellexpand = "*"
//! grep = "*"
//! glob = "*"
//! ```

use glob::glob;
use grep::searcher::Searcher;
use std::{error::Error, sync::atomic::AtomicI64};

pub fn main() -> Result<(), Box<dyn std::error::Error>> {
    let clang = clang::Clang::new().unwrap();

    let index = clang::Index::new(&clang, true, true);

    let parser =
        index.parser(shellexpand::tilde("~/duckdb-rs/libduckdb-sys/duckdb-sources/src/include/duckdb.h").to_string());

    let tu = parser.parse().expect("unable to parse");

    let functions: Vec<String> = clang::sonar::find_functions(tu.get_entity().get_children())
        .into_iter()
        .map(|cursor| cursor.name)
        .filter(|name| name.starts_with("duckdb_"))
        .collect();

    // recursively search for all functions
    let files: Vec<String> = glob(&shellexpand::tilde("~/duckdb-rs/src/**/*.rs"))?
        .map(|f| f.expect("glob").to_string_lossy().to_string())
        .collect();

    println!("files: {:?}", &files.len());
    println!("functions: {:?}", functions.len());

    for function in &functions {
        if search_function(function, &files)? == 0 {
            println!("{} not found", function);
        }
    }

    Ok(())
}

fn search_function(function: &str, files: &Vec<String>) -> Result<i64, Box<dyn Error>> {
    let matcher = grep::regex::RegexMatcherBuilder::new().build(function).unwrap();

    let count = AtomicI64::new(0);

    let searcher = &mut Searcher::new();
    for path in files {
        searcher.search_path(
            matcher.clone(),
            path,
            grep::searcher::sinks::UTF8(|_, _| {
                count.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                Ok(true)
            }),
        )?;
    }

    Ok(count.load(std::sync::atomic::Ordering::SeqCst))
}
