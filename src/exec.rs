//! Turning a task into concrete commands, and running them.

use crate::sources::{CustomTask, Kind, Project, Source, Task};
use anyhow::{Context, Result, bail};
use std::collections::BTreeMap;
use std::io::IsTerminal;
use std::path::{Path, PathBuf};
use std::process::Command;

/// One command to execute.
#[derive(Debug)]
pub struct Step {
    pub argv: Vec<String>,
    pub dir: PathBuf,
    pub env: BTreeMap<String, String>,
}

impl Step {
    /// The step as a copy-pasteable shell line.
    pub fn display(&self) -> String {
        let env = self.env.iter().map(|(k, v)| format!("{k}={}", quote(v)));
        env.chain(self.argv.iter().map(|a| quote(a)))
            .collect::<Vec<_>>()
            .join(" ")
    }
}

/// Quote a string for a POSIX shell, if needed.
pub fn quote(s: &str) -> String {
    let safe = |c: char| c.is_ascii_alphanumeric() || "_-./=:@%+,".contains(c);
    if !s.is_empty() && s.chars().all(safe) {
        s.to_string()
    } else {
        format!("'{}'", s.replace('\'', "'\\''"))
    }
}

/// A task that was matched to exactly one runner.
pub struct Resolved<'a> {
    pub source: &'a Source,
    pub task: &'a Task,
}

/// Find the one runner that provides `name`. Tasks from r.yaml win over
/// detected ones; otherwise more than one match is an error.
pub fn resolve<'a>(project: &'a Project, name: &str) -> Result<Resolved<'a>> {
    if let Some(source) = project.source(Kind::Custom)
        && let Some(task) = source.find(name)
    {
        return Ok(Resolved { source, task });
    }
    let matches: Vec<Resolved> = project
        .sources
        .iter()
        .filter_map(|source| source.find(name).map(|task| Resolved { source, task }))
        .collect();
    match matches.len() {
        1 => Ok(matches.into_iter().next().unwrap()),
        0 => {
            let mut msg = format!("no task named `{name}`");
            if let Some(similar) = closest(project, name) {
                msg += &format!(", did you mean `{similar}`?");
            }
            bail!("{msg}\nRun `r list` to see all available tasks.");
        }
        _ => {
            let options: Vec<String> = matches
                .iter()
                .map(|m| format!("  r {} {name}    ({})", m.source.tool, m.source.file_name()))
                .collect();
            bail!(
                "`{name}` is ambiguous, it is defined by {} runners. Pick one:\n{}",
                matches.len(),
                options.join("\n")
            );
        }
    }
}

fn closest<'a>(project: &'a Project, name: &str) -> Option<&'a str> {
    project
        .sources
        .iter()
        .flat_map(|s| &s.tasks)
        .map(|t| (distance(name, &t.name), t.name.as_str()))
        .filter(|(d, candidate)| *d <= 2.max(name.len() / 3) || candidate.starts_with(name))
        .min_by_key(|(d, _)| *d)
        .map(|(_, n)| n)
}

/// Levenshtein distance.
fn distance(a: &str, b: &str) -> usize {
    let b: Vec<char> = b.chars().collect();
    let mut row: Vec<usize> = (0..=b.len()).collect();
    for (i, ca) in a.chars().enumerate() {
        let mut prev = row[0];
        row[0] = i + 1;
        for (j, cb) in b.iter().enumerate() {
            let cost = if ca == *cb { prev } else { prev + 1 };
            prev = row[j + 1];
            row[j + 1] = cost.min(prev + 1).min(row[j] + 1);
        }
    }
    row[b.len()]
}

/// The commands needed to run `task` from `source` with extra `args`.
/// `tool` overrides the program (e.g. `r npm start` in a yarn project).
pub fn plan(
    project: &Project,
    source: &Source,
    task: &Task,
    tool: Option<&str>,
    args: &[String],
) -> Result<Vec<Step>> {
    let mut steps = Vec::new();
    plan_into(
        project,
        source,
        task,
        tool,
        args,
        &mut Vec::new(),
        &mut steps,
    )?;
    Ok(steps)
}

