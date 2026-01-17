use crate::core::{config, daily, os};
use anyhow::Result;
use console::style;
use reqwest::Client;
use std::collections::HashSet;
use std::fs;

pub async fn run(remote: bool) -> Result<()> {
    let installed_versions = get_installed_versions()?;

    if !remote {
        print_installed_list(&installed_versions);
        return Ok(());
    }

    list_remote_builds(&installed_versions).await?;
    Ok(())
}

fn get_installed_versions() -> Result<HashSet<String>> {
    let app_root = config::get_app_root()?;
    let versions_dir = app_root.join("versions");

    let mut installed_versions = HashSet::new();
    if versions_dir.is_dir() {
        for entry in fs::read_dir(&versions_dir)? {
            let entry = entry?;
            if entry.file_type()?.is_dir() {
                if let Ok(name) = entry.file_name().into_string() {
                    installed_versions.insert(name);
                }
            }
        }
    }
    Ok(installed_versions)
}

fn print_installed_list(installed_versions: &HashSet<String>) {
    println!("{}", style("Installed Blender Versions:").bold());
    if installed_versions.is_empty() {
        println!("  (No versions installed)");
    } else {
        let mut v_list: Vec<_> = installed_versions.iter().collect();
        v_list.sort();
        for v in v_list {
            println!("  {}", v);
        }
    }
}

async fn list_remote_builds(installed_versions: &HashSet<String>) -> Result<()> {
    println!("{}", style("Fetching remote versions...").dim());

    let client = Client::new();
    let builds = daily::fetch_daily_list(&client).await?;
    let platform = os::detect_platform()?;

    let sections = daily::categorize_builds(builds, &platform);

    println!("\n{}", style("Daily Builds (builder.blender.org):").bold());
    if sections.daily.is_empty() {
        println!("  (None found for this platform)");
    }
    for build in sections.daily {
        let full_name = format!("{}-{}-{}", build.version, build.risk_id, build.hash);
        let note = match build.risk_id.as_str() {
            "alpha" => "Alpha",
            "beta" => "Beta",
            "candidate" => "Candidate",
            _ => "Experimental",
        };

        let is_lts = daily::is_lts(&build.version);

        let display_ver = format!("{}-{}", build.version, build.risk_id);
        let hash_short = build.hash.get(0..7).unwrap_or(&build.hash);
        let extra_info = format!("{}, {}", note, hash_short);

        print_remote_entry(
            &full_name,
            &display_ver,
            installed_versions,
            &extra_info,
            is_lts,
        );
    }

    println!("\n{}", style("Stable Releases (Active Support):").bold());
    if sections.stable.is_empty() {
        println!("  (None found)");
    }
    for build in sections.stable {
        let is_lts = daily::is_lts(&build.version);
        print_remote_entry(
            &build.version,
            &build.version,
            installed_versions,
            "",
            is_lts,
        );
    }

    println!(); // Footer margin
    Ok(())
}

fn print_remote_entry(
    install_key: &str,
    display_text: &str,
    installed: &HashSet<String>,
    note: &str,
    is_lts: bool,
) {
    let is_installed = installed.contains(install_key);

    let marker = if is_installed {
        style("*").green().bold()
    } else {
        style(" ")
    };

    let version_text = if is_installed {
        style(display_text).green()
    } else {
        style(display_text)
    };

    let mut tags = Vec::new();
    if is_lts {
        tags.push(style("LTS").yellow().bold().to_string());
    }
    if is_installed {
        tags.push(style("Installed").dim().to_string());
    }
    if !note.is_empty() {
        tags.push(style(note).dim().to_string());
    }

    let tags_str = if tags.is_empty() {
        String::new()
    } else {
        format!(" ({})", tags.join(", "))
    };

    println!("{} {}{}", marker, version_text, tags_str);
}
