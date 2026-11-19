//! The r.yaml / run.yaml config file: custom shell commands and containers.

use super::{Kind, Source, Task, first_existing};
use anyhow::{Context, Result, bail};
use serde::{Deserialize, Deserializer};
use serde_yaml_ng::Value;
use std::collections::BTreeMap;
use std::path::Path;

pub const FILE_NAMES: [&str; 4] = ["r.yaml", "r.yml", "run.yaml", "run.yml"];

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CustomTask {
    #[serde(default, alias = "desc")]
    pub description: Option<String>,
    /// One or more shell commands, run in order.
    #[serde(default, alias = "cmd", deserialize_with = "strings")]
    pub run: Vec<String>,
    /// Working directory, relative to the config file.
    #[serde(default)]
    pub dir: Option<String>,
    #[serde(default, deserialize_with = "string_map")]
    pub env: BTreeMap<String, String>,
    /// Tasks to run first.
    #[serde(default, deserialize_with = "strings")]
    pub deps: Vec<String>,
    /// Run in a container (with `run` as the command, if given).
    #[serde(default)]
    pub container: Option<Container>,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Container {
    pub image: String,
    /// docker, podman, ... Defaults to $R_CONTAINER_ENGINE or whichever is installed.
    #[serde(default)]
    pub engine: Option<String>,
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default, deserialize_with = "strings")]
    pub ports: Vec<String>,
    #[serde(default, deserialize_with = "string_map")]
    pub env: BTreeMap<String, String>,
    #[serde(default, deserialize_with = "strings")]
    pub volumes: Vec<String>,
    #[serde(default)]
    pub workdir: Option<String>,
    /// Command to run instead of the image's default.
    #[serde(default, deserialize_with = "strings")]
    pub command: Vec<String>,
    /// Extra arguments for `<engine> run`.
    #[serde(default, deserialize_with = "strings")]
    pub args: Vec<String>,
    #[serde(default)]
    pub detach: bool,
    /// Remove the container when it exits (default: true).
    #[serde(default = "yes")]
    pub rm: bool,
}

fn yes() -> bool {
    true
}

fn scalar<E: serde::de::Error>(v: Value) -> Result<String, E> {
    match v {
        Value::String(s) => Ok(s),
        Value::Number(n) => Ok(n.to_string()),
        Value::Bool(b) => Ok(b.to_string()),
        other => Err(E::custom(format!("expected a string, got {other:?}"))),
    }
}

/// A string or a list of strings (numbers and booleans are accepted too).
fn strings<'de, D: Deserializer<'de>>(d: D) -> Result<Vec<String>, D::Error> {
    match Value::deserialize(d)? {
        Value::Null => Ok(Vec::new()),
        Value::Sequence(items) => items.into_iter().map(scalar).collect(),
        other => Ok(vec![scalar(other)?]),
    }
}

fn string_map<'de, D: Deserializer<'de>>(d: D) -> Result<BTreeMap<String, String>, D::Error> {
    BTreeMap::<String, Value>::deserialize(d)?
        .into_iter()
        .map(|(k, v)| Ok((k, scalar(v)?)))
        .collect()
}

/// Load the config file in `dir`, returning its tasks and the list of ignored runners.
pub fn load(dir: &Path) -> Result<Option<(Source, Vec<String>)>> {
    let Some(file) = first_existing(dir, &FILE_NAMES) else {
        return Ok(None);
    };
    let text = std::fs::read_to_string(&file)
        .with_context(|| format!("failed to read {}", file.display()))?;
    let (tasks, ignore) =
        parse(&text).with_context(|| format!("invalid config {}", file.display()))?;
    Ok(Some((
        Source {
            kind: Kind::Custom,
            file,
            tool: "r".into(),
            tasks,
        },
        ignore,
    )))
}

