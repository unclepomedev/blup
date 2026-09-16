use assert_cmd::Command;
use predicates::prelude::*;
use predicates::str::contains;
use std::path::PathBuf;
use tempfile::TempDir;

struct TestEnv {
    root: TempDir,
    bin_path: PathBuf,
}

fn strip_ansi(s: &str) -> String {
    let mut result = String::new();
    let mut chars = s.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '\x1b' {
            if let Some(&'[') = chars.peek() {
                chars.next();
                // Consume characters until 'm' or end of escape sequence
                for next in chars.by_ref() {
                    if next.is_ascii_alphabetic() {
                        break;
                    }
                }
            }
        } else {
            result.push(c);
        }
    }
    result
}

fn is_valid_version(version: &str) -> bool {
    let parts: Vec<&str> = version.split('.').collect();
    parts.len() == 3
        && parts
            .iter()
            .all(|p| !p.is_empty() && p.chars().all(|c| c.is_ascii_digit()))
}

fn extract_version_candidate(line: &str) -> Option<&str> {
    line.trim()
        .trim_start_matches(|c: char| c == '*' || c.is_whitespace())
        .split_whitespace()
        .next()
}

fn extract_stable_section_lines(output: &str) -> impl Iterator<Item = &str> {
    output
        .lines()
        .map(str::trim)
        .skip_while(|line| !line.starts_with("Stable Releases (Active Support):"))
        .skip(1)
        .take_while(|line| !line.ends_with(':'))
        .filter(|line| !line.is_empty())
}

fn parse_latest_stable_version(output: &str) -> anyhow::Result<String> {
    let plain_output = strip_ansi(output);

    let candidate = extract_stable_section_lines(&plain_output)
        .find_map(extract_version_candidate)
        .ok_or_else(|| {
            anyhow::anyhow!(
                "Failed to find any stable release version in `blup list --remote` output:\n{}",
                plain_output
            )
        })?;

    if is_valid_version(candidate) {
        Ok(candidate.to_string())
    } else {
        anyhow::bail!(
            "Parsed candidate '{}' in Stable Releases does not look like a valid version format",
            candidate
        )
    }
}

#[test]
fn test_helper_parse_latest_stable_version() {
    let sample_output = r#"
Fetching remote versions...

Daily Builds (builder.blender.org):
  5.3.0-alpha (Alpha, 931bb2e)

Stable Releases (Active Support):
  5.2.2 (LTS)
  5.1.2
* 5.0.1 (Installed)
  4.5.14 (LTS)
  4.4.3
"#;
    let version = parse_latest_stable_version(sample_output).unwrap();
    assert_eq!(version, "5.2.2");

    let sample_installed_first = r#"
Stable Releases (Active Support):
* 5.2.2 (LTS, Installed)
  5.1.2
"#;
    let version = parse_latest_stable_version(sample_installed_first).unwrap();
    assert_eq!(version, "5.2.2");

    let missing_section = "Daily Builds (builder.blender.org):\n  5.3.0-alpha\n";
    assert!(parse_latest_stable_version(missing_section).is_err());

    let invalid_double_dot = "Stable Releases (Active Support):\n  5..2\n";
    assert!(parse_latest_stable_version(invalid_double_dot).is_err());

    let invalid_four_parts = "Stable Releases (Active Support):\n  5.2.2.1\n";
    assert!(parse_latest_stable_version(invalid_four_parts).is_err());

    let invalid_two_parts = "Stable Releases (Active Support):\n  5.2\n";
    assert!(parse_latest_stable_version(invalid_two_parts).is_err());
}

impl TestEnv {
    fn new() -> anyhow::Result<Self> {
        let root = TempDir::new()?;
        let bin_path = PathBuf::from(env!("CARGO_BIN_EXE_blup"));
        Ok(Self { root, bin_path })
    }

    fn blup(&self) -> Command {
        let mut cmd = Command::new(&self.bin_path);
        cmd.env("BLUP_ROOT", self.root.path());
        cmd
    }

    fn versions_dir(&self) -> PathBuf {
        self.root.path().join("versions")
    }
}

