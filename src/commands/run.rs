use crate::core::{config, os, version};
use anyhow::{Context, Result, bail};
use console::style;
use std::fs;
use std::path::Path;
use std::process::Command;

pub fn run(version_arg: Option<String>, scripts: Option<String>, args: Vec<String>) -> Result<()> {
    let (version_arg, args) = prepare_run_args(version_arg, args);
    let mut version_str = config::resolve_version(version_arg)?;

    if version_str == "daily" {
        let actual_version = version::find_latest_daily_installed()?;
        println!(
            "{} Resolved 'daily' to installed version: {}",
            style("i").blue(),
            style(&actual_version).bold()
        );
        version_str = actual_version;
    }

    let app_root = config::get_app_root()?;
    let install_dir = app_root.join("versions").join(&version_str);

    if !install_dir.exists() {
        bail!(
            "Blender {} is not installed. Run `blup install {}` first.",
            version_str,
            version_str
        );
    }

    let bin_path = os::get_bin_path(&install_dir)?;

    println!(
        "{} Starting Blender {}...",
        style("==>").green(),
        version_str
    );

    let mut command = Command::new(bin_path);

    if let Some(scripts_path) = scripts {
        let abs_path = fs::canonicalize(&scripts_path)
            .context(format!("Failed to resolve scripts path: {}", scripts_path))?;

        println!("{} Scripts path: {:?}", style("->").dim(), abs_path);
        command.env("BLENDER_USER_SCRIPTS", abs_path);
    }

    let status = command
        .args(&args)
        .status()
        .context("Failed to start Blender")?;

    if !status.success() {
        bail!("Blender exited with non-zero status code");
    }

    Ok(())
}

fn prepare_run_args(
    target_version: Option<String>,
    args: Vec<String>,
) -> (Option<String>, Vec<String>) {
    if let Some(ref v) = target_version {
        let is_blend_file = v.to_lowercase().ends_with(".blend");
        let exists_as_file = Path::new(v).is_file();
        let is_flag = v.starts_with('-');

        if is_blend_file || exists_as_file || is_flag {
            let mut new_args = args;
            new_args.insert(0, v.clone());
            return (None, new_args);
        }
    }
    (target_version, args)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::File;
    use tempfile::tempdir;

    #[test]
    fn test_prepare_run_args_with_blend_extension() {
        let (ver, args) = prepare_run_args(
            Some("mycheck.blend".to_string()),
            vec!["--background".to_string()],
        );
        assert_eq!(ver, None);
        assert_eq!(args, vec!["mycheck.blend", "--background"]);
    }

    #[test]
    fn test_prepare_run_args_with_existing_file() {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("script.py");
        File::create(&file_path).unwrap();
        let path_str = file_path.to_str().unwrap().to_string();

        let (ver, args) = prepare_run_args(Some(path_str.clone()), vec![]);
        assert_eq!(ver, None);
        assert_eq!(args, vec![path_str]);
    }

    #[test]
    fn test_prepare_run_args_with_version() {
        let (ver, args) = prepare_run_args(Some("4.0.0".to_string()), vec![]);
        assert_eq!(ver, Some("4.0.0".to_string()));
        assert!(args.is_empty());
    }

    #[test]
    fn test_prepare_run_args_with_flag() {
        let (ver, args) = prepare_run_args(
            Some("--background".to_string()),
            vec!["--version".to_string()],
        );
        assert_eq!(ver, None);
        assert_eq!(args, vec!["--background", "--version"]);
    }
}
