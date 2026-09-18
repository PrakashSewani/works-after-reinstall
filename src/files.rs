use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};

use crate::console::Console;
use crate::manifest::FileCopy;
use crate::paths::{expand_vars, home_dir};

const BUNDLE_FILES_DIR: &str = "files";

struct Known {
    bundle_name: &'static str,
    source: &'static str,
    destination: &'static str,
}

const KNOWN: &[Known] = &[
    Known {
        bundle_name: "gitconfig",
        source: "~/.gitconfig",
        destination: r"%USERPROFILE%\.gitconfig",
    },
    Known {
        bundle_name: "gitignore_global",
        source: "~/.gitignore_global",
        destination: r"%USERPROFILE%\.gitignore_global",
    },
    Known {
        bundle_name: "ssh_config",
        source: "~/.ssh/config",
        destination: r"%USERPROFILE%\.ssh\config",
    },
    Known {
        bundle_name: "wslconfig",
        source: "~/.wslconfig",
        destination: r"%USERPROFILE%\.wslconfig",
    },
    Known {
        bundle_name: "npmrc",
        source: "~/.npmrc",
        destination: r"%USERPROFILE%\.npmrc",
    },
    Known {
        bundle_name: "vimrc",
        source: "~/.vimrc",
        destination: r"%USERPROFILE%\.vimrc",
    },
    Known {
        bundle_name: "editorconfig",
        source: "~/.editorconfig",
        destination: r"%USERPROFILE%\.editorconfig",
    },
    Known {
        bundle_name: "powershell_profile.ps1",
        source: "~/Documents/PowerShell/Microsoft.PowerShell_profile.ps1",
        destination: r"%USERPROFILE%\Documents\PowerShell\Microsoft.PowerShell_profile.ps1",
    },
    Known {
        bundle_name: "windows_powershell_profile.ps1",
        source: "~/Documents/WindowsPowerShell/Microsoft.PowerShell_profile.ps1",
        destination: r"%USERPROFILE%\Documents\WindowsPowerShell\Microsoft.PowerShell_profile.ps1",
    },
    Known {
        bundle_name: "vscode_settings.json",
        source: "%APPDATA%/Code/User/settings.json",
        destination: r"%APPDATA%\Code\User\settings.json",
    },
    Known {
        bundle_name: "vscode_keybindings.json",
        source: "%APPDATA%/Code/User/keybindings.json",
        destination: r"%APPDATA%\Code\User\keybindings.json",
    },
];

pub fn capture(bundle: &Path, includes: &[String]) -> Result<Vec<FileCopy>> {
    let files_dir = bundle.join(BUNDLE_FILES_DIR);
    let mut copies = Vec::new();
    let mut names = BTreeSet::new();
    let mut destinations = BTreeSet::new();

    for known in KNOWN {
        let source = expand_vars(known.source);
        if !source.is_file() {
            continue;
        }
        let name = unique_name(known.bundle_name, &mut names);
        copy_into_bundle(&source, &files_dir.join(&name))?;
        copies.push(FileCopy {
            source: format!("{BUNDLE_FILES_DIR}/{name}"),
            destination: known.destination.to_string(),
        });
        destinations.insert(known.destination.to_ascii_lowercase());
    }

    for include in includes {
        let source = expand_vars(include);
        if !source.is_file() {
            bail!("cannot include {}: not a file", source.display());
        }
        let destination = destination_for(&source);
        if !destinations.insert(destination.to_ascii_lowercase()) {
            continue;
        }
        let base = source
            .file_name()
            .map(|name| name.to_string_lossy().to_string())
            .unwrap_or_else(|| "file".to_string());
        let name = unique_name(&base, &mut names);
        copy_into_bundle(&source, &files_dir.join(&name))?;
        copies.push(FileCopy {
            source: format!("{BUNDLE_FILES_DIR}/{name}"),
            destination,
        });
    }

    Ok(copies)
}

