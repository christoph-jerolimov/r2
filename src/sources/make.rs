use super::{Kind, Source, Task, first_existing};
use anyhow::{Context, Result};
use std::path::Path;

pub fn load(dir: &Path) -> Result<Option<Source>> {
    let Some(file) = first_existing(dir, &["GNUmakefile", "makefile", "Makefile"]) else {
        return Ok(None);
    };
    let text = std::fs::read_to_string(&file)
        .with_context(|| format!("failed to read {}", file.display()))?;
    Ok(Some(Source {
        kind: Kind::Make,
        file,
        tool: "make".into(),
        tasks: parse(&text),
    }))
}

/// Extract explicit targets. A description is taken from a trailing
/// `## text` or from a comment line directly above the target.
pub fn parse(text: &str) -> Vec<Task> {
    let mut tasks: Vec<Task> = Vec::new();
    let mut comment: Option<String> = None;
    for line in text.lines() {
        if line.starts_with('\t') {
            continue; // recipe line
        }
        let trimmed = line.trim();
        if let Some(c) = trimmed.strip_prefix('#') {
            comment = Some(c.trim_start_matches('#').trim().to_string()).filter(|c| !c.is_empty());
            continue;
        }
        let above = comment.take();
        let Some(colon) = line.find(':') else {
            continue;
        };
        let (head, rest) = (&line[..colon], &line[colon + 1..]);
        // Variable assignments: `A = b:c`, `A := b`, `A ::= b`
        if head.contains('=') || rest.starts_with('=') || rest.starts_with(":=") {
            continue;
        }
        let description = rest
            .find("##")
            .map(|i| rest[i + 2..].trim().to_string())
            .filter(|d| !d.is_empty())
            .or(above);
        for target in head.split_whitespace() {
            let valid = !target.starts_with('.')
                && target
                    .chars()
                    .all(|c| c.is_ascii_alphanumeric() || "_-./".contains(c));
            if valid && !tasks.iter().any(|t| t.name == target) {
                tasks.push(Task::new(target, description.clone()));
            }
        }
    }
    tasks
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_targets_and_descriptions() {
        let tasks = parse(
            "CC := gcc\nFLAGS = -a:b\n.PHONY: build test\n\n# Build it\nbuild: main.o\n\t$(CC) -o x\n\
             test lint: build ## Run checks\n%.o: %.c\n\t$(CC) -c $<\nclean::\n\trm -f x\n",
        );
        let names: Vec<_> = tasks.iter().map(|t| t.name.as_str()).collect();
        assert_eq!(names, ["build", "test", "lint", "clean"]);
        assert_eq!(tasks[0].description.as_deref(), Some("Build it"));
        assert_eq!(tasks[1].description.as_deref(), Some("Run checks"));
        assert_eq!(tasks[3].description, None);
    }
}
