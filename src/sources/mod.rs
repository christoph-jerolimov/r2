//! Discovery of runner files (Makefile, package.json, ...) and their tasks.

use anyhow::Result;
use std::path::{Path, PathBuf};

pub mod cargo;
pub mod custom;
pub mod just;
pub mod make;
pub mod package_json;
pub mod taskfile;

pub use custom::CustomTask;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    /// Tasks from r.yaml / run.yaml
    Custom,
    Make,
    /// package.json scripts, run through npm, yarn, pnpm or bun
    Package,
    Just,
    Taskfile,
    Cargo,
}

impl Kind {
    pub const ALL: [Kind; 6] = [
        Kind::Custom,
        Kind::Make,
        Kind::Package,
        Kind::Just,
        Kind::Taskfile,
        Kind::Cargo,
    ];

    /// Names usable on the command line to select this runner (`r <selector> <task>`).
    pub fn selectors(self) -> &'static [&'static str] {
        match self {
            Kind::Custom => &["r", "custom"],
            Kind::Make => &["make"],
            Kind::Package => &["npm", "yarn", "pnpm", "bun"],
            Kind::Just => &["just"],
            Kind::Taskfile => &["task"],
            Kind::Cargo => &["cargo"],
        }
    }

    pub fn from_selector(name: &str) -> Option<Kind> {
        Kind::ALL
            .into_iter()
            .find(|k| k.selectors().contains(&name))
    }

    /// What kind of file this runner needs, for error messages.
    pub fn file_hint(self) -> &'static str {
        match self {
            Kind::Custom => "r.yaml or run.yaml",
            Kind::Make => "Makefile",
            Kind::Package => "package.json",
            Kind::Just => "justfile",
            Kind::Taskfile => "Taskfile.yml",
            Kind::Cargo => "Cargo.toml",
        }
    }
}

#[derive(Debug, Clone)]
pub struct Task {
    pub name: String,
    pub description: Option<String>,
    /// Only set for tasks defined in r.yaml / run.yaml.
    pub custom: Option<CustomTask>,
}

impl Task {
    pub fn new(name: impl Into<String>, description: Option<String>) -> Task {
        Task {
            name: name.into(),
            description,
            custom: None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Source {
    pub kind: Kind,
    /// The runner file the tasks were read from.
    pub file: PathBuf,
    /// The program to call (make, yarn, ...). Also the name shown to the user.
    pub tool: String,
    pub tasks: Vec<Task>,
}

impl Source {
    pub fn find(&self, name: &str) -> Option<&Task> {
        self.tasks.iter().find(|t| t.name == name)
    }

    pub fn file_name(&self) -> String {
        self.file
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .into_owned()
    }
}

#[derive(Debug)]
pub struct Project {
    /// Directory containing the runner files; commands run from here.
    pub root: PathBuf,
    pub sources: Vec<Source>,
}

impl Project {
    pub fn source(&self, kind: Kind) -> Option<&Source> {
        self.sources.iter().find(|s| s.kind == kind)
    }
}

/// Find the nearest directory (starting at `start`, walking up) with at least
/// one runner file and load all runner files in it.
pub fn discover(start: &Path) -> Result<Option<Project>> {
    let mut dir = Some(start);
    while let Some(d) = dir {
        let sources = detect(d)?;
        if !sources.is_empty() {
            return Ok(Some(Project {
                root: d.to_path_buf(),
                sources,
            }));
        }
        dir = d.parent();
    }
    Ok(None)
}

fn detect(dir: &Path) -> Result<Vec<Source>> {
    let mut sources = Vec::new();
    let mut ignore = Vec::new();
    if let Some((source, config_ignore)) = custom::load(dir)? {
        sources.push(source);
        ignore = config_ignore;
    }
    let detected = [
        make::load(dir)?,
        package_json::load(dir)?,
        just::load(dir)?,
        taskfile::load(dir)?,
        cargo::load(dir)?,
    ];
    for source in detected.into_iter().flatten() {
        let ignored = ignore
            .iter()
            .any(|i| source.kind.selectors().contains(&i.as_str()) || *i == source.file_name());
        if !ignored {
            sources.push(source);
        }
    }
    Ok(sources)
}

/// First existing file out of `names` in `dir`.
pub(crate) fn first_existing(dir: &Path, names: &[&str]) -> Option<PathBuf> {
    names.iter().map(|n| dir.join(n)).find(|p| p.is_file())
}
