use assert_cmd::Command;
use blup::core::config::Settings;
use predicates::str::contains;
use std::error::Error as StdError;
use std::fs;
#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};

fn create_mock_executable(dir: &Path, content: &str) -> Result<PathBuf, Box<dyn StdError>> {
    #[cfg(windows)]
    let mock_bin = dir.join("mock_blender.bat");
    #[cfg(not(windows))]
    let mock_bin = dir.join("mock_blender");

    #[cfg(windows)]
    {
        let win_content = if content.is_empty() {
            "@echo off\r\n".to_string()
        } else {
            format!(
                "@echo off\r\n{}\r\n",
                content.replace("#!/bin/sh\n", "").replace('\'', "")
            )
        };
        fs::write(&mock_bin, win_content)?;
    }
    #[cfg(not(windows))]
    {
        let sh_content = if content.is_empty() {
            "#!/bin/sh\n".to_string()
        } else {
            content.to_string()
        };
        fs::write(&mock_bin, sh_content)?;
    }

    #[cfg(unix)]
    {
        let mut perms = fs::metadata(&mock_bin)?.permissions();
        perms.set_mode(0o755);
        fs::set_permissions(&mock_bin, perms)?;
    }

    Ok(mock_bin)
}

#[test]
fn test_link_validation_and_success() -> Result<(), Box<dyn StdError>> {
    let temp = tempfile::tempdir()?;
    let root = temp.path();

    let mock_bin = create_mock_executable(root, "#!/bin/sh\necho 'Blender 4.2.0'\n")?;

    let mut cmd = Command::new(env!("CARGO_BIN_EXE_blup"));
    cmd.env("BLUP_ROOT", root).current_dir(root).args([
        "link",
        mock_bin.to_str().unwrap(),
        "--as",
        "4.2-custom",
    ]);

    cmd.assert()
        .success()
        .stdout(contains("Linked '4.2-custom'"));

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
        .stdout(contains("Linked Blender Executables:"))
        .stdout(contains("4.2-custom"));

    // Test blup which finds the linked executable
    let mut which_cmd = Command::new(env!("CARGO_BIN_EXE_blup"));
    which_cmd
        .env("BLUP_ROOT", root)
        .current_dir(root)
        .args(["which", "4.2-custom"]);

    which_cmd
        .assert()
        .success()
        .stdout(contains(mock_bin.to_str().unwrap()));

    // Test blup default can set the link
    let mut default_cmd = Command::new(env!("CARGO_BIN_EXE_blup"));
    default_cmd
        .env("BLUP_ROOT", root)
        .current_dir(root)
        .args(["default", "4.2-custom"]);

    default_cmd
        .assert()
        .success()
        .stdout(contains("Default Blender version set to 4.2-custom"));

    // Test blup which without args now uses default
    let mut which_def_cmd = Command::new(env!("CARGO_BIN_EXE_blup"));
    which_def_cmd
        .env("BLUP_ROOT", root)
        .current_dir(root)
        .arg("which");

    which_def_cmd
        .assert()
        .success()
        .stdout(contains(mock_bin.to_str().unwrap()));

    // Test blup run executes the linked binary
    let mut run_cmd = Command::new(env!("CARGO_BIN_EXE_blup"));
    run_cmd
        .env("BLUP_ROOT", root)
        .current_dir(root)
        .args(["run", "4.2-custom"]);

    run_cmd
        .assert()
        .success()
        .stdout(contains("Starting Blender 4.2-custom (linked)"));

    Ok(())
}

#[test]
fn test_link_conflict_and_force() -> Result<(), Box<dyn StdError>> {
    let temp = tempfile::tempdir()?;
    let root = temp.path();

    let mock_bin = create_mock_executable(root, "#!/bin/sh\necho 'Blender 5.0.0'\n")?;

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
        .stderr(contains("already exists"));

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

    // Linking with the name of an installed version must always fail, even with --force
    let versions_dir = root.join("versions");
    fs::create_dir_all(versions_dir.join("5.0.0"))?;

    let mut cmd_installed_force = Command::new(env!("CARGO_BIN_EXE_blup"));
    cmd_installed_force.env("BLUP_ROOT", root).args([
        "link",
        mock_bin.to_str().unwrap(),
        "--as",
        "5.0.0",
        "--force",
    ]);
    cmd_installed_force
        .assert()
        .failure()
        .stderr(contains("installed version"));

    Ok(())
}

#[test]
fn test_link_broken_path() -> Result<(), Box<dyn StdError>> {
    let temp = tempfile::tempdir()?;
    let root = temp.path();

    let mock_bin = create_mock_executable(root, "")?;

    let mut cmd = Command::new(env!("CARGO_BIN_EXE_blup"));
    cmd.env("BLUP_ROOT", root)
        .args(["link", mock_bin.to_str().unwrap(), "--as", "broken-test"]);
    cmd.assert().success();

    // Remove the file to simulate deletion/moving
    fs::remove_file(&mock_bin)?;

    // list should show broken link warning
    let mut list_cmd = Command::new(env!("CARGO_BIN_EXE_blup"));
    list_cmd.env("BLUP_ROOT", root).arg("list");
    list_cmd.assert().success().stdout(contains("broken link"));

    // run should fail with clear error
    let mut run_cmd = Command::new(env!("CARGO_BIN_EXE_blup"));
    run_cmd.env("BLUP_ROOT", root).args(["run", "broken-test"]);
    run_cmd
        .assert()
        .failure()
        .stderr(contains("does not exist"));

    // which should fail with clear error
    let mut which_cmd = Command::new(env!("CARGO_BIN_EXE_blup"));
    which_cmd
        .env("BLUP_ROOT", root)
        .args(["which", "broken-test"]);
    which_cmd
        .assert()
        .failure()
        .stderr(contains("does not exist"));

    Ok(())
}

