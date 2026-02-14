use assert_cmd::Command;
use predicates::prelude::*;
use std::path::PathBuf;
use tempfile::TempDir;

struct TestEnv {
    root: TempDir,
    bin_path: PathBuf,
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

    let target_version = "5.0.1";

    println!("Using temp home: {:?}", env.root.path());

    println!("Step 1: Checking remote list...");
    env.blup()
        .arg("list")
        .arg("--remote")
        .assert()
        .success()
        .stdout(predicate::str::contains(target_version));

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
        .stdout(predicate::str::contains(target_version));

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
        .stdout(predicate::str::contains(target_version));

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
        .stdout(predicate::str::contains(
            full_expected_path.to_str().unwrap(),
        ));

    println!("Step 5: Testing .blender-version priority...");

    let version_file = env.root.path().join(".blender-version");
    let dummy_version = "99.9.9";
    tokio::fs::write(&version_file, dummy_version).await?;

    env.blup()
        .current_dir(env.root.path())
        .arg("resolve")
        .assert()
        .success()
        .stdout(predicate::str::contains(dummy_version))
        .stdout(predicate::str::contains("5.0.1").not());

    tokio::fs::remove_file(&version_file).await?;

    println!("Step 6: Dry-run execution...");

    env.blup()
        .arg("run")
        .arg("--")
        .arg("--version")
        .assert()
        .success()
        .stdout(predicate::str::contains("Blender"));

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
        .stdout(predicate::str::contains(target_version).not());

    Ok(())
}
