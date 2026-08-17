use std::process::Command;

#[test]
fn displays_the_fixed_node_2_help_oracle() {
    let output = Command::new(env!("CARGO_BIN_EXE_create-yss-spec"))
        .arg("--help")
        .output()
        .expect("run create-yss-spec");

    assert_eq!(output.status.code(), Some(0));
    assert_eq!(
        String::from_utf8(output.stdout).expect("utf-8 stdout"),
        include_str!("../fixtures/node-oracle/help.txt")
    );
    assert_eq!(String::from_utf8(output.stderr).expect("utf-8 stderr"), "");
}

#[test]
fn short_help_alias_displays_the_same_oracle() {
    let output = Command::new(env!("CARGO_BIN_EXE_create-yss-spec"))
        .arg("-h")
        .output()
        .expect("run create-yss-spec");

    assert_eq!(output.status.code(), Some(0));
    assert_eq!(
        String::from_utf8(output.stdout).expect("utf-8 stdout"),
        include_str!("../fixtures/node-oracle/help.txt")
    );
    assert_eq!(String::from_utf8(output.stderr).expect("utf-8 stderr"), "");
}

#[test]
fn unimplemented_command_fails_closed() {
    let output = Command::new(env!("CARGO_BIN_EXE_create-yss-spec"))
        .arg("attach")
        .output()
        .expect("run create-yss-spec");

    assert_eq!(output.status.code(), Some(2));
    assert!(output.stdout.is_empty());
    assert_eq!(
        String::from_utf8(output.stderr).expect("utf-8 stderr"),
        "当前构建仅实现 create-yss-spec --help oracle\n"
    );
}
