# Tasks

## 1. Project scaffolding

- [ ] 1.1 Create `Cargo.toml` (package `r-runner`, `[[bin]] name = "r"`, edition 2021) with dependencies `clap`, `serde`, `serde_json`, `serde_yaml_ng`, `dotenvy`, `anyhow`, `thiserror` and dev-dependencies `assert_cmd`, `predicates`, `tempfile`; verify `cargo build` succeeds and produces `target/debug/r`
- [ ] 1.2 Create the module skeleton from design decision 1 (`main.rs`, `cli.rs`, `project.rs`, `runner/`, `resolve.rs`, `exec.rs`, `env.rs`, `config.rs`, `services.rs`, `completion.rs`) and a `.gitignore` for `target/`; verify `cargo clippy -- -D warnings` and `cargo fmt --check` pass
- [ ] 1.3 Add `tests/common/mod.rs` with a test harness that creates a temp project directory and a temp `bin/` of stub programs (`make`, `npm`, `yarn`, `pnpm`, `bun`, `gradle`, `podman`) logging argv and environment to a file, and runs `r` with `PATH` restricted to it; verify with a smoke test that a stub invocation is recorded
- [ ] 1.4 Add a CI workflow running `cargo fmt --check`, `cargo clippy -- -D warnings` and `cargo test` on Linux and macOS; verify the workflow file passes a YAML lint and the three commands pass locally

## 2. Task discovery

- [ ] 2.1 Define `Task`, `CommandSpec` and the `Runner` trait with `exhaustive()` and a registry listing all runners in a fixed order; verify with a unit test that the registry reports the names `make`, `npm`, `yarn`, `pnpm`, `bun`, `gradle`, `custom`
- [ ] 2.2 Implement project directory lookup (nearest ancestor containing any task source file, error with exit 2 when none); verify with tests for project root, nested subdirectory, nearest-wins and no-source cases
- [ ] 2.3 Implement the Makefile parser (explicit targets, `##` descriptions, skipping pattern rules, dot-targets, assignments, recipe lines); verify with fixture-based unit tests covering each skip rule and multi-target lines
- [ ] 2.4 Implement the `package.json` reader and package manager selection (`packageManager` field, single lockfile, default npm, conflicting lockfiles marked undetermined, malformed JSON error); verify with one unit test per scenario in `specs/task-discovery`
- [ ] 2.5 Implement the Gradle scanner (registered tasks in the root build file plus the plugin-to-lifecycle-task table, `gradlew` detection); verify with Groovy and Kotlin DSL fixtures and a harness test asserting that `r list` records no `gradle`/`gradlew` invocation
- [ ] 2.6 Document supported files, the best-effort nature of Makefile and Gradle listing, and package manager selection in `README.md`; verify every documented file name is covered by a discovery test

## 3. Resolution and execution

- [ ] 3.1 Implement `resolve` returning `Found`, `Ambiguous` (with all candidates) or `NotFound` (with close-name suggestions), plus the runner-qualified variant honouring `exhaustive()`; verify with unit tests for unique, duplicate, unknown, qualified-duplicate, qualified-unknown-on-npm and qualified-unknown-on-gradle
- [ ] 3.2 Implement first-argument dispatch in `main` (built-in, then runner, then task; `r` options only before the first non-option argument; everything after the task collected raw); verify with harness tests that `r build --help` forwards `--help` and that `r make` lists make tasks
- [ ] 3.3 Implement `command()` for each runner (`make <t>`, `<pm> run <t>` with `--` for npm, `./gradlew` or `gradle`) and argument forwarding; verify with harness tests asserting the exact argv logged by each stub, including `r test --watch` and `r build V=1`
- [ ] 3.4 Implement execution: project directory as working directory, inherited stdio, `exec` on Unix for single commands, exit status propagation, status 127 with the program name when the tool is missing, status 2 for `r`'s own errors; verify with harness tests for a stub exiting 3, a removed stub, and running from a subdirectory
- [ ] 3.5 Implement the ambiguity and not-found error messages with runner-qualified suggestions; verify with tests asserting stderr contains `r make test` and `r npm test` for a duplicated `test`, and that no stub was invoked
- [ ] 3.6 Document `r <task>`, `r <runner> <task>`, ambiguity handling, argument forwarding and exit codes in `README.md`; verify each documented example is exercised by a test

## 4. Built-in commands

- [ ] 4.1 Define the clap command tree for `list`, `run`, `which`, `env`, `completions`, `up`, `down`, `ps`, `logs`, `help` and the global options `-e`, `--pass-env`, `--inherit-env`, `--help`, `--version`; verify `r --help` and `r --version` output with snapshot-style tests
- [ ] 4.2 Implement `r list` (grouped by runner, descriptions, ambiguity and shadowed-name markers, optional runner filter, `r` with no arguments) and `r list --json`; verify with tests on human output markers and by parsing the JSON output
- [ ] 4.3 Implement `r run` (never treats the task as a built-in) and `r which` (prints runner, source file and command line, runs nothing, same errors as running); verify with tests for a script named `list`, for `r which start`, and for `r which` on an ambiguous name

