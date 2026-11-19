mod completions;
mod exec;
mod sources;

use anyhow::{Context, Result, bail};
use sources::{Kind, Project, Source};
use std::path::PathBuf;

const HELP: &str = "\
r - one command for every task runner

Usage:
  r                         List all available tasks
  r <task> [args...]        Run a task with the tool that defines it
  r <runner> <task> [args]  Run a task with a specific runner (r npm start)
  r <command>

Runners:
  make    Makefile            npm, yarn, pnpm, bun   package.json
  just    justfile            task                   Taskfile.yml
  cargo   Cargo.toml          r                      r.yaml, run.yaml

Commands:
  list [runner] [--json]    List tasks (alias: ls)
  run <task> [args...]      Run a task, even if it is named like a command
  which <task> [args...]    Show the command a task would run
  init                      Create an r.yaml with examples
  completions <shell>       Print a completion script (bash, zsh, fish)
  help                      Show this help
  version                   Show the version

Options:
  -n, --dry-run             Print the commands instead of running them
  -C, --dir <dir>           Run as if started in <dir>
  -h, --help                Show this help
  -V, --version             Show the version

A task defined by more than one runner is ambiguous and r refuses to guess:
use `r <runner> <task>`, or define the task in r.yaml, which always wins.
Runner files are looked up in the current directory, then in its parents.
";

const COMMANDS: [(&str, &str); 7] = [
    ("list", "List all available tasks"),
    ("run", "Run a task"),
    ("which", "Show the command a task would run"),
    ("init", "Create an r.yaml with examples"),
    ("completions", "Print a shell completion script"),
    ("help", "Show help"),
    ("version", "Show the version"),
];

fn main() {
    let code = match real_main() {
        Ok(code) => code,
        Err(err) => {
            eprintln!("r: {err:#}");
            1
        }
    };
    std::process::exit(code);
}

struct Options {
    dry_run: bool,
    dir: PathBuf,
}

fn real_main() -> Result<i32> {
    let mut args: Vec<String> = std::env::args().skip(1).collect();
    let mut opts = Options {
        dry_run: false,
        dir: std::env::current_dir()?,
    };

    // Global options are only recognised before the first word; everything
    // after the task name belongs to the task.
    while let Some(first) = args.first().filter(|a| a.starts_with('-')).cloned() {
        args.remove(0);
        match first.as_str() {
            "-h" | "--help" => return help(),
            "-V" | "--version" => return version(),
            "-n" | "--dry-run" => opts.dry_run = true,
            "-C" | "--dir" => {
                if args.is_empty() {
                    bail!("{first} needs a directory");
                }
                opts.dir = std::path::absolute(args.remove(0))?;
            }
            "--" => break,
            _ => bail!("unknown option `{first}`, see `r --help`"),
        }
    }

    let Some(first) = args.first().cloned() else {
        return list(&opts, &[]);
    };
    let rest = &args[1..];
    match first.as_str() {
        "help" => help(),
        "version" => version(),
        "list" | "ls" => list(&opts, rest),
        "init" => init(&opts),
        "completions" => completions(rest),
        "__complete" => complete(&opts, rest),
        "which" => {
            opts.dry_run = true;
            run_task(&opts, rest, true)
        }
        "run" => run_task(&opts, rest, false),
        _ => run_task(&opts, &args, true),
    }
}

fn help() -> Result<i32> {
    print!("{HELP}");
    Ok(0)
}

fn version() -> Result<i32> {
    println!("r {}", env!("CARGO_PKG_VERSION"));
    Ok(0)
}

fn load(opts: &Options) -> Result<Project> {
    sources::discover(&opts.dir)?.with_context(|| {
        format!(
            "no runner file (Makefile, package.json, justfile, Taskfile.yml, Cargo.toml, r.yaml) \
             found in {} or its parent directories",
            opts.dir.display()
        )
    })
}