pub fn parse(text: &str) -> Result<(Vec<Task>, Vec<String>)> {
    #[derive(Deserialize, Default)]
    #[serde(deny_unknown_fields)]
    struct Config {
        #[serde(default)]
        tasks: serde_yaml_ng::Mapping,
        #[serde(default, deserialize_with = "strings")]
        ignore: Vec<String>,
    }
    let config: Config = if text.trim().is_empty() {
        Config::default()
    } else {
        serde_yaml_ng::from_str(text)?
    };
    let mut tasks = Vec::new();
    for (name, body) in config.tasks {
        let Some(name) = name.as_str().map(str::to_string) else {
            bail!("task names must be strings, got {name:?}");
        };
        let custom = match body {
            // Shorthand: `name: command`
            Value::String(cmd) => CustomTask {
                run: vec![cmd],
                ..Default::default()
            },
            Value::Sequence(_) => CustomTask {
                run: strings(body).with_context(|| format!("task `{name}`"))?,
                ..Default::default()
            },
            body => serde_yaml_ng::from_value::<CustomTask>(body)
                .with_context(|| format!("task `{name}`"))?,
        };
        if custom.run.is_empty() && custom.container.is_none() && custom.deps.is_empty() {
            bail!("task `{name}` needs one of `run`, `container` or `deps`");
        }
        let description = custom
            .description
            .clone()
            .or_else(|| match &custom.container {
                Some(c) if custom.run.is_empty() => Some(format!("container {}", c.image)),
                _ => custom
                    .run
                    .first()
                    .map(|c| c.lines().next().unwrap_or_default().to_string()),
            });
        tasks.push(Task {
            name,
            description,
            custom: Some(custom),
        });
    }
    Ok((tasks, config.ignore))
}

pub const TEMPLATE: &str = r#"# Configuration for r (https://github.com/christoph-jerolimov/r2)
#
# Tasks defined here take precedence over tasks from Makefile, package.json, ...

# Runners to ignore in this directory (make, npm, just, task, cargo):
# ignore: [cargo]

tasks:
  # Short form: a shell command. Extra arguments are appended (`r hello world`).
  hello: echo Hello

  # Long form
  # check:
  #   description: Lint and test
  #   deps: [hello]          # tasks to run first
  #   dir: .                 # working directory, relative to this file
  #   env:
  #     CI: "true"
  #   run:
  #     - npm run lint
  #     - npm test

  # Start a container
  # db:
  #   description: Start a local postgres
  #   container:
  #     image: postgres:17
  #     name: dev-db
  #     ports: ["5432:5432"]
  #     env:
  #       POSTGRES_PASSWORD: postgres
  #     volumes: ["./data:/var/lib/postgresql/data"]
  #     detach: false        # run in the background
  #     # engine: podman     # default: $R_CONTAINER_ENGINE, docker or podman

  # Run a command inside a container
  # node-test:
  #   run: npm test
  #   container:
  #     image: node:22
  #     volumes: [".:/app"]
  #     workdir: /app
"#;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_config() {
        let (tasks, ignore) = parse(
            "ignore: cargo\ntasks:\n  hello: echo hi\n  multi: [a, b]\n  db:\n    desc: Database\n    \
             container:\n      image: postgres:17\n      ports: [5432]\n      env: {PORT: 5432}\n",
        )
        .unwrap();
        assert_eq!(ignore, ["cargo"]);
        assert_eq!(tasks.len(), 3);
        assert_eq!(tasks[0].custom.as_ref().unwrap().run, ["echo hi"]);
        assert_eq!(tasks[1].custom.as_ref().unwrap().run, ["a", "b"]);
        let c = tasks[2]
            .custom
            .as_ref()
            .unwrap()
            .container
            .as_ref()
            .unwrap();
        assert_eq!(
            (c.ports[0].as_str(), c.env["PORT"].as_str(), c.rm),
            ("5432", "5432", true)
        );
        assert_eq!(tasks[2].description.as_deref(), Some("Database"));
    }

    #[test]
    fn template_is_valid() {
        let (tasks, _) = parse(TEMPLATE).unwrap();
        assert_eq!(tasks[0].name, "hello");
    }

    #[test]
    fn rejects_unknown_fields_and_empty_tasks() {
        assert!(parse("tasks:\n  a:\n    rnu: x\n").is_err());
        assert!(parse("tasks:\n  a:\n    description: x\n").is_err());
    }
}
