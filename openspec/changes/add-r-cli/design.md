# Design

## Context

The repository contains only OpenSpec scaffolding; there is no code, build setup or CI yet. This change creates the whole program, so the design fixes the crate layout and the handful of choices that shape every later feature. Motivation and scope are in `proposal.md`; required behaviour is in `specs/`.

Constraints that drive the design:

- `r` is run interactively many times a day and on every Tab press, so start-up and discovery have to be fast (target: well under 50 ms for `list` and completion in a typical project).
- `r` reads files from projects the user may not fully trust to be well-formed. Reading them must never execute anything.
- The same resolution logic backs running, `which`, `list` and completion, so it must exist exactly once.

## Goals / Non-Goals

**Goals:**

- One static binary, no runtime dependencies beyond the tools a project actually uses.
- A runner abstraction that makes adding a format (justfile, Taskfile, Cargo aliases, Maven, ...) a new module plus a registry entry, with no change to resolution, listing or completion.
- Discovery that is purely static: parse files, never spawn.
- Behaviour that is testable end-to-end without `make`, Node, Gradle or Podman installed, by putting stub programs on `PATH`.

**Non-Goals:**

- Sandboxing. Isolation covers environment variables only; tasks keep full filesystem and network access.
- Replacing the underlying tools. `r npm install` is not a package manager proxy; `r <runner> <x>` always means "run task `x`".
- Monorepo and workspace awareness (npm workspaces, Gradle subproject enumeration, merging tasks from several directories).
- Windows support, PowerShell completion.
- Service orchestration beyond start/stop: no health checks, dependency ordering between services, networks or compose-file import.
- Packaging and publishing (crates.io, Homebrew, release binaries).

## Decisions

### 1. Single crate, binary `r`, modules by capability

```
Cargo.toml            package "r-runner", [[bin]] name = "r"
src/main.rs           argument dispatch, exit codes
src/cli.rs            clap definitions for built-ins and global options
src/project.rs        project directory lookup, loads config + runners
src/runner/mod.rs     Runner trait, Task, registry
src/runner/{make,node,gradle,custom}.rs
src/resolve.rs        name -> Resolution (Found | Ambiguous | NotFound)
src/exec.rs           building and spawning the command
src/env.rs            environment construction
src/config.rs         r.yaml and user config types + validation
src/services.rs       podman integration
src/completion.rs     script templates and the __complete entry point
tests/                integration tests driving the built binary
```

The package is not named `r` because that name is not usable on crates.io and is hard to search for; only the binary carries the short name. A workspace with a library crate was considered and rejected: nothing else consumes the logic yet, and splitting later is mechanical.

### 2. Runner trait with a completeness flag

```rust
trait Runner {
    fn name(&self) -> &'static str;                 // "make", "npm", ...
    fn detect(dir: &Path) -> Result<Option<Self>>;  // cheap file checks
    fn tasks(&self) -> Result<Vec<Task>>;           // static parse
    fn exhaustive(&self) -> bool;                   // false for make, gradle
    fn command(&self, task: &str, args: &[OsString]) -> CommandSpec;
}
```

`exhaustive()` is what lets `r gradle :app:bootRun` and `r make some-pattern-target` pass through while `r npm typo` fails early (spec: task-execution, "Unknown tasks"). `CommandSpec` is plain data (program, args, cwd), which is what `r which` prints and what tests assert on.

Node is one module producing one runner instance whose name is the selected package manager. Explicitly naming another package manager constructs the same runner with that name. Alternative considered: four separate runners all reading `package.json`; rejected because every script would then be ambiguous with itself.

### 3. Static discovery only

- **Makefile**: line-based scan for `name [name...]:` at column 0, skipping lines starting with a tab, `:=`/`::=`/`?=`/`+=` assignments, names containing `%` or `$`, and names starting with `.`. `## text` on the rule line becomes the description. `include`d files are not followed.
- **Gradle**: regex scan of the root `build.gradle(.kts)` for `tasks.register("x")`, `task x`, `task("x")`, and for applied plugins (`java`, `java-library`, `application`, `base`, `org.jetbrains.kotlin.jvm`, `org.springframework.boot`, ...) mapped to a built-in table of lifecycle tasks.
- **package.json**: `serde_json`, reading `scripts` and `packageManager` only.