/// `r [<runner>] <task> [args...]`. Runner names are only recognised when
/// `allow_selector` is set (`r run make` always means the task called make).
fn run_task(opts: &Options, args: &[String], allow_selector: bool) -> Result<i32> {
    let Some(first) = args.first() else {
        bail!("missing task name, see `r list`");
    };
    let project = load(opts)?;

    let selector = Kind::from_selector(first).filter(|_| allow_selector);
    let steps = match selector.map(|kind| (kind, project.source(kind))) {
        Some((_, Some(source))) => {
            let Some(name) = args.get(1) else {
                print_sources(&project, &[source]);
                return Ok(0);
            };
            // With an explicit runner name the tool is called as named: `r npm start`
            // uses npm, even when a yarn.lock is present.
            let tool = if source.kind == Kind::Custom {
                &source.tool
            } else {
                first
            };
            match source.find(name) {
                Some(task) => {
                    exec::plan(&project, source, task, Some(tool), task_args(&args[2..]))?
                }
                None if source.kind == Kind::Custom => {
                    bail!("no task named `{name}` in {}", source.file_name())
                }
                // Not a task we know: hand everything to the tool (`r npm install`).
                None => vec![exec::passthrough(&project, tool, &args[1..])],
            }
        }
        // A runner name without its file, unless some task is called like that.
        Some((kind, None)) if !project.sources.iter().any(|s| s.find(first).is_some()) => {
            bail!(
                "no {} found in {}",
                kind.file_hint(),
                project.root.display()
            )
        }
        _ => {
            let found = exec::resolve(&project, first)?;
            exec::plan(
                &project,
                found.source,
                found.task,
                None,
                task_args(&args[1..]),
            )?
        }
    };

    if opts.dry_run {
        for step in &steps {
            println!("{}", step.display());
        }
        return Ok(0);
    }
    exec::run(&steps)
}

/// `r test -- --watch` and `r test --watch` are the same.
fn task_args(args: &[String]) -> &[String] {
    match args.first() {
        Some(first) if first == "--" => &args[1..],
        _ => args,
    }
}

fn list(opts: &Options, args: &[String]) -> Result<i32> {
    let json = args.iter().any(|a| a == "--json");
    let filters: Vec<&String> = args.iter().filter(|a| *a != "--json").collect();
    let project = load(opts)?;

    let mut selected: Vec<&Source> = Vec::new();
    for filter in &filters {
        let kind = Kind::from_selector(filter)
            .with_context(|| format!("unknown runner `{filter}`, see `r --help`"))?;
        let source = project.source(kind).with_context(|| {
            format!(
                "no {} found in {}",
                kind.file_hint(),
                project.root.display()
            )
        })?;
        selected.push(source);
    }
    if filters.is_empty() {
        selected = project.sources.iter().collect();
    }

    if json {
        let tasks: Vec<serde_json::Value> = selected
            .iter()
            .flat_map(|source| {
                source.tasks.iter().map(|task| {
                    serde_json::json!({
                        "name": task.name,
                        "runner": source.tool,
                        "file": source.file_name(),
                        "description": task.description,
                        "ambiguous": exec::resolve(&project, &task.name).is_err(),
                    })
                })
            })
            .collect();
        println!("{}", serde_json::to_string_pretty(&tasks)?);
    } else {
        print_sources(&project, &selected);
    }
    Ok(0)
}

fn print_sources(project: &Project, selected: &[&Source]) {
    let width = selected
        .iter()
        .flat_map(|s| &s.tasks)
        .map(|t| t.name.chars().count())
        .max()
        .unwrap_or(0);
    let mut ambiguous: Vec<&str> = Vec::new();
    for (i, source) in selected.iter().enumerate() {
        if i > 0 {
            println!();
        }
        println!("{} ({})", source.tool, source.file_name());
        if source.tasks.is_empty() {
            println!("  (no tasks)");
        }
        for task in &source.tasks {
            // Shadowed by r.yaml or defined by several runners?
            let direct = exec::resolve(project, &task.name)
                .is_ok_and(|found| found.source.kind == source.kind);
            let mark = if direct { ' ' } else { '*' };
            if !direct && !ambiguous.contains(&task.name.as_str()) {
                ambiguous.push(&task.name);
            }
            let description = task.description.as_deref().unwrap_or_default();
            let description = description.lines().next().unwrap_or_default();
            println!(
                "{}",
                format!("{mark} {:width$}  {description}", task.name).trim_end()
            );
        }
    }
    if !ambiguous.is_empty() {
        println!(
            "\n* defined more than once, use `r <runner> <task>`: {}",
            ambiguous.join(", ")
        );
    }
}

