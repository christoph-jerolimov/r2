# Proposal

## Why

Every project uses a different command runner (`make`, `npm`/`yarn`/`pnpm`, Gradle, ad-hoc scripts), so developers have to remember per-project which tool starts, tests or builds things, and those tools inherit the full shell environment, including unrelated secrets such as cloud credentials and tokens. A single small command, `r`, that finds the project's tasks, dispatches to the right tool and runs it with a deliberately constructed environment removes both problems.

The repository is empty today, so this change introduces the first version of the tool.

## What Changes

- Add a Rust command-line program whose binary is called `r`.
- Discover tasks from the runner files in a project: `Makefile`, `package.json` (npm, yarn, pnpm, bun), Gradle build files, and `r`'s own `r.yaml`.
- `r <task> [args...]` runs the task with the tool that owns it (`make`, `npm run`, `yarn run`, `./gradlew`, ...), forwarding arguments and the exit code.
- When a task name is offered by more than one runner, `r` fails and lists the candidates; the user resolves it by naming the runner, e.g. `r npm start`.
- Built-in subcommands: `list`, `run`, `which`, `env`, `completions`, and the service commands `up`, `down`, `ps`, `logs`.
- Shell completion for bash, zsh and fish, including dynamic completion of task, runner and service names for the current project.
- A YAML project file, `r.yaml`, that defines custom commands, background services run as Podman containers, and environment rules.
- Environment isolation: every task runs with a new environment built from a small baseline plus explicitly allowed or defined variables, instead of inheriting the caller's whole environment.

Assumptions made where the request left room (each is revisited in `design.md`):

- Isolation is on by default, with an opt-out flag and allow-lists.
- "etc." for runner formats means the four listed above in this first version; the design keeps adding more a contained change.
- "etc." for shells means bash, zsh and fish.
- A custom command in `r.yaml` that has the same name as a discovered task is ambiguous like any other duplicate; it does not silently win.
- Linux and macOS are supported; Windows is out of scope for this change.

## Capabilities

### New Capabilities

- `task-discovery`: Which files `r` recognises as task sources, how the project directory is located, and which tasks each source contributes.
- `task-execution`: Resolving a requested name to exactly one task, ambiguity and not-found failures, runner-qualified invocation, argument forwarding and exit codes.
- `cli-commands`: The built-in subcommands for inspecting a project (`list`, `which`, `run`), their precedence over task names, and output formats.
- `shell-completion`: Generating completion scripts and completing task, runner and service names dynamically.
- `project-config`: The `r.yaml` file format, its validation, and custom commands defined in it.
- `container-services`: Declaring background services in `r.yaml` and managing them as Podman containers (`up`, `down`, `ps`, `logs`, start-on-demand).
- `env-isolation`: How the environment of a task is constructed, what is passed through, how values are defined, and how the result can be inspected.

### Modified Capabilities

None. There are no existing specs.

## Impact

- **Code**: new Rust crate at the repository root (`Cargo.toml`, `src/`, `tests/`); nothing existing is modified.
- **Dependencies**: Rust toolchain to build; crates for argument parsing, JSON/YAML parsing and dotenv parsing.
- **Runtime requirements**: the underlying tools (`make`, a Node package manager, Gradle or a Gradle wrapper) are needed only for the tasks that use them; `podman` is needed only for services.
- **Users**: the name `r` collides with a zsh builtin (`r` re-runs the last command) and, on case-insensitive filesystems, with the R language's `R` binary. Setup documentation has to cover both.
- **Security**: `r` executes commands defined by project files, exactly as `make` or `npm run` do. Isolation limits what those commands can read from the environment; it is not a sandbox.
