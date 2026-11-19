# Spec Delta

## Purpose

Defines how `r` turns a requested name into exactly one task and runs it with the tool that owns it, including what happens when the name is missing or offered by several runners.

## ADDED Requirements

### Requirement: Run a task by name
`r <task> [args...]` SHALL run the task when exactly one runner in the project offers a task with that name, by invoking that runner's tool in the project directory.

#### Scenario: Makefile target
- **WHEN** only the Makefile offers `build` and the user runs `r build`
- **THEN** `make build` is executed in the project directory

#### Scenario: package.json script
- **WHEN** only `package.json` offers `start`, the selected package manager is `yarn`, and the user runs `r start`
- **THEN** `yarn run start` is executed in the project directory

#### Scenario: Run from a subdirectory
- **WHEN** the user runs `r build` in a subdirectory of the project directory
- **THEN** the tool is executed with the project directory as its working directory

### Requirement: Runner invocation
`r` SHALL invoke tasks as: `make <task>`; `npm run <task>`, `yarn run <task>`, `pnpm run <task>`, `bun run <task>`; `./gradlew <task>` when a Gradle wrapper exists in the project directory, otherwise `gradle <task>`; and for `custom`, the command's `run` definition through `sh`.

#### Scenario: Gradle wrapper preferred
- **WHEN** the project directory contains `gradlew` and the user runs `r gradle build`
- **THEN** `./gradlew build` is executed

#### Scenario: No Gradle wrapper
- **WHEN** the project directory contains no `gradlew` and the user runs `r gradle build`
- **THEN** `gradle build` is executed

### Requirement: Ambiguous task names fail
When more than one runner offers a task with the requested name, `r` SHALL NOT run anything. It SHALL print an error listing every runner that offers the name together with the runner-qualified command for each, and exit with status 2.

#### Scenario: Same name in Makefile and package.json
- **WHEN** both the Makefile and `package.json` (npm) offer `test` and the user runs `r test`
- **THEN** nothing is executed, the error names `make` and `npm` and suggests `r make test` and `r npm test`, and the exit status is 2

#### Scenario: Custom command duplicates a discovered task
- **WHEN** `r.yaml` defines a command `start` and `package.json` also has a `start` script
- **THEN** `r start` fails as ambiguous, suggesting `r custom start` and `r npm start`

### Requirement: Runner-qualified invocation
`r <runner> <task> [args...]` SHALL run the task with the named runner, regardless of whether other runners offer the same name. For Node package managers, any of `npm`, `yarn`, `pnpm`, `bun` SHALL be accepted when a `package.json` exists.

#### Scenario: Resolve a duplicate
- **WHEN** both the Makefile and `package.json` offer `start` and the user runs `r npm start`
- **THEN** `npm run start` is executed

#### Scenario: Override the detected package manager
- **WHEN** the detected package manager is `npm` and the user runs `r pnpm build`
- **THEN** `pnpm run build` is executed

#### Scenario: Runner not present in the project
- **WHEN** the project has no Gradle build file and the user runs `r gradle build`
- **THEN** `r` fails with an error saying the project has no `gradle` task source, and exits with status 2

#### Scenario: Runner name without a task
- **WHEN** the user runs `r make`
- **THEN** `r` lists the tasks of the `make` runner and executes nothing

### Requirement: Unknown tasks
When no runner offers the requested name, `r` SHALL print an error naming the task, suggest similarly named tasks if any exist, and exit with status 2. In runner-qualified form, runners whose task list is incomplete (`make`, `gradle`) SHALL be given the name anyway; the others SHALL fail as unknown.

#### Scenario: Unknown task
- **WHEN** no runner offers `tset` but one offers `test`, and the user runs `r tset`
- **THEN** the error says `tset` was not found, suggests `test`, and the exit status is 2

#### Scenario: Undiscovered Gradle task, qualified
- **WHEN** the user runs `r gradle :app:bootRun` and that task was not discovered
- **THEN** the Gradle tool is invoked with `:app:bootRun`

#### Scenario: Unknown npm script, qualified
- **WHEN** `package.json` has no `deploy` script and the user runs `r npm deploy`
- **THEN** `r` fails with an unknown-task error without starting `npm`

### Requirement: Argument forwarding
Every argument after the task name SHALL be forwarded to the task unchanged and SHALL NOT be interpreted as an option of `r`. For `npm`, forwarded arguments SHALL be placed after `--`. For `custom` commands they SHALL be available as positional parameters.

#### Scenario: Flags for the task
- **WHEN** the user runs `r test --watch` and `test` is an npm script
- **THEN** `npm run test -- --watch` is executed

#### Scenario: Make variables
- **WHEN** the user runs `r build V=1` and `build` is a Makefile target
- **THEN** `make build V=1` is executed

#### Scenario: Flag that r also knows
- **WHEN** the user runs `r build --help`
- **THEN** `--help` is forwarded to the task and `r`'s own help is not shown

### Requirement: Exit status and streams
`r` SHALL connect the task's standard input, output and error to its own, and SHALL exit with the task's exit status. When the runner's tool is not installed, `r` SHALL report which program is missing and exit with status 127.

#### Scenario: Failing task
- **WHEN** the executed task exits with status 3
- **THEN** `r` exits with status 3

#### Scenario: Tool missing
- **WHEN** the task belongs to `pnpm` and no `pnpm` program is on the search path
- **THEN** `r` reports that `pnpm` was not found and exits with status 127

#### Scenario: Interrupt
- **WHEN** the user presses Ctrl-C while a task is running
- **THEN** the task receives the interrupt and `r` does not leave it running in the background