#[test]
fn test_remove_linked_entry() -> Result<(), Box<dyn StdError>> {
    let temp = tempfile::tempdir()?;
    let root = temp.path();

    let mock_bin = create_mock_executable(root, "")?;

    let mut cmd = Command::new(env!("CARGO_BIN_EXE_blup"));
    cmd.env("BLUP_ROOT", root)
        .args(["link", mock_bin.to_str().unwrap(), "--as", "removable-link"]);
    cmd.assert().success();

    // Remove link with -y
    let mut rm_cmd = Command::new(env!("CARGO_BIN_EXE_blup"));
    rm_cmd
        .env("BLUP_ROOT", root)
        .args(["remove", "removable-link", "-y"]);
    rm_cmd
        .assert()
        .success()
        .stdout(contains("Link 'removable-link' removed successfully"));

    // Ensure mock executable was NOT deleted!
    assert!(mock_bin.exists());

    // Ensure settings no longer has it
    let settings = config_load(root)?;
    assert!(!settings.links.contains_key("removable-link"));

    Ok(())
}

#[test]
fn test_remove_linked_entry_clears_default() -> Result<(), Box<dyn StdError>> {
    let temp = tempfile::tempdir()?;
    let root = temp.path();

    let mock_bin = create_mock_executable(root, "")?;

    let mut cmd = Command::new(env!("CARGO_BIN_EXE_blup"));
    cmd.env("BLUP_ROOT", root)
        .args(["link", mock_bin.to_str().unwrap(), "--as", "default-link"]);
    cmd.assert().success();

    // Set as default
    let mut def_cmd = Command::new(env!("CARGO_BIN_EXE_blup"));
    def_cmd
        .env("BLUP_ROOT", root)
        .args(["default", "default-link"]);
    def_cmd.assert().success();

    let settings = config_load(root)?;
    assert_eq!(settings.default_version.as_deref(), Some("default-link"));

    // Remove the link
    let mut rm_cmd = Command::new(env!("CARGO_BIN_EXE_blup"));
    rm_cmd
        .env("BLUP_ROOT", root)
        .args(["remove", "default-link", "-y"]);
    rm_cmd
        .assert()
        .success()
        .stdout(contains("Cleared default version."));

    let settings_after = config_load(root)?;
    assert_eq!(settings_after.default_version, None);

    Ok(())
}

#[test]
fn test_link_validation_errors() -> Result<(), Box<dyn StdError>> {
    let temp = tempfile::tempdir()?;
    let root = temp.path();

    let mock_bin = create_mock_executable(root, "")?;

    // Invalid alias name with path separators
    let mut cmd_sep = Command::new(env!("CARGO_BIN_EXE_blup"));
    cmd_sep.env("BLUP_ROOT", root).args([
        "link",
        mock_bin.to_str().unwrap(),
        "--as",
        "custom/nested",
    ]);
    cmd_sep
        .assert()
        .failure()
        .stderr(contains("cannot contain path separators"));

    // Invalid alias name with parent directory traversal
    let mut cmd_parent = Command::new(env!("CARGO_BIN_EXE_blup"));
    cmd_parent
        .env("BLUP_ROOT", root)
        .args(["link", mock_bin.to_str().unwrap(), "--as", ".."]);
    cmd_parent
        .assert()
        .failure()
        .stderr(contains("not a valid"));

    // Empty alias name
    let mut cmd_empty = Command::new(env!("CARGO_BIN_EXE_blup"));
    cmd_empty
        .env("BLUP_ROOT", root)
        .args(["link", mock_bin.to_str().unwrap(), "--as", "   "]);
    cmd_empty
        .assert()
        .failure()
        .stderr(contains("Name cannot be empty"));

    // Non-existent path
    let non_existent = root.join("non_existent_blender");
    let mut cmd_notfound = Command::new(env!("CARGO_BIN_EXE_blup"));
    cmd_notfound.env("BLUP_ROOT", root).args([
        "link",
        non_existent.to_str().unwrap(),
        "--as",
        "test-valid-name",
    ]);
    cmd_notfound
        .assert()
        .failure()
        .stderr(contains("does not exist"));

    // Directory path (not a regular file)
    let dir_path = root.join("dummy_dir");
    fs::create_dir(&dir_path)?;
    let mut cmd_dir = Command::new(env!("CARGO_BIN_EXE_blup"));
    cmd_dir
        .env("BLUP_ROOT", root)
        .args(["link", dir_path.to_str().unwrap(), "--as", "test-dir"]);
    cmd_dir
        .assert()
        .failure()
        .stderr(contains("Path is not a regular file"));

    // Relative path (must be absolute)
    let mut cmd_relative = Command::new(env!("CARGO_BIN_EXE_blup"));
    cmd_relative.env("BLUP_ROOT", root).current_dir(root).args([
        "link",
        "relative/path/blender",
        "--as",
        "test-rel",
    ]);
    cmd_relative
        .assert()
        .failure()
        .stderr(contains("Path must be an absolute path"));

    Ok(())
}

fn config_load(root: &Path) -> Result<Settings, Box<dyn StdError>> {
    let settings_path = root.join("config").join("settings.toml");
    if !settings_path.exists() {
        return Ok(Default::default());
    }
    let content = fs::read_to_string(settings_path)?;
    let s = toml::from_str(&content)?;
    Ok(s)
}
