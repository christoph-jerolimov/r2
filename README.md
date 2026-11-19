# r

One command for every task runner. `r` looks at the files in your project
(`Makefile`, `package.json`, `justfile`, ...) and calls the right tool.

```console
$ r
make (Makefile)
  lint   Run the linters
* build  Build everything

yarn (package.json)
  start  vite
* build  vite build

* defined more than once, use `r <runner> <task>`: build

$ r start            # runs: yarn run start
$ r lint --fix       # runs: make lint --fix
$ r build
r: `build` is ambiguous, it is defined by 2 runners. Pick one:
  r make build    (Makefile)
  r yarn build    (package.json)
$ r yarn build       # runs: yarn run build
```

## Install

```sh
cargo install --path .
```

## Supported runners

| Runner name                  | File                                   | Tasks                         |
| ---------------------------- | -------------------------------------- | ----------------------------- |
| `make`                       | `Makefile`, `makefile`, `GNUmakefile`  | targets                       |
| `npm`, `yarn`, `pnpm`, `bun` | `package.json`                         | scripts                       |
| `just`                       | `justfile`, `.justfile`                | recipes and aliases           |
| `task`                       | `Taskfile.yml`, `Taskfile.yaml`, ...   | tasks                         |
| `cargo`                      | `Cargo.toml`                           | build, check, test, run, ...  |
| `r`                          | `r.yaml`, `r.yml`, `run.yaml`, `run.yml` | your own commands, containers |

The package manager for `package.json` is taken from the `packageManager`
field, then from the lockfile (`yarn.lock`, `pnpm-lock.yaml`, `bun.lock`,
`package-lock.json`), and defaults to npm.

Runner files are looked up in the current directory and then in its parents,
so `r` works from subdirectories. Commands run in the directory of the runner file.

## Usage

```
r                         List all available tasks
r <task> [args...]        Run a task with the tool that defines it
r <runner> <task> [args]  Run a task with a specific runner (r npm start)

r list [runner] [--json]  List tasks (alias: ls)
r run <task> [args...]    Run a task, even if it is named like a command
r which <task> [args...]  Show the command a task would run
r init                    Create an r.yaml with examples
r completions <shell>     Print a completion script (bash, zsh, fish)

-n, --dry-run             Print the commands instead of running them
-C, --dir <dir>           Run as if started in <dir>
```

Rules:

- A task defined by exactly one runner just runs.
- A task defined by several runners fails. Name the runner (`r make build`),
  or define the task in `r.yaml`: tasks from `r.yaml` always win.
- The first word is checked in this order: built-in command, runner name, task.
  Use `r run list` for a task that is called like a built-in command.
- With a runner name, things that are not a known task are passed to the tool:
  `r npm install` runs `npm install`.
- Arguments after the task go to the task. `r test --watch` and
  `r test -- --watch` are the same; for npm the `--` is added for you.
- The exit code is the exit code of the task.

## Shell completion

Completion includes the tasks of the current directory, with descriptions.

```sh
# bash (~/.bashrc)
eval "$(r completions bash)"

# zsh (~/.zshrc, after compinit)
eval "$(r completions zsh)"

# fish
r completions fish > ~/.config/fish/completions/r.fish
```

## r.yaml

`r init` creates a file with examples. `run.yaml` works as well.

```yaml
# Runners to ignore in this directory
ignore: [cargo]

tasks:
  # Short form: a shell command. Extra arguments are appended.
  hello: echo Hello

  check:
    description: Lint and test
    deps: [hello]            # tasks to run first (from any runner)
    dir: frontend            # working directory, relative to this file
    env:
      CI: "true"
    run:                     # one command or a list
      - npm run lint
      - npm test

  # Start a container: docker run --rm --name dev-db --publish ... postgres:17
  db:
    description: Start a local postgres
    container:
      image: postgres:17
      name: dev-db
      ports: ["5432:5432"]
      env:
        POSTGRES_PASSWORD: postgres
      volumes: ["./data:/var/lib/postgresql/data"]

  # Run a command inside a container
  node-test:
    run: npm test
    container:
      image: node:22
      volumes: [".:/app"]
      workdir: /app
```

Container options: `image` (required), `name`, `ports`, `env`, `volumes`,
`workdir`, `command`, `args` (extra arguments for `docker run`), `detach`
(default `false`), `rm` (default `true`) and `engine`. The engine defaults to
`$R_CONTAINER_ENGINE`, then `docker`, then `podman`.

## Development

```sh
cargo test
```