## 5. Project config and custom commands

- [ ] 5.1 Implement the `r.yaml` types with `deny_unknown_fields`, version handling and validation (name rules, required `run`, `dir` inside the project, `needs` referring to declared services, `ports`/`volumes` syntax) with errors naming file and location; verify with one test per rejection scenario in `specs/project-config` and that an invalid file blocks `r build`
- [ ] 5.2 Implement the `custom` runner: tasks from `commands`, `sh -c` execution with positional parameters, `run` lists stopping at first failure, `dir` handling; verify with harness tests for `echo "$1"`, a failing list and `dir: web`
- [ ] 5.3 Add an annotated example `r.yaml` under `examples/` and a "Project file" section in `README.md`; verify with a test that loads the example file through the real validator

## 6. Environment isolation

- [ ] 6.1 Implement environment construction as an ordered `(name, value, source)` list: baseline, `R_PROJECT_DIR`/`R_TASK`, pass rules with `*` matching, dotenv files (required and optional), `env.set`, command `env`, `-e`, and `--inherit-env`; verify with unit tests for each source and for the precedence order in `specs/env-isolation`
- [ ] 6.2 Load user-level pass rules from `$XDG_CONFIG_HOME/r/config.yaml` (default `~/.config/r/config.yaml`); verify with a test that sets `XDG_CONFIG_HOME` to a temp directory
- [ ] 6.3 Apply the constructed environment to every runner via `env_clear()`; verify with harness tests that a variable set in the caller is absent from the stub's logged environment for make, npm, gradle and custom, and present with `--pass-env` and `--inherit-env`
- [ ] 6.4 Implement `r env [task]` with sources, `--show-values`, and no values by default; verify with tests that blocked variables are not listed and that values appear only with the flag
- [ ] 6.5 Document the baseline, pass rules, the user config file, dotenv handling, `--inherit-env`, and the "not a sandbox" limitation in `README.md`; verify the documented baseline list equals the constant in `env.rs` with a test

## 7. Container services

- [ ] 7.1 Implement container naming, project/service/config labels and translation of a service definition into `podman run -d` arguments (image, command, ports, declared env only, named volumes and bind mounts); verify with unit tests on the generated argument list and that two project paths yield different names
- [ ] 7.2 Implement `r up` with idempotent behaviour (running and unchanged: no-op; stopped: start; changed: remove and recreate), unknown-service error and Podman error passthrough; verify with harness tests driving a stub `podman` that returns canned `ps --format json` output for each state
- [ ] 7.3 Implement `r down` (label-selected, includes undeclared leftovers, keeps volumes, no error when nothing runs), `r ps` with `--json`, and `r logs` with `-f`; verify with harness tests asserting the stub's logged commands and the reported states
- [ ] 7.4 Start `needs` services before a custom command and skip the command if a service fails to start; verify with harness tests for the success order (podman calls logged before the command) and the failure case
- [ ] 7.5 Report missing Podman with status 127 for service features only; verify that with no `podman` stub `r up` exits 127 while a Makefile target in the same project still runs
- [ ] 7.6 Add an opt-in test group (enabled by `R_TEST_PODMAN=1`) that runs `up`, `ps`, `logs`, `down` against a real Podman with a small image, and document services and the minimum Podman version in `README.md`; verify the group passes on a machine with Podman and is skipped otherwise

## 8. Shell completion

- [ ] 8.1 Implement the hidden `r __complete <cword> -- <words...>` command producing `candidate<TAB>description` lines for first argument, after a runner, after `run`/`which`, and after `up`/`down`/`logs`, with no output after a completed task name; verify with tests for each position
- [ ] 8.2 Make `__complete` side-effect free and silent (no spawned tools, empty stderr, built-ins only when the project is unreadable); verify with a malformed `package.json` and by asserting the stub log is empty
- [ ] 8.3 Write the bash, zsh and fish script templates and `r completions <shell>` (works outside a project, error listing supported shells otherwise); verify each generated script with the shell's syntax check (`bash -n`, `zsh -n`, `fish -n`) in tests that are skipped when the shell is not installed
- [ ] 8.4 Add an interactive-style test per available shell that sources the generated script and asks the shell for completions of `r b` in a fixture project; verify `build` is among the results
- [ ] 8.5 Document installation for each shell in `README.md`, including `disable r` for zsh and the `R`/`r` name clash on case-insensitive filesystems; verify the documented zsh snippet works in a clean `zsh -f` session

## 9. Integration checks

- [ ] 9.1 Add an end-to-end test on a fixture project containing a Makefile, `package.json`, Gradle build file and `r.yaml` together, covering list, a unique task, an ambiguous task, its qualified resolution and environment isolation in one flow; verify the test passes
- [ ] 9.2 Measure `r list` and `r __complete` on that fixture with a release build and confirm both complete in under 50 ms; record the numbers in the pull request description
- [ ] 9.3 Run `openspec validate add-r-cli --strict` and `cargo test` on a clean checkout; verify both pass
