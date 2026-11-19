# Spec Delta

## Purpose

Defines the `r.yaml` project file: where it lives, how it is validated, and how custom commands are declared in it. Services and environment rules use the same file and are specified in their own capabilities.

## ADDED Requirements

### Requirement: Project file
`r` SHALL read a file named `r.yaml` in the project directory if one exists. The file is a YAML mapping with the optional top-level keys `version`, `commands`, `services` and `env`. An `r.yaml` SHALL count as a task source file for locating the project directory. All other behaviour SHALL work without the file.

#### Scenario: Project without r.yaml
- **WHEN** the project directory contains only a `Makefile`
- **THEN** `r` works and reports no error about a missing `r.yaml`

#### Scenario: r.yaml alone makes a project
- **WHEN** a directory contains only an `r.yaml` with one command
- **THEN** that directory is the project directory and the command is listed

### Requirement: Validation
`r` SHALL reject an `r.yaml` that is not valid YAML, has an unsupported `version`, contains unknown keys, or has values of the wrong type. The error SHALL name the file, the location of the problem and what was expected, and the exit status SHALL be 2. The only supported `version` is `1`; an absent `version` SHALL be treated as `1`.

#### Scenario: Unknown key
- **WHEN** `r.yaml` contains a top-level key `comands`
- **THEN** `r` fails with an error that names `comands` as an unknown key, and exits with status 2

#### Scenario: Newer version
- **WHEN** `r.yaml` contains `version: 2`
- **THEN** `r` fails saying this version of `r` supports version 1 only

#### Scenario: Invalid file blocks everything in the project
- **WHEN** `r.yaml` is invalid and the user runs `r build` for a Makefile target
- **THEN** `r` reports the `r.yaml` error and does not run the target

### Requirement: Custom command definition
Each key under `commands` SHALL define a custom command with a required `run` (a string, or a list of strings) and optional `description`, `dir`, `env` and `needs`. Command names SHALL consist of letters, digits, `-`, `_`, `.` and `:` and SHALL NOT start with `-`.

#### Scenario: Minimal command
- **WHEN** `r.yaml` contains a command `hello` with `run: echo hello` and the user runs `r hello`
- **THEN** `hello` is printed and the exit status is 0

#### Scenario: Missing run
- **WHEN** a command is defined without `run`
- **THEN** `r` rejects the file, naming the command

#### Scenario: Invalid name
- **WHEN** a command is named `--force`
- **THEN** `r` rejects the file, naming the command

### Requirement: Command execution
A custom command's `run` SHALL be executed by `sh`. When `run` is a list, the entries SHALL be executed in order, stopping at the first one that fails, and the command's exit status SHALL be that of the last entry executed. Extra command-line arguments SHALL be available to every entry as positional parameters.

#### Scenario: Shell features
- **WHEN** a command has `run: echo one && echo two`
- **THEN** both lines are printed

#### Scenario: List stops on failure
- **WHEN** a command has `run: ["false", "echo unreachable"]`
- **THEN** `unreachable` is not printed and the exit status is non-zero

#### Scenario: Positional parameters
- **WHEN** a command has `run: echo "$1"` and the user runs `r greet world`
- **THEN** `world` is printed

### Requirement: Working directory
A custom command SHALL run in the project directory, or in `dir` when given. `dir` SHALL be interpreted relative to the project directory and SHALL NOT resolve to a location outside it.

#### Scenario: Subdirectory
- **WHEN** a command has `dir: web` and `run: pwd`
- **THEN** the printed path is the `web` directory inside the project directory

#### Scenario: Escaping the project
- **WHEN** a command has `dir: ../other`
- **THEN** `r` rejects the file

### Requirement: Command dependencies on services
A custom command MAY list service names under `needs`. Every listed name SHALL refer to a service declared in the same file.

#### Scenario: Unknown service
- **WHEN** a command lists `needs: [db]` and no service `db` is declared
- **THEN** `r` rejects the file, naming the command and the missing service
