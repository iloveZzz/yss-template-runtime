use std::fs;
use std::path::PathBuf;
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

fn temp_target(label: &str) -> PathBuf {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock should be after epoch")
        .as_nanos();
    std::env::temp_dir().join(format!("yss-template-runtime-{label}-{nonce}"))
}

fn run_init(target: &PathBuf) {
    let output = Command::new(env!("CARGO_BIN_EXE_create-yss-spec"))
        .args([
            "--project-name",
            "Demo Project",
            "--business-domain",
            "Data Platform",
            "--target-dir",
        ])
        .arg(target)
        .env("PATH", "")
        .output()
        .expect("run create-yss-spec");
    assert_eq!(
        output.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn attach_dry_run_does_not_mutate_and_apply_works_without_node() {
    let target = temp_target("attach");
    fs::create_dir_all(&target).expect("target should exist");
    fs::write(target.join("README.md"), "local README\n").expect("local file should exist");
    let before = fs::read_to_string(target.join("README.md")).expect("read local file");

    let dry_run = Command::new(env!("CARGO_BIN_EXE_create-yss-spec"))
        .args([
            "attach",
            "--target-dir",
            target.to_str().expect("target should be utf-8"),
            "--project-name",
            "Demo Project",
            "--business-domain",
            "Data Platform",
            "--dry-run",
        ])
        .env("PATH", "")
        .output()
        .expect("run attach dry-run");
    assert_eq!(
        dry_run.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&dry_run.stderr)
    );
    assert!(String::from_utf8_lossy(&dry_run.stdout).contains("attach dry-run 预览"));
    assert_eq!(
        fs::read_to_string(target.join("README.md")).expect("read local file"),
        before
    );
    assert!(!target.join(".yss-template.json").exists());

    let apply = Command::new(env!("CARGO_BIN_EXE_create-yss-spec"))
        .args([
            "attach",
            "--target-dir",
            target.to_str().expect("target should be utf-8"),
            "--project-name",
            "Demo Project",
            "--business-domain",
            "Data Platform",
            "--apply",
            "--force",
        ])
        .env("PATH", "")
        .output()
        .expect("run attach apply");
    assert_eq!(
        apply.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&apply.stderr)
    );
    assert!(target.join(".yss-template.json").is_file());
    assert!(target.join("AGENTS.md").is_file());
    assert!(target.join(".agents/skills").is_dir());

    fs::remove_dir_all(target).expect("test target should be removable");
}

