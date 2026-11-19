use super::{Kind, Source, Task, first_existing};
use anyhow::{Context, Result};
use serde_yaml_ng::Value;
use std::path::Path;

const NAMES: [&str; 8] = [
    "Taskfile.yml",
    "taskfile.yml",
    "Taskfile.yaml",
    "taskfile.yaml",
    "Taskfile.dist.yml",
    "taskfile.dist.yml",
    "Taskfile.dist.yaml",
    "taskfile.dist.yaml",
];

pub fn load(dir: &Path) -> Result<Option<Source>> {
    let Some(file) = first_existing(dir, &NAMES) else {
        return Ok(None);
    };
    let text = std::fs::read_to_string(&file)
        .with_context(|| format!("failed to read {}", file.display()))?;
    let yaml: Value = serde_yaml_ng::from_str(&text)
        .with_context(|| format!("failed to parse {}", file.display()))?;
    let mut tasks = Vec::new();
    if let Some(map) = yaml.get("tasks").and_then(|t| t.as_mapping()) {
        for (name, body) in map {
            let Some(name) = name.as_str() else { continue };
            if body.get("internal").and_then(|v| v.as_bool()) == Some(true) {
                continue;
            }
            let description = ["desc", "summary"]
                .iter()
                .find_map(|k| body.get(k).and_then(|v| v.as_str()))
                .map(|d| d.lines().next().unwrap_or_default().to_string());
            tasks.push(Task::new(name, description));
        }
    }
    Ok(Some(Source {
        kind: Kind::Taskfile,
        file,
        tool: "task".into(),
        tasks,
    }))
}
