# Spec Delta

## Purpose

Defines how a project declares supporting services such as databases in `r.yaml` and how `r` runs them as background Podman containers that outlive individual commands.

## ADDED Requirements

### Requirement: Service definition
Each key under `services` in `r.yaml` SHALL define a service with a required `image` and optional `command` (list of strings), `ports` (list of `host:container` mappings), `env` (mapping), `volumes` (list of `source:target` mappings) and `description`. Service names SHALL consist of lowercase letters, digits and `-`.

#### Scenario: Minimal service
- **WHEN** `r.yaml` declares a service `cache` with only `image: docker.io/library/redis:7`
- **THEN** the file is accepted and `cache` is a known service

#### Scenario: Missing image
- **WHEN** a service is declared without `image`
- **THEN** `r` rejects the file, naming the service

### Requirement: Start services in the background
`r up [service...]` SHALL start the named services, or all declared services when none is named, as detached Podman containers, and SHALL return once they are started. Images that are not present locally SHALL be pulled.

#### Scenario: Start all
- **WHEN** `r.yaml` declares `db` and `cache` and the user runs `r up`
- **THEN** one container per service is running in the background and `r` has exited with status 0

#### Scenario: Start one
- **WHEN** the user runs `r up db`
- **THEN** only the `db` container is started

#### Scenario: Unknown service
- **WHEN** the user runs `r up queue` and no service `queue` is declared
- **THEN** `r` fails, lists the declared services, and exits with status 2

#### Scenario: Start failure
- **WHEN** Podman fails to start a container
- **THEN** `r` shows Podman's error and exits with a non-zero status

### Requirement: Idempotent start
`r up` SHALL leave an already running service untouched when its definition has not changed, start it again when its container exists but is stopped, and replace the container when the service definition has changed since the container was created.

#### Scenario: Already running
- **WHEN** `db` is running with an unchanged definition and the user runs `r up db`
- **THEN** the container is not restarted and the exit status is 0

#### Scenario: Definition changed
- **WHEN** the `image` of `db` was changed in `r.yaml` after its container was created and the user runs `r up db`
- **THEN** the old container is removed and a new one is started from the new image

### Requirement: Container identity
Containers SHALL be named so that the same service name in two different project directories maps to two different containers, and SHALL be marked so that `r` only ever acts on containers it created for the current project.

#### Scenario: Two projects with a db service
- **WHEN** two project directories each declare a service `db` and `r up db` is run in both
- **THEN** two separate containers are running

#### Scenario: Foreign containers are left alone
- **WHEN** the user runs `r down` and other containers not created by `r` for this project are running
- **THEN** those containers keep running

### Requirement: Stop services
`r down [service...]` SHALL stop and remove the containers of the named services, or of all the project's services when none is named, including containers of services that are no longer declared. Named volumes SHALL be kept. Stopping a service that is not running SHALL NOT be an error.

#### Scenario: Stop all
- **WHEN** `db` and `cache` are running and the user runs `r down`
- **THEN** both containers are gone

#### Scenario: Data survives
- **WHEN** `db` uses a named volume, and the user runs `r down` followed by `r up`
- **THEN** the data in the volume is still present

#### Scenario: Nothing running
- **WHEN** no service is running and the user runs `r down`
- **THEN** the exit status is 0

### Requirement: Service status
`r ps` SHALL list every declared service with its state (running, stopped or not created), its image and its published ports. `r ps --json` SHALL print the same information as JSON.

#### Scenario: Mixed states
- **WHEN** `db` is running and `cache` was never started
- **THEN** `r ps` shows `db` as running and `cache` as not created

### Requirement: Service logs
`r logs <service>` SHALL print the output of the service's container, and with `-f` SHALL keep following it until interrupted.

#### Scenario: Logs of a running service
- **WHEN** `db` is running and the user runs `r logs db`
- **THEN** the container's output so far is printed

#### Scenario: Service never started
- **WHEN** the user runs `r logs cache` and `cache` has no container
- **THEN** `r` says the service is not created and exits with a non-zero status

### Requirement: Start on demand for commands
Before running a custom command that lists services under `needs`, `r` SHALL start those services as `r up` would. The services SHALL be left running after the command ends. If a service cannot be started, the command SHALL NOT run.

#### Scenario: Command needs a database
- **WHEN** the command `dev` lists `needs: [db]`, `db` is not running, and the user runs `r dev`
- **THEN** `db` is started first, then the command runs, and `db` is still running after the command exits

#### Scenario: Service fails to start
- **WHEN** a needed service cannot be started
- **THEN** the command is not run and `r` exits with a non-zero status

### Requirement: Podman availability
Service features SHALL require the `podman` program. When it is not installed, service commands and commands with `needs` SHALL fail with a message that says Podman is required, exit status 127, and all features that do not involve services SHALL keep working.

#### Scenario: Podman missing, service command
- **WHEN** `podman` is not installed and the user runs `r up`
- **THEN** `r` reports that Podman is required and exits with status 127

#### Scenario: Podman missing, ordinary task
- **WHEN** `podman` is not installed, `r.yaml` declares services, and the user runs a Makefile target through `r`
- **THEN** the target runs normally

### Requirement: Service environment
A service container SHALL receive only the environment variables declared in the service's `env`. Variables of the caller's shell SHALL NOT be passed into the container.

#### Scenario: Caller's secret is not forwarded
- **WHEN** the caller's shell has `AWS_SECRET_ACCESS_KEY` set and the user runs `r up db`
- **THEN** the variable is not set inside the `db` container