#[test]
fn attach_migrates_legacy_paths_before_native_verify() {
    let target = temp_target("legacy-attach");
    fs::create_dir_all(target.join("docs/templates")).expect("legacy template dir should exist");
    fs::write(
        target.join("docs/templates/prd-template.md"),
        "legacy template path\n",
    )
    .expect("legacy file should exist");
    fs::create_dir_all(target.join(".agents/skills/to-prd")).expect("legacy skill should exist");
    fs::write(
        target.join(".agents/skills/to-prd/SKILL.md"),
        "legacy skill path\n",
    )
    .expect("legacy skill should exist");

    let output = Command::new(env!("CARGO_BIN_EXE_create-yss-spec"))
        .args([
            "attach",
            "--target-dir",
            target.to_str().expect("target should be utf-8"),
            "--project-name",
            "Demo Project",
            "--business-domain",
            "Data Platform",
            "--apply",
        ])
        .env("PATH", "")
        .output()
        .expect("run attach migration");
    assert_eq!(
        output.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(!target.join("docs/templates/prd-template.md").exists());
    assert!(!target.join(".agents/skills/to-prd").exists());
    assert!(target.join("docs/templates/spec-template.md").is_file());
    assert!(target.join(".agents/skills/to-spec/SKILL.md").is_file());

    fs::remove_dir_all(target).expect("test target should be removable");
}

#[test]
fn attach_converts_template_source_identity_without_force() {
    let target = temp_target("attach-identity");
    fs::create_dir_all(&target).expect("target should exist");
    fs::write(
        target.join("yss-project.yaml"),
        "schema_version: 1\nrepository_mode: template-source\n",
    )
    .expect("legacy identity should be writable");
    let output = Command::new(env!("CARGO_BIN_EXE_create-yss-spec"))
        .args([
            "attach",
            "--target-dir",
            target.to_str().expect("target should be utf-8"),
            "--project-name",
            "attach-identity",
            "--business-domain",
            "runtime",
            "--apply",
        ])
        .env("PATH", "")
        .output()
        .expect("run identity attach");
    assert_eq!(
        output.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        fs::read_to_string(target.join("yss-project.yaml"))
            .expect("identity should exist")
            .contains("repository_mode: project-instance")
    );
    assert!(target.join(".yss-template.json").is_file());
    fs::remove_dir_all(target).expect("test target should be removable");
}

#[test]
fn sync_dry_run_preserves_local_changes_and_force_updates_managed_file() {
    let target = temp_target("sync");
    run_init(&target);
    let readme = target.join("README.md");
    fs::write(&readme, "local edit\n").expect("managed file should be editable");

    let dry_run = Command::new(env!("CARGO_BIN_EXE_create-yss-spec"))
        .args([
            "sync",
            "--target-dir",
            target.to_str().expect("target should be utf-8"),
            "--dry-run",
        ])
        .env("PATH", "")
        .output()
        .expect("run sync dry-run");
    assert_eq!(
        dry_run.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&dry_run.stderr)
    );
    assert!(String::from_utf8_lossy(&dry_run.stdout).contains("sync dry-run 预览"));
    assert_eq!(
        fs::read_to_string(&readme).expect("read local edit"),
        "local edit\n"
    );

    let keep_local = Command::new(env!("CARGO_BIN_EXE_create-yss-spec"))
        .args([
            "sync",
            "--target-dir",
            target.to_str().expect("target should be utf-8"),
        ])
        .env("PATH", "")
        .output()
        .expect("run sync without force");
    assert_eq!(
        keep_local.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&keep_local.stderr)
    );
    assert_eq!(
        fs::read_to_string(&readme).expect("read local edit"),
        "local edit\n"
    );

    let force = Command::new(env!("CARGO_BIN_EXE_create-yss-spec"))
        .args([
            "sync",
            "--target-dir",
            target.to_str().expect("target should be utf-8"),
            "--force",
        ])
        .env("PATH", "")
        .output()
        .expect("run sync force");
    assert_eq!(
        force.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&force.stderr)
    );
    assert_ne!(
        fs::read_to_string(&readme).expect("read synced file"),
        "local edit\n"
    );

    fs::remove_dir_all(target).expect("test target should be removable");
}

#[test]
fn sync_skips_an_unmanaged_conflict_without_mutating_it() {
    let target = temp_target("unmanaged-conflict");
    run_init(&target);
    let metadata_path = target.join(".yss-template.json");
    let mut metadata: serde_json::Value =
        serde_json::from_slice(&fs::read(&metadata_path).expect("metadata should exist"))
            .expect("metadata should be JSON");
    metadata
        .as_object_mut()
        .expect("metadata object")
        .get_mut("managedFiles")
        .and_then(serde_json::Value::as_object_mut)
        .expect("managed files object")
        .remove("README.md");
    fs::write(
        &metadata_path,
        serde_json::to_vec_pretty(&metadata).expect("metadata should serialize"),
    )
    .expect("metadata should be writable");
    let local = b"locally-owned README\n";
    fs::write(target.join("README.md"), local).expect("local conflict should be writable");

    let output = Command::new(env!("CARGO_BIN_EXE_create-yss-spec"))
        .args([
            "sync",
            "--target-dir",
            target.to_str().expect("target should be utf-8"),
        ])
        .env("PATH", "")
        .output()
        .expect("run unmanaged sync");
    assert_eq!(
        output.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        fs::read(target.join("README.md")).expect("README should exist"),
        local
    );
    let migrated: serde_json::Value =
        serde_json::from_slice(&fs::read(&metadata_path).expect("metadata should exist"))
            .expect("metadata should be JSON");
    assert!(migrated["managedFiles"].get("README.md").is_none());
    fs::remove_dir_all(target).expect("test target should be removable");
}

