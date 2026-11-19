use super::{Kind, Source, Task, first_existing};
use anyhow::{Context, Result};
use std::path::Path;

pub fn load(dir: &Path) -> Result<Option<Source>> {
    let Some(file) = first_existing(dir, &["justfile", "Justfile", ".justfile"]) else {
        return Ok(None);
    };
    let text = std::fs::read_to_string(&file)
        .with_context(|| format!("failed to read {}", file.display()))?;
    Ok(Some(Source {
        kind: Kind::Just,
        file,
        tool: "just".into(),
        tasks: parse(&text),
    }))
}

fn valid_name(name: &str) -> bool {
    !name.is_empty()
        && !name.starts_with('_')
        && name
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
}

/// Extract public recipes and aliases; the comment above a recipe is its description.
pub fn parse(text: &str) -> Vec<Task> {
    let mut tasks: Vec<Task> = Vec::new();
    let mut comment: Option<String> = None;
    let mut private = false;
    for line in text.lines() {
        if line.starts_with([' ', '\t']) {
            continue; // recipe body
        }
        if let Some(c) = line.strip_prefix('#') {
            comment = Some(c.trim().to_string()).filter(|c| !c.is_empty() && !c.starts_with('!'));
            continue;
        }
        if line.starts_with('[') {
            private |= line.contains("private");
            continue; // attribute, keeps the comment above it
        }
        let above = comment.take();
        let was_private = std::mem::take(&mut private);
        let mut words = line.split_whitespace();
        let Some(first) = words.next() else { continue };
        if first == "alias" {
            let name = words.next().unwrap_or_default().trim_end_matches(":=");
            let target = line.split(":=").nth(1).unwrap_or_default().trim();
            if valid_name(name) && !was_private {
                tasks.push(Task::new(name, Some(format!("alias for {target}"))));
            }
            continue;
        }
        if ["set", "import", "mod", "export", "unexport"].contains(&first) {
            continue;
        }
        let Some(colon) = line.find(':') else {
            continue;
        };
        let name = line[..colon].split_whitespace().next().unwrap_or_default();
        let name = name.trim_start_matches('@');
        // `name := value` is a variable assignment
        if line[name.len()..].trim_start().starts_with(":=") && !line.starts_with('@') {
            continue;
        }
        if valid_name(name) && !was_private && !tasks.iter().any(|t| t.name == name) {
            tasks.push(Task::new(name, above));
        }
    }
    tasks
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_recipes() {
        let tasks = parse(
            "set shell := [\"bash\", \"-c\"]\nversion := \"1.0\"\nalias b := build\n\n# Build the app\n\
             build target=\"a:b\":\n    cargo build\n\n[private]\nhidden:\n    true\n\n_helper:\n    true\n\n\
             @test *args: build\n    cargo test\n",
        );
        let names: Vec<_> = tasks.iter().map(|t| t.name.as_str()).collect();
        assert_eq!(names, ["b", "build", "test"]);
        assert_eq!(tasks[1].description.as_deref(), Some("Build the app"));
    }
}
