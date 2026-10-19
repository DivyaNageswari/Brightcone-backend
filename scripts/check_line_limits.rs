use std::fs;
use std::io::{self, BufRead};
use std::path::{Path, PathBuf};

const MAXIMUM_LINES: usize = 400;
const CODE_DIRECTORIES: [&str; 3] = ["src", "tests", "examples"];

fn rust_sources(directory: &Path, files: &mut Vec<PathBuf>) -> io::Result<()> {
    for entry in fs::read_dir(directory)? {
        let path = entry?.path();
        if path.is_dir() {
            rust_sources(&path, files)?;
        } else if path.extension().is_some_and(|extension| extension == "rs") {
            files.push(path);
        }
    }
    Ok(())
}

fn line_count(path: &Path) -> io::Result<usize> {
    let file = fs::File::open(path)?;
    Ok(io::BufReader::new(file).lines().count())
}

fn main() -> io::Result<()> {
    let root = std::env::current_dir()?;
    let mut files = Vec::new();
    for directory in CODE_DIRECTORIES {
        rust_sources(&root.join(directory), &mut files)?;
    }
    let build_script = root.join("build.rs");
    if build_script.is_file() {
        files.push(build_script);
    }
    files.push(root.join("scripts/check_line_limits.rs"));

    let violations: Vec<_> = files
        .iter()
        .filter_map(|path| match line_count(path) {
            Ok(lines) if lines > MAXIMUM_LINES => Some((path, lines)),
            _ => None,
        })
        .collect();

    if violations.is_empty() {
        println!(
            "Verified {} Rust files: all are at most {} lines.",
            files.len(),
            MAXIMUM_LINES
        );
        return Ok(());
    }

    for (path, lines) in violations {
        eprintln!("{}: {lines} lines", path.display());
    }
    std::process::exit(1);
}
