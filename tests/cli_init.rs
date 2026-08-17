use std::fs;
use std::io::Write;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::time::{SystemTime, UNIX_EPOCH};

fn temp_target(label: &str) -> PathBuf {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock should be after epoch")
        .as_nanos();
    std::env::temp_dir().join(format!("yss-template-runtime-{label}-{nonce}"))
}

#[test]
fn init_dry_run_binds_the_frozen_snapshot_without_mutating_target() {
    let target = temp_target("dry-run");
    let output = Command::new(env!("CARGO_BIN_EXE_create-yss-spec"))
        .args([
            "--project-name",
            "Demo Project",
            "--business-domain",
            "Data Platform",
            "--target-dir",
        ])
        .arg(&target)
        .arg("--dry-run")
        .output()
        .expect("run create-yss-spec");

    assert_eq!(output.status.code(), Some(0));
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    assert!(stdout.contains("dry-run 预览"));
    assert!(stdout.contains("模板快照：68c367a13d5006cca83f1c5e369678af28c4bf15"));
    assert!(stdout.contains("yss-project.yaml"));
    assert!(stdout.contains("AGENTS.md"));
    assert!(
        !target.exists(),
        "dry-run must not create a target directory"
    );
}

#[test]
fn init_apply_extracts_and_renders_the_frozen_snapshot() {
    let target = temp_target("apply");
    let output = Command::new(env!("CARGO_BIN_EXE_create-yss-spec"))
        .args([
            "--project-name",
            "Demo Project",
            "--business-domain",
            "Data Platform",
            "--target-dir",
        ])
        .arg(&target)
        .output()
        .expect("run create-yss-spec");

    assert_eq!(
        output.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(target.join(".yss-template.json").is_file());
    assert!(target.join("AGENTS.md").is_file());
    assert!(target.join("README.md").is_file());
    assert_eq!(
        fs::read_to_string(target.join("yss-project.yaml")).expect("identity should exist"),
        "schema_version: 1\nrepository_mode: project-instance\n"
    );
    assert!(
        !fs::read_to_string(target.join("AGENTS.md"))
            .expect("agents should exist")
            .contains("[填写]")
    );
    assert!(
        fs::read_to_string(target.join("README.md"))
            .expect("readme should exist")
            .starts_with("# Demo Project")
    );
    let metadata =
        fs::read_to_string(target.join(".yss-template.json")).expect("metadata should exist");
    assert!(metadata.contains("\"metadataSchemaVersion\": 2"));
    assert!(metadata.contains("\"templateCommit\": \"68c367a13d5006cca83f1c5e369678af28c4bf15\""));
    assert!(metadata.contains("\"runtime\""));
    assert!(target.join(".agents/skills").is_dir());

    fs::remove_dir_all(target).expect("test target should be removable");
}

#[test]
fn init_force_rolls_back_when_git_init_fails() {
    let target = temp_target("force-rollback");
    fs::create_dir_all(&target).expect("target should exist");
    fs::write(target.join("keep.txt"), "keep me\n").expect("marker should exist");

    let output = Command::new(env!("CARGO_BIN_EXE_create-yss-spec"))
        .args([
            "--project-name",
            "Demo Project",
            "--business-domain",
            "Data Platform",
            "--target-dir",
        ])
        .arg(&target)
        .args(["--force", "--git-init"])
        .env("PATH", "")
        .output()
        .expect("run create-yss-spec");

    assert_eq!(output.status.code(), Some(1));
    assert_eq!(
        fs::read_to_string(target.join("keep.txt")).expect("rollback should restore marker"),
        "keep me\n"
    );
    assert!(!target.join(".yss-template.json").exists());
    assert!(!target.join("AGENTS.md").exists());

    fs::remove_dir_all(target).expect("test target should be removable");
}

#[test]
fn init_failure_preserves_an_existing_empty_target_directory() {
    let target = temp_target("empty-target-rollback");
    fs::create_dir_all(&target).expect("empty target should exist");
    let output = Command::new(env!("CARGO_BIN_EXE_create-yss-spec"))
        .args([
            "--project-name",
            "rollback",
            "--business-domain",
            "runtime",
            "--target-dir",
            target.to_str().expect("target should be utf-8"),
            "--git-init",
        ])
        .env("PATH", "")
        .output()
        .expect("run init");
    assert_eq!(output.status.code(), Some(1));
    assert!(target.is_dir());
    assert_eq!(
        fs::read_dir(&target)
            .expect("target should be readable")
            .count(),
        0
    );
    fs::remove_dir_all(target).expect("test target should be removable");
}

#[test]
fn init_force_preserves_the_existing_git_directory() {
    let target = temp_target("force-preserve-git");
    fs::create_dir_all(&target).expect("target should exist");
    let git = Command::new("git")
        .arg("init")
        .current_dir(&target)
        .output()
        .expect("git should be available for fixture setup");
    assert!(git.status.success());
    fs::write(target.join("local.txt"), "local\n").expect("local file should exist");
    let output = Command::new(env!("CARGO_BIN_EXE_create-yss-spec"))
        .args([
            "--project-name",
            "force",
            "--business-domain",
            "runtime",
            "--target-dir",
            target.to_str().expect("target should be utf-8"),
            "--force",
        ])
        .env("PATH", "")
        .output()
        .expect("run force init");
    assert_eq!(
        output.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(target.join(".git").exists());
    assert!(!target.join("local.txt").exists());
    fs::remove_dir_all(target).expect("test target should be removable");
}

#[test]
fn init_force_failure_restores_the_existing_git_directory() {
    let target = temp_target("force-rollback-git");
    fs::create_dir_all(&target).expect("target should exist");
    let git = Command::new("git")
        .arg("init")
        .current_dir(&target)
        .output()
        .expect("git should be available for fixture setup");
    assert!(git.status.success());
    fs::write(target.join("local.txt"), "local\n").expect("local file should exist");
    let output = Command::new(env!("CARGO_BIN_EXE_create-yss-spec"))
        .args([
            "--project-name",
            "force",
            "--business-domain",
            "runtime",
            "--target-dir",
            target.to_str().expect("target should be utf-8"),
            "--force",
            "--git-init",
        ])
        .env("PATH", "")
        .output()
        .expect("run force init");
    assert_eq!(output.status.code(), Some(1));
    assert!(target.join(".git").exists());
    assert_eq!(
        fs::read_to_string(target.join("local.txt")).expect("local file should be restored"),
        "local\n"
    );
    fs::remove_dir_all(target).expect("test target should be removable");
}

#[test]
fn init_accepts_buffered_interactive_answers() {
    let target = temp_target("interactive");
    let mut child = Command::new(env!("CARGO_BIN_EXE_create-yss-spec"))
        .env("PATH", "")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn create-yss-spec");
    child
        .stdin
        .take()
        .expect("stdin should be available")
        .write_all(format!("Demo Project\nData Platform\n12\n{}\n", target.display()).as_bytes())
        .expect("write interactive answers");
    let output = child.wait_with_output().expect("wait for create-yss-spec");

    assert_eq!(
        output.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(target.join(".yss-template.json").is_file());
    assert!(String::from_utf8_lossy(&output.stdout).contains("项目名称:"));

    fs::remove_dir_all(target).expect("test target should be removable");
}
