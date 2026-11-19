# Spec Delta

## Purpose

Defines how `r` provides tab completion in interactive shells, covering both its built-in commands and the task, runner and service names of the project the user is currently in.

## ADDED Requirements

### Requirement: Completion script generation
`r completions <shell>` SHALL print a completion script for the named shell to standard output. The shells `bash`, `zsh` and `fish` SHALL be supported. An unsupported shell name SHALL produce an error listing the supported shells and exit status 2.

#### Scenario: Bash script
- **WHEN** the user runs `r completions bash`
- **THEN** a script is printed that, once sourced in bash, enables completion for the `r` command

#### Scenario: Unsupported shell
- **WHEN** the user runs `r completions tcsh`
- **THEN** an error lists `bash`, `zsh` and `fish`, and the exit status is 2

#### Scenario: Works outside a project
- **WHEN** `r completions zsh` is run in a directory with no task source files
- **THEN** the script is still printed and the exit status is 0

### Requirement: First-argument completion
Completing the first argument SHALL offer the built-in commands, the runner names present in the project, and the names of all discovered tasks. Candidates SHALL reflect the project of the current directory at the time of completion, not at the time the script was generated.

#### Scenario: Task names offered
- **WHEN** the project has tasks `build` and `bench` and the user types `r b` and presses Tab
- **THEN** `build` and `bench` are offered

#### Scenario: Follows the current directory
- **WHEN** the user changes into a different project and presses Tab after `r `
- **THEN** the candidates are the tasks of the new project

#### Scenario: Descriptions
- **WHEN** completion runs in zsh or fish and a task has a description
- **THEN** the description is shown next to the candidate

### Requirement: Runner-scoped completion
After a runner name, completion SHALL offer only the tasks of that runner. After `run` and `which`, completion SHALL offer runner names and task names.

#### Scenario: Tasks of one runner
- **WHEN** the Makefile has `build` and `package.json` has `start`, and the user types `r make ` and presses Tab
- **THEN** only `build` is offered

### Requirement: Service name completion
After `up`, `down` and `logs`, completion SHALL offer the service names declared in `r.yaml`.

#### Scenario: Service names
- **WHEN** `r.yaml` declares services `db` and `cache` and the user types `r up ` and presses Tab
- **THEN** `db` and `cache` are offered

### Requirement: Completion is side-effect free
Producing completion candidates SHALL NOT execute any runner tool, task or container command, and SHALL NOT print errors to the terminal. When the project cannot be read, completion SHALL offer the built-in commands only.

#### Scenario: Broken project file
- **WHEN** `package.json` is malformed and the user presses Tab after `r `
- **THEN** built-in commands are offered and no error text appears in the terminal

#### Scenario: No tool is started
- **WHEN** the user presses Tab in a Gradle project
- **THEN** no Gradle process is started

### Requirement: Arguments after a task are not completed by r
Once a task name has been completed, `r` SHALL offer no candidates of its own for the following arguments, so that the shell's default file name completion applies.

#### Scenario: File completion after the task
- **WHEN** the user types `r test src/` and presses Tab
- **THEN** file names under `src/` are offered by the shell
