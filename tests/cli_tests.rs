use assert_cmd::Command;
use predicates::prelude::*;
use predicates::str::contains;
use std::error::Error as StdError;
use std::fs;

#[test]
fn test_list_formatting() -> Result<(), Box<dyn StdError>> {
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
    cmd.env("BLUP_ROOT", root).current_dir(root).arg("list");

    cmd.assert()
        .success()
        .stdout(contains("Installed Blender Versions:"))
        .stdout(contains("* 5.0.0 (default)")) // Green color codes might make exact match hard, but text should be there
        .stdout(contains("• 4.5.1"))
        .stdout(contains("• 5.1.0-alpha"));

    Ok(())
}

#[test]
fn test_list_active_vs_default() -> Result<(), Box<dyn StdError>> {
    let temp = tempfile::tempdir()?;
    let root = temp.path();
    let versions_dir = root.join("versions");
    fs::create_dir_all(&versions_dir)?;

    // Create fake versions
    fs::create_dir(versions_dir.join("4.5.3"))?;
    fs::create_dir(versions_dir.join("4.5.4"))?;

    // Create config with default = 4.5.3
    let config_dir = root.join("config");
    fs::create_dir_all(&config_dir)?;
    fs::write(
        config_dir.join("settings.toml"),
        r#"default_version = "4.5.3""#,
    )?;

    // Create .blender-version with 4.5.4 (Override)
    fs::write(root.join(".blender-version"), "4.5.4")?;

    let mut cmd = Command::new(env!("CARGO_BIN_EXE_blup"));
    cmd.env("BLUP_ROOT", root)
        .current_dir(root) // Important to pick up .blender-version
        .arg("list");

    cmd.assert()
        .success()
        .stdout(predicate::str::is_match(r"•\s+4\.5\.3.*\(default\)")?)
        .stdout(predicate::str::is_match(r"\*\s+4\.5\.4.*\(active\)")?);

    Ok(())
}