Alternatives considered: `make -pRrq` and `gradle tasks --all` give exact lists, but both execute project code (`$(shell ...)`, build scripts), and Gradle takes seconds, which is unusable for Tab completion. A disk cache refreshed in the background was considered for Gradle and deferred; the `exhaustive = false` pass-through covers the gap for running, at the cost of incomplete listing.

### 4. First-argument dispatch before clap

`main` inspects the first non-option argument itself: built-in name → hand the whole line to clap; runner name → qualified run; anything else → task run. Everything after the task name is collected raw and never parsed (spec: "Flag that r also knows"). Global options (`-e`, `--pass-env`, `--inherit-env`) are parsed from the leading arguments only.

Alternative considered: a clap `external_subcommand` catch-all. It works, but the precedence rules and "options only before the task" become implicit in clap's behaviour instead of being ten explicit lines that the resolution tests cover.

### 5. Strict ambiguity, including custom commands

Resolution returns `Ambiguous` whenever two runners offer a name, with no priority order and no configuration to pick a winner. This is the behaviour requested. The known cost is that an `r.yaml` command wrapping a same-named script (`test` running `npm test` plus a linter) makes `r test` ambiguous; the workaround is a different name. Letting `custom` win, or a `prefer:` map in `r.yaml`, are both additive later and were left out to keep the first rule simple and predictable.

### 6. Exit codes

| Status | Meaning |
| --- | --- |
| task's own | task ran |
| 2 | `r` usage, resolution or configuration error |
| 127 | required program (`make`, `pnpm`, `podman`, ...) not found |

On Unix, a plain task run with no further work to do is started with `exec`, so signals, job control and the exit status belong to the tool directly. Custom commands with a `run` list, and anything after `needs`, use spawn-and-wait in the foreground process group so Ctrl-C reaches the child.

### 7. Environment construction

`env.rs` builds an ordered list of `(name, value, source)` and the child is always started with `env_clear()` followed by that list. Keeping the source is what makes `r env` a view of the same data rather than a second implementation.

- Isolation is on by default and cannot be disabled from `r.yaml`, only by `--inherit-env`. A project file that could switch it off would make the protection depend on reading every repository's config first.
- Pass rules come from three places with equal effect: `r.yaml` (`env.pass`), the user file `$XDG_CONFIG_HOME/r/config.yaml` (default `~/.config/r/config.yaml`, only `env.pass` is read), and `--pass-env`. The user file exists because things like `SSH_AUTH_SOCK`, `JAVA_HOME` or `NVM_*` are per-machine, not per-project.
- Patterns support `*` only, matched with a small hand-written matcher; no dependency on a glob crate.
- Dotenv files are parsed with `dotenvy`'s iterator API without touching the process environment. Entries are either a path string or `{path: .env.local, optional: true}`.
- No `${VAR}` interpolation in `env.set`. Interpolation would reintroduce a way to copy arbitrary caller variables under another name without a visible pass rule.

`podman` itself is started with `r`'s full environment, because it needs `XDG_RUNTIME_DIR`, `CONTAINERS_*`, registry auth and similar; it is a tool the user installed, not project code. Containers receive only the service's declared `env`, passed as explicit `--env NAME=VALUE` arguments.

### 8. `r.yaml` shape

```yaml
version: 1

env:
  pass: [JAVA_HOME, "NVM_*"]
  files: [.env, {path: .env.local, optional: true}]
  set:
    NODE_ENV: development

commands:
  dev:
    description: Start the app against local services
    run: npm run dev
    needs: [db]
    env:
      DATABASE_URL: postgres://postgres:dev@localhost:5432/app
  check:
    run:
      - cargo fmt --check
      - cargo test
    dir: tools

services:
  db:
    image: docker.io/library/postgres:16
    ports: ["5432:5432"]
    env:
      POSTGRES_PASSWORD: dev
    volumes: ["pgdata:/var/lib/postgresql/data"]
```

Types use `#[serde(deny_unknown_fields)]`, which gives the "unknown key" rejection for free and keeps the format free to grow. The YAML crate is `serde_yaml_ng` (the original `serde_yaml` is unmaintained); the choice is confined to `config.rs`.

