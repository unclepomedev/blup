use assert_cmd::Command;
use predicates::prelude::*;
use tempfile::TempDir;

#[tokio::test]
#[ignore]
async fn test_real_server_install_and_remove() -> anyhow::Result<()> {
    let temp_home = TempDir::new()?;
    let home_path = temp_home.path().to_str().unwrap();

    let target_version = "4.5.0";

    println!("Using temp home (BLUP_ROOT): {}", home_path);

    println!(
        "==> Installing Blender {} from official server...",
        target_version
    );

    let mut cmd_install = Command::new(env!("CARGO_BIN_EXE_blup"));
    cmd_install
        .env("BLUP_ROOT", home_path)
        .arg("install")
        .arg(target_version);

    let output = cmd_install
        .output()
        .expect("Failed to execute install command");

    if !output.status.success() {
        println!("--- Install stdout ---");
        println!("{}", String::from_utf8_lossy(&output.stdout));
        println!("--- Install stderr ---");
        println!("{}", String::from_utf8_lossy(&output.stderr));
        panic!("Install command failed with status: {}", output.status);
    }

    println!("==> Verifying installation in list...");

    let mut cmd_list = Command::new(env!("CARGO_BIN_EXE_blup"));
    cmd_list.env("BLUP_ROOT", home_path).arg("list");

    cmd_list
        .assert()
        .success()
        .stdout(predicate::str::contains(target_version));

    #[cfg(target_os = "macos")]
    let bin_path = temp_home
        .path()
        .join("versions")
        .join(target_version)
        .join("Blender.app")
        .join("Contents")
        .join("MacOS")
        .join("Blender");

    #[cfg(not(target_os = "macos"))]
    let bin_path = temp_home
        .path()
        .join("versions")
        .join(target_version)
        .join(bin_name);

    if !bin_path.exists() {
        panic!(
            "Binary not found at expected path: {:?}. \nCheck extraction logic (strip components).",
            bin_path
        );
    }

    println!("==> Uninstalling...");

    let mut cmd_uninstall = Command::new(env!("CARGO_BIN_EXE_blup"));
    cmd_uninstall
        .env("BLUP_ROOT", home_path)
        .arg("remove")
        .arg(target_version)
        .arg("-y");

    let output_uninstall = cmd_uninstall
        .output()
        .expect("Failed to execute remove command");
    if !output_uninstall.status.success() {
        println!("--- Remove stdout ---");
        println!("{}", String::from_utf8_lossy(&output_uninstall.stdout));
        println!("--- Remove stderr ---");
        println!("{}", String::from_utf8_lossy(&output_uninstall.stderr));
        panic!("Remove command failed");
    }

    let mut cmd_list_after = Command::new(env!("CARGO_BIN_EXE_blup"));
    cmd_list_after.env("BLUP_ROOT", home_path).arg("list");

    cmd_list_after
        .assert()
        .success()
        .stdout(predicate::str::contains(target_version).not());

    Ok(())
}