#[test]
fn test_install_default_flag_existing_version() -> Result<(), Box<dyn StdError>> {
    let temp = tempfile::tempdir()?;
    let root = temp.path();
    let versions_dir = root.join("versions");

    // Simulate existing version
    fs::create_dir_all(versions_dir.join("3.6.0"))?;

    let mut cmd = Command::new(env!("CARGO_BIN_EXE_blup"));
    cmd.env("BLUP_ROOT", root)
        .args(["install", "3.6.0", "--default"]);

    cmd.assert()
        .success()
        .stdout(contains("already installed"))
        .stdout(contains("Default Blender version set to 3.6.0"));

    // Verify config file
    let config_content = fs::read_to_string(root.join("config/settings.toml"))?;
    assert!(config_content.contains(r#"default_version = "3.6.0""#));

    Ok(())
}

#[test]
fn test_install_no_args_no_file() -> Result<(), Box<dyn StdError>> {
    let temp = tempfile::tempdir()?;
    let root = temp.path();

    let mut cmd = Command::new(env!("CARGO_BIN_EXE_blup"));
    cmd.env("BLUP_ROOT", root).current_dir(root).arg("install");

    cmd.assert()
        .failure()
        .stderr(contains("No version specified"));

    Ok(())
}

#[test]
fn test_install_from_file() -> Result<(), Box<dyn StdError>> {
    let temp = tempfile::tempdir()?;
    let root = temp.path();
    let versions_dir = root.join("versions");

    fs::create_dir_all(versions_dir.join("5.0.0"))?;

    fs::write(root.join(".blender-version"), "5.0.0")?;

    let mut cmd = Command::new(env!("CARGO_BIN_EXE_blup"));
    cmd.env("BLUP_ROOT", root).current_dir(root).arg("install");

    cmd.assert()
        .success()
        .stderr(contains("Found .blender-version: 5.0.0"))
        .stdout(contains("already installed"));

    Ok(())
}

#[test]
fn test_install_from_file_invalid() -> Result<(), Box<dyn StdError>> {
    let temp = tempfile::tempdir()?;
    let root = temp.path();

    fs::write(root.join(".blender-version"), "../invalid")?;

    let mut cmd = Command::new(env!("CARGO_BIN_EXE_blup"));
    cmd.env("BLUP_ROOT", root).current_dir(root).arg("install");

    cmd.assert()
        .failure()
        .stderr(contains("not a valid version string"));

    Ok(())
}

#[test]
fn test_install_from_file_empty() -> Result<(), Box<dyn StdError>> {
    let temp = tempfile::tempdir()?;
    let root = temp.path();

    fs::write(root.join(".blender-version"), "   \n")?;

    let mut cmd = Command::new(env!("CARGO_BIN_EXE_blup"));
    cmd.env("BLUP_ROOT", root).current_dir(root).arg("install");

    cmd.assert()
        .failure()
        .stderr(contains("Found .blender-version but it is empty"));

    Ok(())
}

#[test]
fn test_install_conflict_with_existing_link() -> Result<(), Box<dyn StdError>> {
    let temp = tempfile::tempdir()?;
    let root = temp.path();

    // Register a link named "5.0.0"
    let config_dir = root.join("config");
    fs::create_dir_all(&config_dir)?;
    fs::write(
        config_dir.join("settings.toml"),
        r#"
[links]
"5.0.0" = { path = "/dummy/blender" }
"#,
    )?;

    let mut cmd = Command::new(env!("CARGO_BIN_EXE_blup"));
    cmd.env("BLUP_ROOT", root).args(["install", "5.0.0"]);

    cmd.assert()
        .failure()
        .stderr(contains("a link with the same name already exists"));

    Ok(())
}

#[test]
fn test_default_command_lifecycle() -> Result<(), Box<dyn StdError>> {
    let temp = tempfile::tempdir()?;
    let root = temp.path();
    let versions_dir = root.join("versions");
    fs::create_dir_all(versions_dir.join("4.2.0"))?;

    // Show default when none is set
    let mut show_cmd = Command::new(env!("CARGO_BIN_EXE_blup"));
    show_cmd.env("BLUP_ROOT", root).arg("default");
    show_cmd
        .assert()
        .success()
        .stdout(contains("No default version set."));

    // Setting an uninstalled / unlinked version fails
    let mut fail_cmd = Command::new(env!("CARGO_BIN_EXE_blup"));
    fail_cmd
        .env("BLUP_ROOT", root)
        .args(["default", "non-existent-version"]);
    fail_cmd
        .assert()
        .failure()
        .stderr(contains("is not installed"));

    // Setting an installed version succeeds
    let mut set_cmd = Command::new(env!("CARGO_BIN_EXE_blup"));
    set_cmd.env("BLUP_ROOT", root).args(["default", "4.2.0"]);
    set_cmd
        .assert()
        .success()
        .stdout(contains("Default Blender version set to 4.2.0"));

    // Show default shows the set version
    let mut show_set_cmd = Command::new(env!("CARGO_BIN_EXE_blup"));
    show_set_cmd.env("BLUP_ROOT", root).arg("default");
    show_set_cmd
        .assert()
        .success()
        .stdout(contains("Current default: 4.2.0"));

    Ok(())
}

#[test]
fn test_resolve_priority() -> Result<(), Box<dyn StdError>> {
    let temp = tempfile::tempdir()?;
    let root = temp.path();

    // Case 1: No version specified at all -> fails
    let mut cmd_fail = Command::new(env!("CARGO_BIN_EXE_blup"));
    cmd_fail
        .env("BLUP_ROOT", root)
        .current_dir(root)
        .arg("resolve");
    cmd_fail
        .assert()
        .failure()
        .stderr(contains("No version specified"));

    // Case 2: Settings default is set
    let config_dir = root.join("config");
    fs::create_dir_all(&config_dir)?;
    fs::write(
        config_dir.join("settings.toml"),
        r#"default_version = "4.2.0""#,
    )?;

    let mut cmd_default = Command::new(env!("CARGO_BIN_EXE_blup"));
    cmd_default
        .env("BLUP_ROOT", root)
        .current_dir(root)
        .arg("resolve");
    cmd_default.assert().success().stdout(contains("4.2.0"));

    // Case 3: .blender-version overrides default
    fs::write(root.join(".blender-version"), "5.0.0")?;

    let mut cmd_file = Command::new(env!("CARGO_BIN_EXE_blup"));
    cmd_file
        .env("BLUP_ROOT", root)
        .current_dir(root)
        .arg("resolve");
    cmd_file.assert().success().stdout(contains("5.0.0"));

    // Case 4: CLI argument overrides .blender-version
    let mut cmd_arg = Command::new(env!("CARGO_BIN_EXE_blup"));
    cmd_arg
        .env("BLUP_ROOT", root)
        .current_dir(root)
        .args(["resolve", "5.1.0"]);
    cmd_arg.assert().success().stdout(contains("5.1.0"));

    Ok(())
}

#[test]
fn test_remove_clears_default_version() -> Result<(), Box<dyn StdError>> {
    let temp = tempfile::tempdir()?;
    let root = temp.path();
    let versions_dir = root.join("versions");
    fs::create_dir_all(versions_dir.join("4.2.0"))?;

    let config_dir = root.join("config");
    fs::create_dir_all(&config_dir)?;
    fs::write(
        config_dir.join("settings.toml"),
        r#"default_version = "4.2.0""#,
    )?;

    // Remove the version with -y
    let mut rm_cmd = Command::new(env!("CARGO_BIN_EXE_blup"));
    rm_cmd
        .env("BLUP_ROOT", root)
        .args(["remove", "4.2.0", "-y"]);
    rm_cmd
        .assert()
        .success()
        .stdout(contains("Cleared default version."))
        .stdout(contains("uninstalled successfully"));

    // Verify default_version is cleared in settings.toml
    let content = fs::read_to_string(config_dir.join("settings.toml"))?;
    assert!(!content.contains("4.2.0"));

    // Try removing a non-installed non-linked version
    let mut rm_non_existent = Command::new(env!("CARGO_BIN_EXE_blup"));
    rm_non_existent
        .env("BLUP_ROOT", root)
        .args(["remove", "non-existent", "-y"]);
    rm_non_existent
        .assert()
        .success()
        .stdout(contains("is not installed"));

    Ok(())
}

#[test]
fn test_which_uninstalled_and_no_default() -> Result<(), Box<dyn StdError>> {
    let temp = tempfile::tempdir()?;
    let root = temp.path();

    // which with no args and no default fails
    let mut which_no_default = Command::new(env!("CARGO_BIN_EXE_blup"));
    which_no_default
        .env("BLUP_ROOT", root)
        .current_dir(root)
        .arg("which");
    which_no_default
        .assert()
        .failure()
        .stderr(contains("No version specified"));

    // which with specific uninstalled version fails
    let mut which_missing = Command::new(env!("CARGO_BIN_EXE_blup"));
    which_missing
        .env("BLUP_ROOT", root)
        .args(["which", "9.9.9"]);
    which_missing
        .assert()
        .failure()
        .stderr(contains("is not installed or linked"));

    Ok(())
}
