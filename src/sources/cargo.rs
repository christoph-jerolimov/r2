use super::{Kind, Source, Task};
use anyhow::Result;
use std::path::Path;

/// Cargo has no user-defined tasks, so offer the common built-in commands.
const COMMANDS: [(&str, &str); 9] = [
    ("build", "Compile the current package"),
    ("check", "Check the package for errors without building"),
    ("test", "Run the tests"),
    ("run", "Run the binary"),
    ("bench", "Run the benchmarks"),
    ("doc", "Build the documentation"),
    ("clippy", "Run the clippy lints"),
    ("fmt", "Format the code"),
    ("clean", "Remove the target directory"),
];

pub fn load(dir: &Path) -> Result<Option<Source>> {
    let file = dir.join("Cargo.toml");
    if !file.is_file() {
        return Ok(None);
    }
    let tasks = COMMANDS
        .iter()
        .map(|(n, d)| Task::new(*n, Some(d.to_string())))
        .collect();
    Ok(Some(Source {
        kind: Kind::Cargo,
        file,
        tool: "cargo".into(),
        tasks,
    }))
}
