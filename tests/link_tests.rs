use assert_cmd::Command;
use blup::core::config::Settings;
use predicates::prelude::*;
use std::error::Error;
use std::fs;
#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;
use std::path::Path;

#[test]
fn test_link_validation_and_success() -> Result<(), Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    let root = temp.path();

    // Create a mock executable
    let mock_bin = root.join("mock_blender");
    fs::write(&mock_bin, "#!/bin/sh\necho 'Blender 4.2.0'\n")?;
    #[cfg(unix)]
    {
        let mut perms = fs::metadata(&mock_bin)?.permissions();
        perms.set_mode(0o755);
        fs::set_permissions(&mock_bin, perms)?;
    }

    let mut cmd = Command::new(env!("CARGO_BIN_EXE_blup"));
    cmd.env("BLUP_ROOT", root).current_dir(root).args([
        "link",
        mock_bin.to_str().unwrap(),
        "--as",
        "4.2-custom",
    ]);

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("Linked '4.2-custom'"));

    // Verify settings.toml contains the link
    let settings_path = root.join("config").join("settings.toml");
    let content = fs::read_to_string(&settings_path)?;
    assert!(content.contains("4.2-custom"));

    // Test blup list includes the link
    let mut list_cmd = Command::new(env!("CARGO_BIN_EXE_blup"));
    list_cmd
        .env("BLUP_ROOT", root)
        .current_dir(root)
        .arg("list");

    list_cmd
        .assert()
        .success()
        .stdout(predicate::str::contains("Linked Blender Executables:"))
        .stdout(predicate::str::contains("4.2-custom"));

    // Test blup which finds the linked executable
    let mut which_cmd = Command::new(env!("CARGO_BIN_EXE_blup"));
    which_cmd
        .env("BLUP_ROOT", root)
        .current_dir(root)
        .args(["which", "4.2-custom"]);

    which_cmd
        .assert()
        .success()
        .stdout(predicate::str::contains(mock_bin.to_str().unwrap()));

    // Test blup default can set the link
    let mut default_cmd = Command::new(env!("CARGO_BIN_EXE_blup"));
    default_cmd
        .env("BLUP_ROOT", root)
        .current_dir(root)
        .args(["default", "4.2-custom"]);

    default_cmd
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "Default Blender version set to 4.2-custom",
        ));

    // Test blup which without args now uses default
    let mut which_def_cmd = Command::new(env!("CARGO_BIN_EXE_blup"));
    which_def_cmd
        .env("BLUP_ROOT", root)
        .current_dir(root)
        .arg("which");

    which_def_cmd
        .assert()
        .success()
        .stdout(predicate::str::contains(mock_bin.to_str().unwrap()));

    // Test blup run executes the linked binary
    let mut run_cmd = Command::new(env!("CARGO_BIN_EXE_blup"));
    run_cmd
        .env("BLUP_ROOT", root)
        .current_dir(root)
        .args(["run", "4.2-custom"]);

    run_cmd.assert().success().stdout(predicate::str::contains(
        "Starting Blender 4.2-custom (linked)",
    ));

    Ok(())
}

#[test]
fn test_link_conflict_and_force() -> Result<(), Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    let root = temp.path();

    // Create a mock executable
    let mock_bin = root.join("mock_blender");
    fs::write(&mock_bin, "#!/bin/sh\necho 'Blender 5.0.0'\n")?;
    #[cfg(unix)]
    {
        let mut perms = fs::metadata(&mock_bin)?.permissions();
        perms.set_mode(0o755);
        fs::set_permissions(&mock_bin, perms)?;
    }

    // Link once
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_blup"));
    cmd.env("BLUP_ROOT", root)
        .args(["link", mock_bin.to_str().unwrap(), "--as", "test-alias"]);
    cmd.assert().success();

    // Link again without --force should fail
    let mut cmd_fail = Command::new(env!("CARGO_BIN_EXE_blup"));
    cmd_fail.env("BLUP_ROOT", root).args([
        "link",
        mock_bin.to_str().unwrap(),
        "--as",
        "test-alias",
    ]);
    cmd_fail
        .assert()
        .failure()
        .stderr(predicate::str::contains("already exists"));

    // Link with --force should succeed
    let mut cmd_force = Command::new(env!("CARGO_BIN_EXE_blup"));
    cmd_force.env("BLUP_ROOT", root).args([
        "link",
        mock_bin.to_str().unwrap(),
        "--as",
        "test-alias",
        "--force",
    ]);
    cmd_force.assert().success();

    Ok(())
}