fn init(opts: &Options) -> Result<i32> {
    if let Some(existing) = sources::custom::FILE_NAMES
        .iter()
        .find(|n| opts.dir.join(n).exists())
    {
        bail!("{existing} already exists");
    }
    let file = opts.dir.join("r.yaml");
    std::fs::write(&file, sources::custom::TEMPLATE)
        .with_context(|| format!("failed to write {}", file.display()))?;
    println!("Created {}", file.display());
    Ok(0)
}

fn completions(args: &[String]) -> Result<i32> {
    let shells = completions::SHELLS.join(", ");
    let Some(shell) = args.first() else {
        bail!("missing shell, one of: {shells}\nExample: eval \"$(r completions bash)\"");
    };
    let script = completions::script(shell)
        .with_context(|| format!("unsupported shell `{shell}`, one of: {shells}"))?;
    print!("{script}");
    Ok(0)
}

/// Hidden command used by the completion scripts. Gets the words before the
/// cursor and prints one `name<TAB>description` candidate per line.
fn complete(opts: &Options, words: &[String]) -> Result<i32> {
    let mut out: Vec<(String, String)> = Vec::new();
    let project = sources::discover(&opts.dir).ok().flatten();
    let sources: &[Source] = project
        .as_ref()
        .map(|p| p.sources.as_slice())
        .unwrap_or_default();

    let all_tasks = |out: &mut Vec<(String, String)>| {
        for source in sources {
            for task in &source.tasks {
                if out.iter().any(|(name, _)| *name == task.name) {
                    continue;
                }
                let description = task.description.as_deref().unwrap_or_default();
                let description = description.lines().next().unwrap_or_default();
                out.push((
                    task.name.clone(),
                    format!("[{}] {description}", source.tool),
                ));
            }
        }
    };
    let source_tasks = |out: &mut Vec<(String, String)>, kind: Kind| {
        for task in sources
            .iter()
            .filter(|s| s.kind == kind)
            .flat_map(|s| &s.tasks)
        {
            out.push((
                task.name.clone(),
                task.description.clone().unwrap_or_default(),
            ));
        }
    };
    let runners = |out: &mut Vec<(String, String)>| {
        for source in sources {
            for selector in source.kind.selectors() {
                // Offer the detected package manager only, not all four.
                if source.kind != Kind::Package || *selector == source.tool {
                    out.push((
                        selector.to_string(),
                        format!("Tasks from {}", source.file_name()),
                    ));
                }
            }
        }
    };

    // Skip global options
    let mut words = words;
    while let Some(first) = words.first().filter(|w| w.starts_with('-')) {
        let skip = if first == "-C" || first == "--dir" {
            2
        } else {
            1
        };
        words = &words[skip.min(words.len())..];
    }

    match words {
        [] => {
            all_tasks(&mut out);
            runners(&mut out);
            for (name, description) in COMMANDS {
                out.push((name.to_string(), description.to_string()));
            }
        }
        [command] => match command.as_str() {
            "run" => all_tasks(&mut out),
            "which" => {
                all_tasks(&mut out);
                runners(&mut out);
            }
            "list" | "ls" => runners(&mut out),
            "completions" => out.extend(
                completions::SHELLS
                    .iter()
                    .map(|s| (s.to_string(), String::new())),
            ),
            other => {
                if let Some(kind) = Kind::from_selector(other) {
                    source_tasks(&mut out, kind);
                }
            }
        },
        [which, runner] if which == "which" => {
            if let Some(kind) = Kind::from_selector(runner) {
                source_tasks(&mut out, kind);
            }
        }
        _ => {} // task arguments: let the shell complete file names
    }

    let mut seen = std::collections::HashSet::new();
    for (name, description) in out {
        if seen.insert(name.clone()) {
            println!("{name}\t{}", description.trim_end());
        }
    }
    Ok(0)
}
