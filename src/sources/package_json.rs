use super::{Kind, Source, Task};
use anyhow::{Context, Result};
use std::path::Path;

pub fn load(dir: &Path) -> Result<Option<Source>> {
    let file = dir.join("package.json");
    if !file.is_file() {
        return Ok(None);
    }
    let text = std::fs::read_to_string(&file)
        .with_context(|| format!("failed to read {}", file.display()))?;
    let json: serde_json::Value = serde_json::from_str(&text)
        .with_context(|| format!("failed to parse {}", file.display()))?;
    let tasks = json
        .get("scripts")
        .and_then(|s| s.as_object())
        .map(|scripts| {
            scripts
                .iter()
                .map(|(name, cmd)| Task::new(name, cmd.as_str().map(str::to_string)))
                .collect()
        })
        .unwrap_or_default();
    let tool = detect_tool(dir, &json).to_string();
    Ok(Some(Source {
        kind: Kind::Package,
        file,
        tool,
        tasks,
    }))
}

/// Pick the package manager: the `packageManager` field wins, then lockfiles
/// (also in parent directories, for workspaces), then npm.
fn detect_tool(dir: &Path, json: &serde_json::Value) -> &'static str {
    const TOOLS: [&str; 4] = ["npm", "yarn", "pnpm", "bun"];
    if let Some(pm) = json.get("packageManager").and_then(|v| v.as_str()) {
        let name = pm.split('@').next().unwrap_or_default();
        if let Some(tool) = TOOLS.into_iter().find(|t| *t == name) {
            return tool;
        }
    }
    const LOCKFILES: [(&str, &str); 5] = [
        ("yarn.lock", "yarn"),
        ("pnpm-lock.yaml", "pnpm"),
        ("bun.lock", "bun"),
        ("bun.lockb", "bun"),
        ("package-lock.json", "npm"),
    ];
    let mut current = Some(dir);
    while let Some(d) = current {
        if let Some((_, tool)) = LOCKFILES.into_iter().find(|(f, _)| d.join(f).is_file()) {
            return tool;
        }
        current = d.parent();
    }
    "npm"
}