Each `run` entry is executed as `sh -c <entry> r-<command> <args...>`, so arguments arrive as `"$@"` without `r` doing any quoting.

### 9. Podman integration through the CLI

`services.rs` shells out to `podman` (`run -d`, `start`, `stop`, `rm`, `ps --format json`, `logs`).

- **Name**: `r-<dirname>-<hash8>-<service>`, where `hash8` is the first 8 hex characters of a hash of the canonical project path.
- **Labels**: `dev.r.project=<canonical path>`, `dev.r.service=<name>`, `dev.r.config=<hash of the service definition>`. `down` and `ps` select by the project label; `up` compares the config label to decide between no-op, `start` and remove-and-recreate.
- Named volumes are prefixed the same way as containers; sources starting with `/`, `./` or `~` are bind mounts, resolved against the project directory.

Alternatives considered: the Podman REST API over its socket (needs the socket service running, more code, no benefit at this scale) and generating a compose file for `podman compose` (adds a second tool the user must install).

### 10. Completion through a hidden `__complete` command

`r completions <shell>` prints a small static script, embedded with `include_str!`, that calls `r __complete <cword> -- <words...>` and reads `candidate<TAB>description` lines. `__complete` reuses project loading and the runner registry, swallows every error, and prints built-ins only when the project is unreadable.

Alternative considered: `clap_complete`'s dynamic completion engine. It would save the three script templates, but it is still behind an unstable feature flag, and the grammar here (`r <task>`, `r <runner> <task>`) is the part clap does not model anyway.

### 11. Testing approach

Integration tests (`assert_cmd`, `tempfile`) build a temporary project directory and a temporary `bin/` with stub scripts named `make`, `npm`, `yarn`, `pnpm`, `bun`, `gradle`, `podman` that append their argv and environment to a log file. Tests run `r` with `PATH` set to that directory and assert on the log. This covers dispatch, argument forwarding, environment contents and Podman command sequences on any CI machine. Parsers get unit tests with fixture files. One optional test group, enabled by an environment variable, runs against a real `podman`.

## Risks / Trade-offs

- **zsh has a builtin called `r`** (re-run last command), which takes precedence over the binary → The zsh setup instructions and the generated zsh script's header comment say to add `disable r` before loading completion; the README covers it prominently. Renaming the binary was not considered because the name is part of the request.
- **`r` and R's `R` are the same file name on case-insensitive filesystems (macOS default)** → Document it; installation into a directory that precedes R's on `PATH` shadows R. No technical mitigation in this change.
- **Default isolation breaks tasks that rely on ambient variables** (`JAVA_HOME`, `NVM_DIR`, `SSH_AUTH_SOCK`, proxy settings) → The not-passed variables are easy to find with `r env`, `--inherit-env` is a one-word escape, and the user-level pass list fixes machine-specific cases once. Proxy variables (`HTTP_PROXY`, `HTTPS_PROXY`, `NO_PROXY`) are deliberately not in the baseline because they can carry credentials.
- **A project's `r.yaml` can pass any variable it wants (`env.pass: ["*"]`)** → Accepted. `r.yaml` is as trusted as the `Makefile` next to it; the feature protects against incidental exposure to tools and their dependencies, not against a hostile repository. `r env` shows what a project passes before anything runs.
- **Static Makefile and Gradle parsing misses or misreads tasks** (included makefiles, generated targets, tasks from convention plugins or subprojects) → Runner-qualified invocation passes unknown names through for these runners; listing is documented as best-effort for them.
- **Ambiguity between a custom wrapper and the script it wraps is annoying** → See decision 5; revisit with a `prefer` mechanism if it comes up in use.
- **Podman CLI output formats change between versions** → Only `--format json` output and exit codes are consumed; the minimum supported Podman version (4.x) is stated in the README.
- **`exec` path and spawn path could diverge** → Both go through the same `CommandSpec` and environment list; integration tests cover one runner on each path.

## Open Questions

- Should `r list` gain a `--refresh` mode that asks Gradle for its real task list and caches it? Additive; does not affect the task breakdown.
- Should later versions add fish-style abbreviations or a `r init` that writes a starter `r.yaml`? Additive.
