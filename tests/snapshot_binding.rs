use sha2::{Digest, Sha256};
use std::process::Command;
use tar::Archive;

const SNAPSHOT_COMMIT: &str = "68c367a13d5006cca83f1c5e369678af28c4bf15";
const SNAPSHOT_HASH: &str = "f4276bfa8e6ca7781f905372d912f8fd9ba806566e212550b4548eda0f877387";
const SNAPSHOT_ARCHIVE_SHA256: &str =
    "f72c6bd76c48247ec31245f150be257b9eeb4388da32a29a5e958d3b2600778e";
const TEMPLATE_MANIFEST_SHA256: &str =
    "48549af09ac85a9e0caf97d9342e8ee31b1cc8b608704bc9f1aa0d546f9a635c";

#[test]
fn frozen_snapshot_hashes_and_public_dry_run_are_bound() {
    let archive = include_bytes!("../assets/template.snapshot.tar");
    let actual = Sha256::digest(archive);
    let actual = actual
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>();
    assert_eq!(actual, SNAPSHOT_ARCHIVE_SHA256);

    let mut archive_reader = Archive::new(std::io::Cursor::new(archive.as_slice()));
    let mut entries = archive_reader
        .entries()
        .expect("snapshot archive should be readable");
    let mut files = 0;
    let mut directories = 0;
    let mut manifest = None;
    let mut snapshot = None;
    for entry in entries.by_ref() {
        let mut entry = entry.expect("snapshot entry should be readable");
        if entry.header().entry_type().is_dir() {
            directories += 1;
        } else {
            assert!(entry.header().entry_type().is_file());
            let path = entry
                .path()
                .expect("snapshot path should be readable")
                .to_string_lossy()
                .trim_start_matches("./")
                .to_owned();
            let mut content = Vec::new();
            std::io::Read::read_to_end(&mut entry, &mut content)
                .expect("snapshot content should be readable");
            match path.as_str() {
                "__yss_runtime/template.manifest.json" => manifest = Some(content),
                "__yss_runtime/template.snapshot.json" => snapshot = Some(content),
                _ => {}
            }
            files += 1;
        }
    }
    assert_eq!(files, 5_235);
    assert_eq!(directories, 2_957);
    assert_eq!(
        Sha256::digest(manifest.expect("manifest binding should be present"))
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect::<String>(),
        TEMPLATE_MANIFEST_SHA256
    );
    let snapshot = snapshot.expect("snapshot binding should be present");
    let snapshot = serde_json::from_slice::<serde_json::Value>(&snapshot)
        .expect("snapshot binding should be JSON");
    assert_eq!(snapshot["templateCommit"], SNAPSHOT_COMMIT);
    assert_eq!(snapshot["snapshotHash"], SNAPSHOT_HASH);

    let target = std::env::temp_dir().join(format!(
        "create-yss-spec-snapshot-binding-{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&target);
    let output = Command::new(env!("CARGO_BIN_EXE_create-yss-spec"))
        .args([
            "--project-name",
            "snapshot-fixture",
            "--business-domain",
            "runtime",
            "--target-dir",
            target.to_str().expect("target should be utf-8"),
            "--dry-run",
        ])
        .env("PATH", "")
        .output()
        .expect("run snapshot dry-run");
    assert_eq!(output.status.code(), Some(0));
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    assert!(stdout.contains(SNAPSHOT_COMMIT));
    assert!(stdout.contains(SNAPSHOT_HASH));
    assert!(!target.exists());
}