pub fn apply(bundle: &Path, copies: &[FileCopy], dry_run: bool, console: &Console) -> Result<()> {
    if copies.is_empty() {
        console.line("  nothing to restore");
        return Ok(());
    }

    for copy in copies {
        let source = resolve_source(bundle, &copy.source);
        if !source.is_file() {
            bail!("bundle is missing {}", source.display());
        }
        let destination = expand_vars(&copy.destination);
        if destination.is_file() && same_contents(&source, &destination) {
            console.line(format!("  {} unchanged", destination.display()));
            continue;
        }

        console.line(format!("  {} -> {}", copy.source, destination.display()));
        if dry_run {
            continue;
        }
        if let Some(parent) = destination.parent() {
            fs::create_dir_all(parent).with_context(|| format!("creating {}", parent.display()))?;
        }
        fs::copy(&source, &destination).with_context(|| {
            format!("copying {} to {}", source.display(), destination.display())
        })?;
    }

    Ok(())
}

pub fn destination_for(path: &Path) -> String {
    destination_for_home(path, home_dir().as_deref())
}

fn destination_for_home(path: &Path, home: Option<&Path>) -> String {
    if let Some(home) = home
        && let Ok(relative) = path.strip_prefix(home)
    {
        return format!(r"%USERPROFILE%\{}", relative.display());
    }
    path.display().to_string()
}

fn resolve_source(bundle: &Path, source: &str) -> PathBuf {
    let path = expand_vars(source);
    if path.is_absolute() {
        path
    } else {
        bundle.join(path)
    }
}

fn copy_into_bundle(source: &Path, destination: &Path) -> Result<()> {
    if let Some(parent) = destination.parent() {
        fs::create_dir_all(parent).with_context(|| format!("creating {}", parent.display()))?;
    }
    fs::copy(source, destination)
        .with_context(|| format!("copying {} to {}", source.display(), destination.display()))?;
    Ok(())
}

fn unique_name(base: &str, used: &mut BTreeSet<String>) -> String {
    if used.insert(base.to_string()) {
        return base.to_string();
    }
    let mut counter = 2;
    loop {
        let candidate = format!("{base}.{counter}");
        if used.insert(candidate.clone()) {
            return candidate;
        }
        counter += 1;
    }
}

fn same_contents(left: &Path, right: &Path) -> bool {
    match (fs::read(left), fs::read(right)) {
        (Ok(left), Ok(right)) => left == right,
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn turns_home_paths_into_portable_destinations() {
        let home = Path::new(r"C:\Users\tester");
        assert_eq!(
            destination_for_home(Path::new(r"C:\Users\tester\.ssh\config"), Some(home)),
            r"%USERPROFILE%\.ssh\config"
        );
    }

    #[test]
    fn keeps_paths_outside_home_absolute() {
        let home = Path::new(r"C:\Users\tester");
        assert_eq!(
            destination_for_home(Path::new(r"D:\shared\tool.config"), Some(home)),
            r"D:\shared\tool.config"
        );
    }

    #[test]
    fn falls_back_when_home_is_unknown() {
        assert_eq!(
            destination_for_home(Path::new(r"C:\Users\tester\.gitconfig"), None),
            r"C:\Users\tester\.gitconfig"
        );
    }

    #[test]
    fn avoids_bundle_name_collisions() {
        let mut used = BTreeSet::new();
        assert_eq!(unique_name("config", &mut used), "config");
        assert_eq!(unique_name("config", &mut used), "config.2");
        assert_eq!(unique_name("config", &mut used), "config.3");
    }

    #[test]
    fn known_files_have_unique_bundle_names_and_destinations() {
        let mut names = BTreeSet::new();
        let mut destinations = BTreeSet::new();
        for known in KNOWN {
            assert!(
                names.insert(known.bundle_name),
                "duplicate {}",
                known.bundle_name
            );
            assert!(
                destinations.insert(known.destination.to_ascii_lowercase()),
                "duplicate {}",
                known.destination
            );
        }
    }

    #[test]
    fn resolves_bundle_relative_sources() {
        assert_eq!(
            resolve_source(Path::new(r"D:\bundle"), "files/gitconfig"),
            PathBuf::from(r"D:\bundle\files\gitconfig")
        );
    }
}
