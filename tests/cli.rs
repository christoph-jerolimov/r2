//! End-to-end tests: run the binary against a small project on disk.

use std::path::{Path, PathBuf};
use std::process::Command;

fn project(name: &str, files: &[(&str, &str)]) -> PathBuf {
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR")).join(name);
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    for (file, content) in files {
        std::fs::write(dir.join(file), content).unwrap();
    }
    dir
}

/// Returns (success, stdout, stderr).
fn r(dir: &Path, args: &[&str]) -> (bool, String, String) {
    let out = Command::new(env!("CARGO_BIN_EXE_r"))
        .args(args)
        .current_dir(dir)
        .output()
        .unwrap();
    (
        out.status.success(),
        String::from_utf8(out.stdout).unwrap(),
        String::from_utf8(out.stderr).unwrap(),
    )
}

const MAKEFILE: &str = "build:\n\t@echo make-build\nlint:\n\t@echo make-lint\n";
const PACKAGE: &str = r#"{"scripts": {"build": "echo npm-build", "start": "echo npm-start"}}"#;

#[test]
fn unique_task_uses_its_runner() {
    let dir = project(
        "unique",
        &[("Makefile", MAKEFILE), ("package.json", PACKAGE)],
    );
    assert_eq!(r(&dir, &["-n", "lint"]).1, "make lint\n");
    assert_eq!(
        r(&dir, &["-n", "start", "--port", "1"]).1,
        "npm run start -- --port 1\n"
    );
    assert_eq!(
        r(&dir, &["which", "start", "--", "-x"]).1,
        "npm run start -- -x\n"
    );
}

#[test]
fn ambiguous_task_fails_and_runner_name_resolves_it() {
    let dir = project(
        "ambiguous",
        &[("Makefile", MAKEFILE), ("package.json", PACKAGE)],
    );
    let (ok, _, err) = r(&dir, &["build"]);
    assert!(!ok);
    assert!(
        err.contains("ambiguous") && err.contains("r make build") && err.contains("r npm build")
    );
    assert_eq!(r(&dir, &["-n", "make", "build"]).1, "make build\n");
    assert_eq!(r(&dir, &["-n", "npm", "build"]).1, "npm run build\n");
    assert_eq!(r(&dir, &["-n", "yarn", "build"]).1, "yarn run build\n");
}

#[test]
fn package_manager_is_detected_from_lockfile() {
    let dir = project("yarn", &[("package.json", PACKAGE), ("yarn.lock", "")]);
    assert_eq!(r(&dir, &["-n", "start", "-x"]).1, "yarn run start -x\n");
    // Unknown scripts go straight to the tool
    assert_eq!(r(&dir, &["-n", "yarn", "install"]).1, "yarn install\n");
}

#[test]
fn runs_tasks_and_forwards_exit_codes() {
    let dir = project(
        "run",
        &[
            ("Makefile", MAKEFILE),
            ("r.yaml", "tasks:\n  fail: exit 7\n  echo: echo got\n"),
        ],
    );
    assert_eq!(r(&dir, &["build"]).1, "make-build\n");
    assert_eq!(r(&dir, &["echo", "a b"]).1, "got a b\n");
    let out = Command::new(env!("CARGO_BIN_EXE_r"))
        .arg("fail")
        .current_dir(&dir)
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(7));
    // Found from subdirectories too
    std::fs::create_dir(dir.join("sub")).unwrap();
    assert_eq!(r(&dir.join("sub"), &["build"]).1, "make-build\n");
}

#[test]
fn config_tasks_win_and_support_deps_and_containers() {
    let config = "\
tasks:
  build:
    deps: [lint]
    run: echo custom
  db:
    container:
      engine: podman
      image: postgres:17
      ports: ['5432:5432']
      env: {POSTGRES_PASSWORD: x}
      detach: true
";
    let dir = project("config", &[("Makefile", MAKEFILE), ("r.yaml", config)]);
    assert_eq!(
        r(&dir, &["-n", "build"]).1,
        "make lint\nsh -c 'echo custom'\n"
    );
    assert_eq!(r(&dir, &["-n", "make", "build"]).1, "make build\n");
    assert_eq!(
        r(&dir, &["-n", "db"]).1,
        "podman run --rm --detach --publish 5432:5432 --env POSTGRES_PASSWORD=x postgres:17\n"
    );
}

#[test]
fn ignore_removes_a_runner() {
    let dir = project(
        "ignore",
        &[
            ("Makefile", MAKEFILE),
            ("package.json", PACKAGE),
            ("run.yaml", "ignore: [make]\n"),
        ],
    );
    assert_eq!(r(&dir, &["-n", "build"]).1, "npm run build\n");
}

#[test]
fn list_and_completion() {
    let dir = project("list", &[("Makefile", MAKEFILE), ("package.json", PACKAGE)]);
    let (ok, out, _) = r(&dir, &["list"]);
    assert!(ok);
    assert!(out.contains("make (Makefile)") && out.contains("npm (package.json)"));
    assert!(out.contains("* build") && out.contains("  lint"));

    let (_, out, _) = r(&dir, &["__complete"]);
    let names: Vec<&str> = out.lines().map(|l| l.split('\t').next().unwrap()).collect();
    for expected in ["build", "lint", "start", "make", "npm", "list"] {
        assert!(names.contains(&expected), "{expected} missing in {names:?}");
    }
    assert_eq!(
        r(&dir, &["__complete", "npm"]).1,
        "build\techo npm-build\nstart\techo npm-start\n"
    );
    for shell in ["bash", "zsh", "fish"] {
        assert!(r(&dir, &["completions", shell]).0);
    }
}

#[test]
fn unknown_task_suggests_a_similar_one() {
    let dir = project("unknown", &[("Makefile", MAKEFILE)]);
    let (ok, _, err) = r(&dir, &["biuld"]);
    assert!(!ok && err.contains("did you mean `build`"));
}