#[test]
fn test_link_broken_path() -> Result<(), Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    let root = temp.path();

    let mock_bin = root.join("mock_blender");
    fs::write(&mock_bin, "#!/bin/sh\n")?;
    #[cfg(unix)]
    {
        let mut perms = fs::metadata(&mock_bin)?.permissions();
        perms.set_mode(0o755);
        fs::set_permissions(&mock_bin, perms)?;
    }

    let mut cmd = Command::new(env!("CARGO_BIN_EXE_blup"));
    cmd.env("BLUP_ROOT", root)
        .args(["link", mock_bin.to_str().unwrap(), "--as", "broken-test"]);
    cmd.assert().success();

    // Remove the file to simulate deletion/moving
    fs::remove_file(&mock_bin)?;

    // list should show broken link warning
    let mut list_cmd = Command::new(env!("CARGO_BIN_EXE_blup"));
    list_cmd.env("BLUP_ROOT", root).arg("list");
    list_cmd
        .assert()
        .success()
        .stdout(predicate::str::contains("broken link"));

    // run should fail with clear error
    let mut run_cmd = Command::new(env!("CARGO_BIN_EXE_blup"));
    run_cmd.env("BLUP_ROOT", root).args(["run", "broken-test"]);
    run_cmd
        .assert()
        .failure()
        .stderr(predicate::str::contains("does not exist"));

    // which should fail with clear error
    let mut which_cmd = Command::new(env!("CARGO_BIN_EXE_blup"));
    which_cmd
        .env("BLUP_ROOT", root)
        .args(["which", "broken-test"]);
    which_cmd
        .assert()
        .failure()
        .stderr(predicate::str::contains("does not exist"));

    Ok(())
}

#[test]
fn test_remove_linked_entry() -> Result<(), Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    let root = temp.path();

    let mock_bin = root.join("mock_blender");
    fs::write(&mock_bin, "#!/bin/sh\n")?;
    #[cfg(unix)]
    {
        let mut perms = fs::metadata(&mock_bin)?.permissions();
        perms.set_mode(0o755);
        fs::set_permissions(&mock_bin, perms)?;
    }

    let mut cmd = Command::new(env!("CARGO_BIN_EXE_blup"));
    cmd.env("BLUP_ROOT", root)
        .args(["link", mock_bin.to_str().unwrap(), "--as", "removable-link"]);
    cmd.assert().success();

    // Remove link with -y
    let mut rm_cmd = Command::new(env!("CARGO_BIN_EXE_blup"));
    rm_cmd
        .env("BLUP_ROOT", root)
        .args(["remove", "removable-link", "-y"]);
    rm_cmd.assert().success().stdout(predicate::str::contains(
        "Link 'removable-link' removed successfully",
    ));

    // Ensure mock executable was NOT deleted!
    assert!(mock_bin.exists());

    // Ensure settings no longer has it
    let settings = config_load(root)?;
    assert!(!settings.links.contains_key("removable-link"));

    Ok(())
}

fn config_load(root: &Path) -> Result<Settings, Box<dyn Error>> {
    let settings_path = root.join("config").join("settings.toml");
    if !settings_path.exists() {
        return Ok(Default::default());
    }
    let content = fs::read_to_string(settings_path)?;
    let s = toml::from_str(&content)?;
    Ok(s)
}
