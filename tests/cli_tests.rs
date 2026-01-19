use assert_cmd::Command;
use predicates::prelude::*;
use std::fs;

#[test]
fn test_list_formatting() -> Result<(), Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    let root = temp.path();
    let versions_dir = root.join("versions");
    fs::create_dir_all(&versions_dir)?;

    // Create fake versions
    fs::create_dir(versions_dir.join("4.5.1"))?;
    fs::create_dir(versions_dir.join("5.0.0"))?;
    fs::create_dir(versions_dir.join("5.1.0-alpha"))?;

    // Create config with default
    let config_dir = root.join("config");
    fs::create_dir_all(&config_dir)?;
    fs::write(
        config_dir.join("settings.toml"),
        r#"default_version = "5.0.0""#,
    )?;

    let mut cmd = Command::new(env!("CARGO_BIN_EXE_blup"));
    cmd.env("BLUP_ROOT", root).arg("list");

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("Installed Blender Versions:"))
        .stdout(predicate::str::contains("* 5.0.0 (default)")) // Green color codes might make exact match hard, but text should be there
        .stdout(predicate::str::contains("• 4.5.1"))
        .stdout(predicate::str::contains("• 5.1.0-alpha"));

    Ok(())
}

#[test]
fn test_install_default_flag_existing_version() -> Result<(), Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    let root = temp.path();
    let versions_dir = root.join("versions");

    // Simulate existing version
    fs::create_dir_all(versions_dir.join("3.6.0"))?;

    let mut cmd = Command::new(env!("CARGO_BIN_EXE_blup"));
    cmd.env("BLUP_ROOT", root)
        .args(&["install", "3.6.0", "--default"]);

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("already installed"))
        .stdout(predicate::str::contains(
            "Default Blender version set to 3.6.0",
        ));

    // Verify config file
    let config_content = fs::read_to_string(root.join("config/settings.toml"))?;
    assert!(config_content.contains(r#"default_version = "3.6.0""#));

    Ok(())
}
