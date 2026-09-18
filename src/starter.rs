use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};

use crate::catalog;
use crate::manifest::MANIFEST_FILE;

const STARTER_MANIFEST: &str = r#"# Works After Reinstall - starter configuration
#
# Everything here is optional: delete what you do not want and it simply will
# not be applied. When you are ready:
#
#   works-after-reinstall setup "<this folder>"
#
# `works-after-reinstall starter --list` prints every package id this tool
# knows about, if you would rather pick from a list.

version: 1

# WinGet packages to install.
apps:
  - Git.Git
  - Microsoft.VisualStudioCode
  - Microsoft.WindowsTerminal
  - 7zip.7zip
  # Uncomment any of these, or add your own:
  # - Google.Chrome
  # - Mozilla.Firefox
  # - Microsoft.PowerToys
  # - Notepad++.Notepad++
  # - OpenJS.NodeJS.LTS
  # - Python.Python.3.13
  # - Valve.Steam
  # - VideoLAN.VLC
  #
  # Packages that need a source or installer arguments look like this:
  # - id: XP89DCGQ3K6VLD
  #   source: msstore

# Windows preferences. Delete the ones you would rather leave alone.
windows:
  dark_mode: true
  show_file_extensions: true
  developer_mode: true

# Environment variables for your account. %VARIABLES% stay literal until
# something reads them.
environment:
  variables:
    EDITOR: code
    DOTNET_CLI_TELEMETRY_OPTOUT: "1"

# Files to copy into place. The paths on the left live inside this folder, and
# the destinations accept %VARIABLES% and a leading ~/.
# Each of these already exists in files/ - put your own details in them, then
# uncomment the lines you want:
files:
  copy: []
  # - source: files/gitconfig
  #   destination: '%USERPROFILE%\.gitconfig'
  # - source: files/vscode_settings.json
  #   destination: '%APPDATA%\Code\User\settings.json'
  # - source: files/powershell_profile.ps1
  #   destination: '%USERPROFILE%\Documents\PowerShell\Microsoft.PowerShell_profile.ps1'
"#;

const STARTER_GITCONFIG: &str = r#"# Replace the placeholders below, then uncomment the matching entry in
# works-after-reinstall.yaml.
[user]
	name = Your Name
	email = you@example.com

[init]
	defaultBranch = main

[core]
	autocrlf = true

[credential]
	helper = manager

[push]
	autoSetupRemote = true

[pull]
	rebase = false

[alias]
	st = status -sb
	co = checkout
	br = branch
	lg = log --oneline --graph --decorate
	last = log -1 HEAD
"#;

const STARTER_VSCODE: &str = r#"{
  "editor.formatOnSave": true,
  "editor.tabSize": 2,
  "editor.renderWhitespace": "boundary",
  "files.eol": "\n",
  "files.trimTrailingWhitespace": true,
  "files.insertFinalNewline": true,
  "telemetry.telemetryLevel": "off",
  "git.autofetch": true,
  "terminal.integrated.defaultProfile.windows": "PowerShell"
}
"#;

const STARTER_PROFILE: &str = r#"# A small starting point - add your own aliases and helpers.
Set-PSReadLineOption -PredictionSource History

function .. { Set-Location .. }
function ll { Get-ChildItem -Force }
function gs { git status -sb }
"#;

pub fn scaffold(dir: &Path, force: bool) -> Result<PathBuf> {
    let manifest_path = dir.join(MANIFEST_FILE);
    if manifest_path.exists() && !force {
        bail!(
            "{} already exists - pass --force to overwrite it, or pick another folder",
            manifest_path.display()
        );
    }

    let files = dir.join("files");
    fs::create_dir_all(&files).with_context(|| format!("creating {}", files.display()))?;

    fs::write(&manifest_path, STARTER_MANIFEST)
        .with_context(|| format!("writing {}", manifest_path.display()))?;
    for (name, contents) in [
        ("gitconfig", STARTER_GITCONFIG),
        ("vscode_settings.json", STARTER_VSCODE),
        ("powershell_profile.ps1", STARTER_PROFILE),
    ] {
        let path = files.join(name);
        fs::write(&path, contents).with_context(|| format!("writing {}", path.display()))?;
    }

    Ok(manifest_path)
}

pub fn listing() -> String {
    let mut out = String::new();
    for category in catalog::CATEGORIES {
        out.push_str(category);
        out.push('\n');
        for app in catalog::APPS.iter().filter(|app| app.category == category) {
            out.push_str(&format!("  {:<30} {}\n", app.name, app.id));
        }
        out.push('\n');
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::manifest::Manifest;

    fn temp_dir(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("war-starter-{}-{name}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        dir
    }

    #[test]
    fn the_starter_manifest_parses_under_our_own_rules() {
        let path = std::env::temp_dir().join(format!("war-starter-{}.yaml", std::process::id()));
        fs::write(&path, STARTER_MANIFEST).unwrap();

        let manifest = Manifest::load(&path).unwrap();

        assert_eq!(manifest.apps.len(), 4);
        assert_eq!(manifest.windows.dark_mode, Some(true));
        assert_eq!(manifest.windows.developer_mode, Some(true));
        assert_eq!(
            manifest.environment.variables.get("EDITOR"),
            Some(&"code".to_string())
        );
        assert!(manifest.files.copy.is_empty());

        fs::remove_file(&path).unwrap();
    }

    #[test]
    fn scaffolding_writes_the_manifest_and_the_example_files() {
        let dir = temp_dir("writes");

        let manifest_path = scaffold(&dir, false).unwrap();

        assert!(manifest_path.is_file());
        assert!(dir.join("files/gitconfig").is_file());
        assert!(dir.join("files/vscode_settings.json").is_file());
        assert!(dir.join("files/powershell_profile.ps1").is_file());
        Manifest::load(&manifest_path).unwrap();

        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn scaffolding_refuses_to_clobber_without_force() {
        let dir = temp_dir("force");
        scaffold(&dir, false).unwrap();

        assert!(scaffold(&dir, false).is_err());
        assert!(scaffold(&dir, true).is_ok());

        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn the_example_files_are_not_empty() {
        for contents in [STARTER_GITCONFIG, STARTER_VSCODE, STARTER_PROFILE] {
            assert!(contents.len() > 40);
        }
        assert!(STARTER_VSCODE.trim_start().starts_with('{'));
    }

    #[test]
    fn the_listing_covers_every_catalog_app() {
        let listing = listing();

        for category in catalog::CATEGORIES {
            assert!(listing.contains(category));
        }
        for app in catalog::APPS {
            assert!(listing.contains(app.id), "{} is missing", app.id);
        }
    }
}
