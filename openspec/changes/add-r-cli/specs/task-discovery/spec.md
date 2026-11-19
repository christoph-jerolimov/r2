# Spec Delta

## Purpose

Defines how `r` locates a project and which tasks it finds in the project's command runner files, so that every other command works from one consistent task inventory.

## ADDED Requirements

### Requirement: Project directory lookup
`r` SHALL use as the project directory the nearest directory, starting at the current working directory and moving through its ancestors, that contains at least one supported task source file. Only files in that one directory SHALL be used as task sources.

#### Scenario: Invoked in the project root
- **WHEN** `r list` is run in a directory containing a `Makefile`
- **THEN** that directory is the project directory and the Makefile's targets are listed

#### Scenario: Invoked in a subdirectory
- **WHEN** `r list` is run in `web/src/`, neither `web/src/` nor `web/` contains a task source file, and the parent of `web/` contains a `package.json`
- **THEN** the parent of `web/` is the project directory

#### Scenario: Nearest directory wins
- **WHEN** both the current directory and its parent contain task source files
- **THEN** only the current directory's files are used

#### Scenario: No task source anywhere
- **WHEN** no directory from the current one up to the filesystem root contains a task source file
- **THEN** `r` prints an error saying no supported task file was found, names the supported file types, and exits with status 2

### Requirement: Supported runners
`r` SHALL recognise these runners, each identified by a fixed runner name: `make` (`Makefile`, `makefile`, `GNUmakefile`), `npm`, `yarn`, `pnpm` and `bun` (`package.json`), `gradle` (`build.gradle`, `build.gradle.kts`, `settings.gradle`, `settings.gradle.kts`), and `custom` (`r.yaml`).

#### Scenario: Several runner files in one project
- **WHEN** the project directory contains a `Makefile`, a `package.json` and an `r.yaml` with commands
- **THEN** tasks from all three are part of the task inventory, each tagged with its runner name

#### Scenario: Unsupported files are ignored
- **WHEN** the project directory contains files `r` does not recognise as task sources
- **THEN** they contribute no tasks and cause no error

### Requirement: Makefile targets
`r` SHALL list the explicit targets of the Makefile as tasks without executing `make`. Pattern rules, targets whose name starts with a dot, and variable assignments SHALL NOT be listed. Text following `##` on a target line SHALL be used as the task description.

#### Scenario: Explicit targets
- **WHEN** the Makefile defines `build:`, `test: build`, `.PHONY: build test` and `%.o: %.c`
- **THEN** the tasks `build` and `test` are discovered and nothing else

#### Scenario: Target description
- **WHEN** the Makefile contains `build: ## Compile everything`
- **THEN** the task `build` has the description `Compile everything`

### Requirement: package.json scripts
`r` SHALL list each entry of the `scripts` object in `package.json` as a task, using the script's command text as its description. A `package.json` without a `scripts` object SHALL contribute no tasks.

#### Scenario: Scripts become tasks
- **WHEN** `package.json` contains `"scripts": {"start": "vite", "test": "vitest"}`
- **THEN** the tasks `start` and `test` are discovered

#### Scenario: Malformed package.json
- **WHEN** `package.json` is not valid JSON
- **THEN** `r` reports the file and the parse error and exits with status 2

### Requirement: Package manager selection
The scripts of `package.json` SHALL belong to exactly one of the runners `npm`, `yarn`, `pnpm` or `bun`. `r` SHALL choose it from the `packageManager` field if present, otherwise from the single lockfile present (`package-lock.json`, `yarn.lock`, `pnpm-lock.yaml`, `bun.lock` or `bun.lockb`), otherwise `npm`.

#### Scenario: packageManager field
- **WHEN** `package.json` contains `"packageManager": "pnpm@9.1.0"` and a `yarn.lock` also exists
- **THEN** the scripts belong to the `pnpm` runner

#### Scenario: Single lockfile
- **WHEN** there is no `packageManager` field and only `yarn.lock` exists
- **THEN** the scripts belong to the `yarn` runner

#### Scenario: No hints
- **WHEN** there is no `packageManager` field and no lockfile
- **THEN** the scripts belong to the `npm` runner

#### Scenario: Conflicting lockfiles
- **WHEN** there is no `packageManager` field and both `yarn.lock` and `package-lock.json` exist
- **THEN** the scripts are listed with the package manager marked as undetermined, and running one without naming a runner fails with an error that names both lockfiles

### Requirement: Gradle tasks
`r` SHALL discover Gradle tasks without executing Gradle: tasks registered by name in the root build file, plus the standard lifecycle tasks of the plugins the root build file applies. The Gradle task list SHALL be treated as incomplete.

#### Scenario: Java project
- **WHEN** `build.gradle.kts` applies the `java` plugin and registers a task `generateDocs`
- **THEN** the discovered tasks include `build`, `clean`, `check`, `test`, `assemble` and `generateDocs`

#### Scenario: Listing does not start Gradle
- **WHEN** `r list` is run in a Gradle project
- **THEN** no `gradle` or `gradlew` process is started

### Requirement: Custom commands as tasks
Each entry under `commands` in `r.yaml` SHALL be a task of the `custom` runner, with its `description` if one is given.

#### Scenario: Custom command discovered
- **WHEN** `r.yaml` defines a command `dev` with description `Start everything`
- **THEN** the task `dev` of runner `custom` is discovered with that description
