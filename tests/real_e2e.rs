use assert_cmd::Command;
use predicates::prelude::*;
use tempfile::TempDir;

#[tokio::test]
#[ignore]
async fn test_real_server_install_and_remove() -> anyhow::Result<()> {
    let temp_home = TempDir::new()?;
    let home_path = temp_home.path().to_str().unwrap();
    let env_home_key = if cfg!(windows) { "UserProfile" } else { "HOME" };

    let target_version = "4.5.0";

    println!("Using temp home: {}", home_path);

    println!(
        "==> Installing Blender {} from official server...",
        target_version
    );

    let mut cmd_install = Command::new(env!("CARGO_BIN_EXE_blup"));
    cmd_install
        .env(env_home_key, home_path)
        .arg("install")
        .arg(target_version);

    cmd_install.assert().success();

    println!("==> Verifying installation in list...");

    let mut cmd_list = Command::new(env!("CARGO_BIN_EXE_blup"));
    cmd_list.env(env_home_key, home_path).arg("list");

    cmd_list
        .assert()
        .success()
        .stdout(predicate::str::contains(target_version));

    println!("==> Uninstalling...");

    let mut cmd_uninstall = Command::new(env!("CARGO_BIN_EXE_blup"));
    cmd_uninstall
        .env(env_home_key, home_path)
        .arg("remove")
        .arg(target_version)
        .arg("-y");

    cmd_uninstall.assert().success();

    Ok(())
}
