# Spec Delta

## Purpose

Defines the built-in subcommands of `r` for inspecting a project and running tasks explicitly, and how built-in names relate to task names found in the project.

## ADDED Requirements

### Requirement: Built-in command set
`r` SHALL provide the built-in commands `list`, `run`, `which`, `env`, `completions`, `up`, `down`, `ps`, `logs` and `help`, and the options `--help` and `--version`. Options of `r` itself SHALL only be recognised before the first non-option argument.

#### Scenario: Version
- **WHEN** the user runs `r --version`
- **THEN** the version of `r` is printed and the exit status is 0

#### Scenario: Help
- **WHEN** the user runs `r --help`
- **THEN** usage is printed that lists the built-in commands and explains `r <task>` and `r <runner> <task>`

### Requirement: First-argument precedence
The first non-option argument SHALL be interpreted as a built-in command if it matches one, otherwise as a runner name if it matches one, otherwise as a task name. A task hidden by this rule SHALL remain runnable through `r run <task>` or `r <runner> <task>`.

#### Scenario: Task named like a built-in
- **WHEN** `package.json` has a script called `list` and the user runs `r list`
- **THEN** the built-in `list` runs, not the script

#### Scenario: Reaching the hidden task
- **WHEN** `package.json` has a script called `list` and the user runs `r run list`
- **THEN** `npm run list` is executed

### Requirement: List tasks
`r list` SHALL print every discovered task grouped by runner, with its description when there is one. Names offered by more than one runner and names hidden by a built-in or runner name SHALL be visibly marked. Running `r` without arguments SHALL behave like `r list`.

#### Scenario: Grouped listing
- **WHEN** the project has a Makefile with `build` and a `package.json` with `start`
- **THEN** `r list` shows `build` under `make` and `start` under the package manager's runner name

#### Scenario: Duplicates are marked
- **WHEN** `test` is offered by both `make` and `npm`
- **THEN** both `test` entries are marked as ambiguous in the listing

#### Scenario: No arguments
- **WHEN** the user runs `r` with no arguments
- **THEN** the output is the same as `r list`

#### Scenario: Filter by runner
- **WHEN** the user runs `r list make`
- **THEN** only the tasks of the `make` runner are shown

### Requirement: Machine-readable listing
`r list --json` SHALL print the task inventory as a JSON array in which every entry has the task name, the runner name, the description (or null), the source file, and whether the name is ambiguous.

#### Scenario: JSON output
- **WHEN** the user runs `r list --json` in a project with one Makefile target `build`
- **THEN** standard output is a valid JSON array with one object whose name is `build` and whose runner is `make`

### Requirement: Explicit run command
`r run <task> [args...]` and `r run <runner> <task> [args...]` SHALL resolve and execute tasks with the same rules as the short forms, except that `<task>` is never interpreted as a built-in command.

#### Scenario: Run is equivalent to the short form
- **WHEN** only the Makefile offers `build`
- **THEN** `r run build` and `r build` execute the same command

### Requirement: Show what would run
`r which <task>` and `r which <runner> <task>` SHALL resolve the task with the same rules as running it and print the runner name, the source file and the exact command line, without executing anything. Resolution failures SHALL be reported exactly as when running.

#### Scenario: Resolved task
- **WHEN** only `package.json` (yarn) offers `start` and the user runs `r which start`
- **THEN** the output names runner `yarn`, the `package.json` path and the command `yarn run start`, and no task is started

#### Scenario: Ambiguous task
- **WHEN** `test` is offered by two runners and the user runs `r which test`
- **THEN** the same ambiguity error as for `r test` is printed and the exit status is 2
