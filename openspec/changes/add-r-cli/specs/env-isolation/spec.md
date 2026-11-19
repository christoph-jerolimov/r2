# Spec Delta

## Purpose

Defines the environment that tasks started by `r` receive, so that secrets and other unrelated variables in the caller's shell are not exposed to project commands unless that is explicitly allowed.

## ADDED Requirements

### Requirement: Tasks get a new environment
Every task `r` runs, for every runner, SHALL be started with a newly constructed environment. A variable of the caller's environment SHALL be present only if it is in the baseline, is allowed by a pass rule, or is defined by configuration or on the command line.

#### Scenario: Secret is not inherited
- **WHEN** the caller's shell has `GITHUB_TOKEN` set, no rule passes it, and the user runs a task that prints its environment
- **THEN** `GITHUB_TOKEN` does not appear in the output

#### Scenario: Applies to every runner
- **WHEN** the same unlisted variable is set and the user runs a Makefile target, an npm script, a Gradle task and a custom command
- **THEN** none of them sees the variable

### Requirement: Baseline variables
The baseline SHALL consist of `PATH`, `HOME`, `USER`, `LOGNAME`, `SHELL`, `TERM`, `COLORTERM`, `LANG`, `LC_*`, `TZ`, `TMPDIR`, `NO_COLOR`, `FORCE_COLOR`, `CI` and `XDG_*`, each passed with the caller's value when set. `r` SHALL additionally set `R_PROJECT_DIR` to the project directory and `R_TASK` to the task name.

#### Scenario: PATH is available
- **WHEN** a task runs a program found through the caller's `PATH`
- **THEN** the program is found

#### Scenario: Task metadata
- **WHEN** a custom command prints `$R_TASK`
- **THEN** the command's own name is printed

### Requirement: Pass rules
Additional variables SHALL be passed from the caller's environment when their name matches a pass rule. A rule is an exact name or a pattern with `*` wildcards. Rules SHALL be taken from `env.pass` in `r.yaml`, from `env.pass` in the user's own configuration file, and from repeated `--pass-env <rule>` options. A rule that matches no set variable SHALL NOT be an error.

#### Scenario: Project rule
- **WHEN** `r.yaml` has `env.pass: [JAVA_HOME]` and `JAVA_HOME` is set in the caller's shell
- **THEN** tasks see `JAVA_HOME` with the caller's value

#### Scenario: Wildcard rule
- **WHEN** a rule `NVM_*` is in effect and `NVM_DIR` and `NVM_BIN` are set
- **THEN** tasks see both variables

#### Scenario: One-off rule
- **WHEN** the user runs `r --pass-env SSH_AUTH_SOCK deploy`
- **THEN** the `deploy` task sees `SSH_AUTH_SOCK` for this invocation only

#### Scenario: User-level rule
- **WHEN** the user's configuration file has `env.pass: [SSH_AUTH_SOCK]`
- **THEN** tasks in every project see `SSH_AUTH_SOCK`

### Requirement: Defined variables
Variables SHALL be definable in `r.yaml` under `env.set` (mapping), through `env.files` (list of dotenv files relative to the project directory), per custom command under its `env`, and on the command line with repeated `-e NAME=VALUE`. Values SHALL be taken literally, without expansion. A listed dotenv file that does not exist SHALL be an error unless its entry is marked optional.

#### Scenario: Project-wide value
- **WHEN** `r.yaml` has `env.set: {NODE_ENV: development}`
- **THEN** every task sees `NODE_ENV=development`

#### Scenario: Dotenv file
- **WHEN** `r.yaml` has `env.files: [.env]` and `.env` contains `API_URL=http://localhost:8080`
- **THEN** every task sees `API_URL` with that value

#### Scenario: Missing dotenv file
- **WHEN** `env.files` lists `.env.local` without marking it optional and the file does not exist
- **THEN** `r` fails naming the file and exits with status 2

#### Scenario: Command-specific value
- **WHEN** only the command `serve` has `env: {PORT: "3000"}`
- **THEN** `serve` sees `PORT=3000` and other tasks do not

### Requirement: Precedence
When the same variable comes from several sources, the later source in this order SHALL win: baseline and pass rules, `env.files` in listed order, `env.set`, the custom command's `env`, `-e` on the command line.

#### Scenario: Command line overrides configuration
- **WHEN** `env.set` has `PORT: "3000"` and the user runs `r -e PORT=4000 serve`
- **THEN** the task sees `PORT=4000`

#### Scenario: Configuration overrides the caller
- **WHEN** the caller's shell has `LANG=de_DE.UTF-8` and `env.set` has `LANG: C`
- **THEN** the task sees `LANG=C`

### Requirement: Opting out
`r --inherit-env` SHALL start the task with the caller's complete environment, with defined variables still applied on top. Isolation SHALL NOT be switchable off from `r.yaml`.

#### Scenario: Inherit for one invocation
- **WHEN** the caller's shell has `GITHUB_TOKEN` set and the user runs `r --inherit-env release`
- **THEN** the task sees `GITHUB_TOKEN`

#### Scenario: Project file cannot disable isolation
- **WHEN** `r.yaml` contains a key that attempts to turn isolation off
- **THEN** the file is rejected as containing an unknown key

### Requirement: Inspect the environment
`r env [task]` SHALL print the names of the variables a task would receive, one per line with the source of each (baseline, pass rule, file, set, command, command line), without running anything. Values SHALL be shown only with `--show-values`. Without a task it SHALL describe the environment common to all tasks.

#### Scenario: Names and sources
- **WHEN** the user runs `r env serve`
- **THEN** each variable `serve` would receive is listed with its source, and no values are printed

#### Scenario: Values on request
- **WHEN** the user runs `r env --show-values serve`
- **THEN** each variable is listed with its value

#### Scenario: Blocked variables are not listed
- **WHEN** the caller's shell has `GITHUB_TOKEN` set and no rule passes it
- **THEN** `GITHUB_TOKEN` does not appear in the output of `r env`