fn plan_into(
    project: &Project,
    source: &Source,
    task: &Task,
    tool: Option<&str>,
    args: &[String],
    stack: &mut Vec<String>,
    steps: &mut Vec<Step>,
) -> Result<()> {
    let Some(custom) = &task.custom else {
        let argv = tool_argv(source.kind, tool.unwrap_or(&source.tool), &task.name, args);
        steps.push(Step {
            argv,
            dir: project.root.clone(),
            env: BTreeMap::new(),
        });
        return Ok(());
    };
    if stack.contains(&task.name) {
        bail!("dependency cycle: {} -> {}", stack.join(" -> "), task.name);
    }
    stack.push(task.name.clone());
    for dep in &custom.deps {
        let dep = resolve(project, dep).with_context(|| format!("in deps of `{}`", task.name))?;
        plan_into(project, dep.source, dep.task, None, &[], stack, steps)?;
    }
    stack.pop();

    let dir = match &custom.dir {
        Some(d) => project.root.join(d),
        None => project.root.clone(),
    };
    if custom.container.is_some() {
        let argv = container_argv(custom, &dir, args);
        steps.push(Step {
            argv,
            dir,
            env: custom.env.clone(),
        });
    } else {
        for (i, cmd) in custom.run.iter().enumerate() {
            let last = i + 1 == custom.run.len();
            let cmd = with_args(cmd, if last { args } else { &[] });
            steps.push(Step {
                argv: shell_argv(cmd),
                dir: dir.clone(),
                env: custom.env.clone(),
            });
        }
    }
    Ok(())
}

/// How each tool wants to be called to run a task with extra arguments.
fn tool_argv(kind: Kind, tool: &str, task: &str, args: &[String]) -> Vec<String> {
    let mut argv = vec![tool.to_string()];
    match kind {
        Kind::Package => argv.push("run".into()),
        Kind::Make | Kind::Just | Kind::Taskfile | Kind::Cargo | Kind::Custom => {}
    }
    argv.push(task.to_string());
    // npm and task need `--` before arguments meant for the script
    let separator = kind == Kind::Taskfile || (kind == Kind::Package && tool == "npm");
    if separator && !args.is_empty() {
        argv.push("--".into());
    }
    argv.extend(args.iter().cloned());
    argv
}

/// Calling a tool directly, for things that are not a known task
/// (`r npm install`, `r make some-pattern-target`).
pub fn passthrough(project: &Project, tool: &str, args: &[String]) -> Step {
    let mut argv = vec![tool.to_string()];
    argv.extend(args.iter().cloned());
    Step {
        argv,
        dir: project.root.clone(),
        env: BTreeMap::new(),
    }
}

fn with_args(cmd: &str, args: &[String]) -> String {
    let mut cmd = cmd.trim_end().to_string();
    for arg in args {
        cmd.push(' ');
        cmd.push_str(&quote(arg));
    }
    cmd
}

fn shell_argv(cmd: String) -> Vec<String> {
    if cfg!(windows) {
        vec!["cmd".into(), "/C".into(), cmd]
    } else {
        vec!["sh".into(), "-c".into(), cmd]
    }
}