#[tokio::test]
#[ignore]
async fn test_e2e_lifecycle() -> anyhow::Result<()> {
    let env = TestEnv::new()?;

    println!("Using temp home: {:?}", env.root.path());

    println!("Step 1: Checking remote list...");
    let output = env.blup().arg("list").arg("--remote").output()?;
    if !output.status.success() {
        anyhow::bail!(
            "`blup list --remote` failed with status {:?}:\n{}",
            output.status,
            String::from_utf8_lossy(&output.stderr)
        );
    }
    let stdout = String::from_utf8(output.stdout)?;

    let target_version = parse_latest_stable_version(&stdout)?;
    let target_version = target_version.as_str();
    println!("Dynamically selected target version: {}", target_version);

    println!("Step 2: Installing {}...", target_version);
    env.blup()
        .arg("install")
        .arg(target_version)
        .assert()
        .success();

    env.blup()
        .arg("list")
        .assert()
        .success()
        .stdout(contains(target_version));

    println!("Step 3: Setting default version...");
    env.blup()
        .arg("default")
        .arg(target_version)
        .assert()
        .success();

    env.blup()
        .arg("default")
        .assert()
        .success()
        .stdout(contains(target_version));

    println!("Step 4: Verifying binary path resolution...");

    let expected_bin_suffix = if cfg!(target_os = "macos") {
        PathBuf::from("Blender.app/Contents/MacOS/Blender")
    } else if cfg!(windows) {
        PathBuf::from("blender.exe")
    } else {
        PathBuf::from("blender")
    };

    let full_expected_path = env
        .versions_dir()
        .join(target_version)
        .join(&expected_bin_suffix);

    if !full_expected_path.exists() {
        panic!("Binary not found at: {:?}", full_expected_path);
    }

    env.blup()
        .arg("which")
        .assert()
        .success()
        .stdout(contains(full_expected_path.to_str().unwrap()));

    println!("Step 5: Testing .blender-version priority...");

    let version_file = env.root.path().join(".blender-version");
    let dummy_version = "99.9.9";
    tokio::fs::write(&version_file, dummy_version).await?;

    env.blup()
        .current_dir(env.root.path())
        .arg("resolve")
        .assert()
        .success()
        .stdout(contains(dummy_version))
        .stdout(contains(target_version).not());

    tokio::fs::remove_file(&version_file).await?;

    println!("Step 6: Dry-run execution...");

    env.blup()
        .arg("run")
        .arg("--")
        .arg("--version")
        .assert()
        .success()
        .stdout(contains("Blender"));

    println!("Step 7: Uninstalling...");
    env.blup()
        .arg("remove")
        .arg(target_version)
        .arg("-y")
        .assert()
        .success();

    if full_expected_path.exists() {
        panic!("Binary still exists after removal!");
    }

    env.blup()
        .arg("list")
        .assert()
        .success()
        .stdout(contains(target_version).not());

    println!("Step 8: Testing linked executable in E2E lifecycle...");
    // Create a dummy mock executable in another folder outside BLUP_ROOT
    let external_dir = tempfile::tempdir()?;
    #[cfg(windows)]
    let mock_bin = external_dir.path().join("external_blender.bat");
    #[cfg(not(windows))]
    let mock_bin = external_dir.path().join("external_blender");

    #[cfg(windows)]
    std::fs::write(&mock_bin, "@echo off\r\necho Blender 5.3.0-custom\r\n")?;
    #[cfg(not(windows))]
    {
        use std::fs;
        use std::os::unix::fs::PermissionsExt;
        fs::write(&mock_bin, "#!/bin/sh\necho 'Blender 5.3.0-custom'\n")?;
        let mut perms = fs::metadata(&mock_bin)?.permissions();
        perms.set_mode(0o755);
        fs::set_permissions(&mock_bin, perms)?;
    }

    // Link it
    env.blup()
        .args(["link", mock_bin.to_str().unwrap(), "--as", "linked-custom"])
        .assert()
        .success();

    // Verify which resolves to the external mock binary
    env.blup()
        .args(["which", "linked-custom"])
        .assert()
        .success()
        .stdout(contains(mock_bin.to_str().unwrap()));

    // Verify running works
    env.blup()
        .args(["run", "linked-custom", "--", "--version"])
        .assert()
        .success()
        .stdout(contains("Blender 5.3.0-custom"));

    // Remove the linked entry
    env.blup()
        .args(["remove", "linked-custom", "-y"])
        .assert()
        .success();

    // Verify mock binary still exists externally
    assert!(mock_bin.exists());

    Ok(())
}