#[test]
fn sync_migrates_legacy_metadata_transactionally_without_node() {
    let target = temp_target("metadata-migration");
    run_init(&target);
    let metadata_path = target.join(".yss-template.json");
    let mut metadata: serde_json::Value =
        serde_json::from_slice(&fs::read(&metadata_path).expect("metadata should exist"))
            .expect("metadata should be JSON");
    metadata["metadataSchemaVersion"] = serde_json::json!(1);
    metadata
        .as_object_mut()
        .expect("metadata object")
        .remove("runtime");
    metadata
        .as_object_mut()
        .expect("metadata object")
        .remove("managedFiles");
    fs::write(
        &metadata_path,
        serde_json::to_vec_pretty(&metadata).expect("legacy metadata should serialize"),
    )
    .expect("legacy metadata should be written");
    let removed_file = target.join("docs/templates/spec-template.md");
    fs::remove_file(&removed_file).expect("fixture should have a managed file");

    let output = Command::new(env!("CARGO_BIN_EXE_create-yss-spec"))
        .args([
            "sync",
            "--target-dir",
            target.to_str().expect("target should be utf-8"),
        ])
        .env("PATH", "")
        .output()
        .expect("run sync migration");
    assert_eq!(
        output.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let migrated: serde_json::Value =
        serde_json::from_slice(&fs::read(&metadata_path).expect("migrated metadata should exist"))
            .expect("migrated metadata should be JSON");
    assert_eq!(migrated["metadataSchemaVersion"], serde_json::json!(2));
    assert_eq!(
        migrated["runtime"]["kind"],
        serde_json::json!("native-rust")
    );
    assert!(removed_file.is_file());

    fs::remove_dir_all(target).expect("test target should be removable");
}

#[test]
fn sync_migrates_node_v2_metadata_without_runtime() {
    let target = temp_target("node-v2-migration");
    run_init(&target);
    let metadata_path = target.join(".yss-template.json");
    let mut metadata: serde_json::Value =
        serde_json::from_slice(&fs::read(&metadata_path).expect("metadata should exist"))
            .expect("metadata should be JSON");
    metadata
        .as_object_mut()
        .expect("metadata object")
        .remove("runtime");
    let baseline = metadata["managedFiles"]["README.md"]["contentHash"]
        .as_str()
        .expect("README baseline should exist")
        .to_owned();
    let local_readme = b"Node 2.x local edit\n";
    fs::write(target.join("README.md"), local_readme).expect("local edit should be writable");
    fs::write(
        &metadata_path,
        serde_json::to_vec_pretty(&metadata).expect("legacy metadata should serialize"),
    )
    .expect("legacy metadata should be written");

    let output = Command::new(env!("CARGO_BIN_EXE_create-yss-spec"))
        .args([
            "sync",
            "--target-dir",
            target.to_str().expect("target should be utf-8"),
        ])
        .env("PATH", "")
        .output()
        .expect("run node v2 migration");
    assert_eq!(
        output.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let migrated: serde_json::Value =
        serde_json::from_slice(&fs::read(&metadata_path).expect("migrated metadata should exist"))
            .expect("migrated metadata should be JSON");
    assert_eq!(migrated["metadataSchemaVersion"], serde_json::json!(2));
    assert_eq!(
        migrated["runtime"]["kind"],
        serde_json::json!("native-rust")
    );
    assert_eq!(
        migrated["managedFiles"]["README.md"]["contentHash"],
        serde_json::json!(baseline)
    );
    assert_eq!(
        fs::read(target.join("README.md")).expect("local README should remain"),
        local_readme
    );
    fs::remove_dir_all(target).expect("test target should be removable");
}

#[test]
fn sync_rejects_node_v2_metadata_with_tampered_core_fields() {
    let target = temp_target("node-v2-tamper");
    run_init(&target);
    let metadata_path = target.join(".yss-template.json");
    let mut metadata: serde_json::Value =
        serde_json::from_slice(&fs::read(&metadata_path).expect("metadata should exist"))
            .expect("metadata should be JSON");
    metadata
        .as_object_mut()
        .expect("metadata object")
        .remove("runtime");
    metadata["templateCommit"] = serde_json::json!("tampered");
    fs::write(
        &metadata_path,
        serde_json::to_vec_pretty(&metadata).expect("metadata should serialize"),
    )
    .expect("metadata should be writable");
    let output = Command::new(env!("CARGO_BIN_EXE_create-yss-spec"))
        .args([
            "sync",
            "--target-dir",
            target.to_str().expect("target should be utf-8"),
        ])
        .env("PATH", "")
        .output()
        .expect("run tampered sync");
    assert_eq!(output.status.code(), Some(1));
    assert!(String::from_utf8_lossy(&output.stderr).contains("templateCommit"));
    fs::remove_dir_all(target).expect("test target should be removable");
}

#[test]
fn sync_rejects_non_integer_metadata_schema_version() {
    let target = temp_target("invalid-metadata-version");
    run_init(&target);
    let metadata_path = target.join(".yss-template.json");
    let mut metadata: serde_json::Value =
        serde_json::from_slice(&fs::read(&metadata_path).expect("metadata should exist"))
            .expect("metadata should be JSON");
    metadata["metadataSchemaVersion"] = serde_json::json!(2.5);
    fs::write(
        &metadata_path,
        serde_json::to_vec_pretty(&metadata).expect("metadata should serialize"),
    )
    .expect("metadata should be writable");

    let output = Command::new(env!("CARGO_BIN_EXE_create-yss-spec"))
        .args([
            "sync",
            "--target-dir",
            target.to_str().expect("target should be utf-8"),
        ])
        .env("PATH", "")
        .output()
        .expect("run invalid metadata version sync");
    assert_eq!(output.status.code(), Some(1));
    assert!(String::from_utf8_lossy(&output.stderr).contains("metadataSchemaVersion"));
    fs::remove_dir_all(target).expect("test target should be removable");
}

#[test]
fn sync_rejects_non_object_metadata_variables() {
    let target = temp_target("invalid-metadata-variables");
    run_init(&target);
    let metadata_path = target.join(".yss-template.json");
    let mut metadata: serde_json::Value =
        serde_json::from_slice(&fs::read(&metadata_path).expect("metadata should exist"))
            .expect("metadata should be JSON");
    metadata["variables"] = serde_json::json!("not-an-object");
    fs::write(
        &metadata_path,
        serde_json::to_vec_pretty(&metadata).expect("metadata should serialize"),
    )
    .expect("metadata should be writable");

    let output = Command::new(env!("CARGO_BIN_EXE_create-yss-spec"))
        .args([
            "sync",
            "--target-dir",
            target.to_str().expect("target should be utf-8"),
        ])
        .env("PATH", "")
        .output()
        .expect("run invalid metadata variables sync");
    assert_eq!(output.status.code(), Some(1));
    assert!(String::from_utf8_lossy(&output.stderr).contains("variables"));
    fs::remove_dir_all(target).expect("test target should be removable");
}

#[test]
fn sync_rejects_missing_metadata_cli_version() {
    let target = temp_target("missing-metadata-cli-version");
    run_init(&target);
    let metadata_path = target.join(".yss-template.json");
    let mut metadata: serde_json::Value =
        serde_json::from_slice(&fs::read(&metadata_path).expect("metadata should exist"))
            .expect("metadata should be JSON");
    metadata
        .as_object_mut()
        .expect("metadata object")
        .remove("cliVersion");
    fs::write(
        &metadata_path,
        serde_json::to_vec_pretty(&metadata).expect("metadata should serialize"),
    )
    .expect("metadata should be writable");

    let output = Command::new(env!("CARGO_BIN_EXE_create-yss-spec"))
        .args([
            "sync",
            "--target-dir",
            target.to_str().expect("target should be utf-8"),
        ])
        .env("PATH", "")
        .output()
        .expect("run missing metadata cli version sync");
    assert_eq!(output.status.code(), Some(1));
    assert!(String::from_utf8_lossy(&output.stderr).contains("cliVersion"));
    fs::remove_dir_all(target).expect("test target should be removable");
}

#[test]
fn sync_converts_legacy_template_source_identity_without_force() {
    let target = temp_target("identity-migration");
    run_init(&target);
    let identity = target.join("yss-project.yaml");
    fs::write(
        &identity,
        "schema_version: 1\nrepository_mode: template-source\n",
    )
    .expect("legacy identity should be writable");

    let output = Command::new(env!("CARGO_BIN_EXE_create-yss-spec"))
        .args([
            "sync",
            "--target-dir",
            target.to_str().expect("target should be utf-8"),
        ])
        .env("PATH", "")
        .output()
        .expect("run identity migration");
    assert_eq!(
        output.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        fs::read_to_string(identity).expect("identity should exist"),
        "schema_version: 1\nrepository_mode: project-instance\n"
    );

    fs::remove_dir_all(target).expect("test target should be removable");
}

#[test]
fn verify_template_is_a_native_public_seam() {
    let target = temp_target("verify-template");
    run_init(&target);
    let output = Command::new(env!("CARGO_BIN_EXE_create-yss-spec"))
        .args([
            "verify-template",
            "--target-dir",
            target.to_str().expect("target should be utf-8"),
        ])
        .env("PATH", "")
        .output()
        .expect("run native verify");
    assert_eq!(
        output.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(String::from_utf8_lossy(&output.stdout).contains("native verify 通过"));

    fs::remove_dir_all(target).expect("test target should be removable");
}

#[cfg(unix)]
#[test]
fn verify_template_rejects_required_file_symlinks() {
    use std::os::unix::fs::symlink;

    let target = temp_target("verify-symlink");
    run_init(&target);
    let outside = target
        .parent()
        .expect("target should have a parent")
        .join("create-yss-spec-verify-outside.txt");
    fs::write(&outside, "outside").expect("outside fixture should be writable");
    fs::remove_file(target.join("AGENTS.md")).expect("required file should be removable");
    symlink(&outside, target.join("AGENTS.md")).expect("symlink fixture should be creatable");
    let output = Command::new(env!("CARGO_BIN_EXE_create-yss-spec"))
        .args([
            "verify-template",
            "--target-dir",
            target.to_str().expect("target should be utf-8"),
        ])
        .env("PATH", "")
        .output()
        .expect("run native verify");
    assert_eq!(output.status.code(), Some(1));
    assert!(String::from_utf8_lossy(&output.stderr).contains("符号链接"));
    fs::remove_file(target.join("AGENTS.md")).expect("symlink should be removable");
    fs::remove_file(outside).expect("outside fixture should be removable");
    fs::remove_dir_all(target).expect("test target should be removable");
}

#[test]
fn verify_template_rejects_an_incomplete_managed_file_set() {
    let target = temp_target("verify-managed-set");
    run_init(&target);
    let metadata_path = target.join(".yss-template.json");
    let mut metadata: serde_json::Value =
        serde_json::from_slice(&fs::read(&metadata_path).expect("metadata should exist"))
            .expect("metadata should be JSON");
    metadata["managedFiles"] = serde_json::json!({});
    fs::write(
        &metadata_path,
        serde_json::to_vec_pretty(&metadata).expect("metadata should serialize"),
    )
    .expect("metadata should be writable");
    let output = Command::new(env!("CARGO_BIN_EXE_create-yss-spec"))
        .args([
            "verify-template",
            "--target-dir",
            target.to_str().expect("target should be utf-8"),
        ])
        .env("PATH", "")
        .output()
        .expect("run native verify");
    assert_eq!(output.status.code(), Some(1));
    assert!(String::from_utf8_lossy(&output.stderr).contains("managedFiles 集合"));
    fs::remove_dir_all(target).expect("test target should be removable");
}