fn container_argv(task: &CustomTask, dir: &Path, args: &[String]) -> Vec<String> {
    let c = task.container.as_ref().expect("container task");
    let mut argv = vec![container_engine(c.engine.as_deref()), "run".to_string()];
    if c.rm {
        argv.push("--rm".into());
    }
    if c.detach {
        argv.push("--detach".into());
    } else if std::io::stdin().is_terminal() {
        argv.push("--interactive".into());
        if std::io::stdout().is_terminal() {
            argv.push("--tty".into());
        }
    }
    if let Some(name) = &c.name {
        argv.extend(["--name".into(), name.clone()]);
    }
    for port in &c.ports {
        argv.extend(["--publish".into(), port.clone()]);
    }
    for (key, value) in &c.env {
        argv.extend(["--env".into(), format!("{key}={value}")]);
    }
    for volume in &c.volumes {
        argv.extend(["--volume".into(), absolute_volume(volume, dir)]);
    }
    if let Some(workdir) = &c.workdir {
        argv.extend(["--workdir".into(), workdir.clone()]);
    }
    argv.extend(c.args.iter().cloned());
    argv.push(c.image.clone());
    if task.run.is_empty() {
        argv.extend(c.command.iter().cloned());
        argv.extend(args.iter().cloned());
    } else {
        let script = with_args(&task.run.join(" && "), args);
        argv.extend(["sh".into(), "-c".into(), script]);
    }
    argv
}

/// Container engines need absolute host paths: `./data:/data` -> `/project/data:/data`.
fn absolute_volume(volume: &str, dir: &Path) -> String {
    let (host, rest) = volume.split_once(':').unwrap_or((volume, ""));
    if host == "." || host.starts_with("./") || host.starts_with("../") {
        let host = match host.strip_prefix("./").unwrap_or(host) {
            "." | "" => dir.to_path_buf(),
            relative => dir.join(relative),
        };
        format!("{}:{rest}", host.display())
    } else {
        volume.to_string()
    }
}

fn container_engine(configured: Option<&str>) -> String {
    if let Some(engine) = configured {
        return engine.to_string();
    }
    if let Ok(engine) = std::env::var("R_CONTAINER_ENGINE")
        && !engine.is_empty()
    {
        return engine;
    }
    let installed = |name: &str| {
        std::env::var_os("PATH")
            .is_some_and(|path| std::env::split_paths(&path).any(|dir| dir.join(name).is_file()))
    };
    if !installed("docker") && installed("podman") {
        "podman".into()
    } else {
        "docker".into()
    }
}

/// Run the steps in order, stopping at the first failure. Returns the exit code.
pub fn run(steps: &[Step]) -> Result<i32> {
    for step in steps {
        let status = Command::new(&step.argv[0])
            .args(&step.argv[1..])
            .current_dir(&step.dir)
            .envs(&step.env)
            .status()
            .map_err(|e| match e.kind() {
                std::io::ErrorKind::NotFound => {
                    anyhow::anyhow!("`{}` not found, is it installed?", step.argv[0])
                }
                _ => anyhow::anyhow!("failed to run `{}`: {e}", step.argv[0]),
            })?;
        if !status.success() {
            return Ok(exit_code(status));
        }
    }
    Ok(0)
}

fn exit_code(status: std::process::ExitStatus) -> i32 {
    #[cfg(unix)]
    {
        use std::os::unix::process::ExitStatusExt;
        if let Some(signal) = status.signal() {
            return 128 + signal;
        }
    }
    status.code().unwrap_or(1)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn quoting() {
        assert_eq!(quote("build:prod"), "build:prod");
        assert_eq!(quote("a b"), "'a b'");
        assert_eq!(quote("it's"), "'it'\\''s'");
        assert_eq!(quote(""), "''");
    }

    #[test]
    fn tool_arguments() {
        let args = ["--watch".to_string()];
        assert_eq!(
            tool_argv(Kind::Package, "npm", "test", &args),
            ["npm", "run", "test", "--", "--watch"]
        );
        assert_eq!(
            tool_argv(Kind::Package, "yarn", "test", &args),
            ["yarn", "run", "test", "--watch"]
        );
        assert_eq!(tool_argv(Kind::Make, "make", "test", &[]), ["make", "test"]);
    }

    #[test]
    fn edit_distance() {
        assert_eq!(distance("biuld", "build"), 2);
        assert_eq!(distance("test", "test"), 0);
    }
}
