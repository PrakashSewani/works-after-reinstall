use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use anyhow::{Result, bail};

use crate::commands;
use crate::console::Console;
use crate::manifest::MANIFEST_FILE;
use crate::paths;
use crate::prompt;

pub const DEFAULT_BUNDLE_DIR: &str = "works-after-reinstall-bundle";

pub enum BundleChoice {
    Ready(PathBuf),
    NothingToApply,
}

/// Decide which bundle `setup` should apply. A missing bundle is not an error:
/// we look around for one, and on an interactive terminal offer to capture this
/// machine instead of exiting.
pub fn resolve_requested_bundle(requested: Option<PathBuf>) -> Result<BundleChoice> {
    if let Some(path) = requested {
        if path.exists() {
            return Ok(BundleChoice::Ready(path));
        }
        let nearby = find_bundles();
        if nearby.is_empty() {
            bail!(
                "bundle not found: {}\n\x20 run `works-after-reinstall export` to create one, or pass the path to an existing bundle",
                path.display()
            );
        }
        bail!(
            "bundle not found: {}\n\x20 bundles found nearby:\n{}",
            path.display(),
            bullet_list(&nearby)
        );
    }

    let default = PathBuf::from(DEFAULT_BUNDLE_DIR);
    if default.is_dir() {
        return Ok(BundleChoice::Ready(default));
    }

    let found = find_bundles();
    match found.len() {
        0 => offer_export(),
        1 => {
            println!("Using the bundle found at {}\n", found[0].display());
            Ok(BundleChoice::Ready(found[0].clone()))
        }
        _ => {
            if !prompt::is_interactive() {
                bail!(
                    "no bundle given and several were found:\n{}",
                    bullet_list(&found)
                );
            }
            let labels: Vec<String> = found
                .iter()
                .map(|path| path.display().to_string())
                .collect();
            match prompt::choose("Which bundle should I apply?", &labels) {
                Some(index) => Ok(BundleChoice::Ready(found[index].clone())),
                None => Ok(BundleChoice::NothingToApply),
            }
        }
    }
}

fn offer_export() -> Result<BundleChoice> {
    let searched = describe_search_locations();
    if !prompt::is_interactive() {
        bail!(
            "no bundle found - looked in {searched}\n\x20 run `works-after-reinstall export` here to capture this machine first, or pass the bundle path"
        );
    }

    println!("No bundle found here.");
    println!();
    println!("A bundle is a folder containing {MANIFEST_FILE}.");
    println!("I looked in {searched}.");
    println!();
    println!("If you are on the machine you want to capture, I can create one now.");

    if !prompt::confirm(
        "Export this machine into ./works-after-reinstall-bundle?",
        true,
    )? {
        println!();
        println!(
            "Nothing to do then. Run `works-after-reinstall export` when you are ready to capture this machine."
        );
        return Ok(BundleChoice::NothingToApply);
    }

    println!();
    let output = PathBuf::from(DEFAULT_BUNDLE_DIR);
    commands::export(&output, &[], &[], &Console::stdout())?;
    println!();
    println!("That bundle is ready to be taken to the new machine:");
    println!(
        "  1. copy \"{}\" somewhere you will still have it after reinstalling",
        output.display()
    );
    println!("  2. after the reinstall, run the exe next to it and pick \"Setup from a bundle\"");
    Ok(BundleChoice::NothingToApply)
}

pub fn find_bundles() -> Vec<PathBuf> {
    let mut roots: Vec<PathBuf> = Vec::new();

    if let Ok(exe) = std::env::current_exe()
        && let Some(dir) = exe.parent()
    {
        roots.push(dir.to_path_buf());
    }
    if let Ok(cwd) = std::env::current_dir() {
        roots.push(cwd);
    }
    if let Some(home) = paths::home_dir() {
        roots.push(home.join("Desktop"));
        roots.push(home.join("Downloads"));
    }

    find_bundles_in(&roots)
}

pub fn find_bundles_in(roots: &[PathBuf]) -> Vec<PathBuf> {
    let mut found = Vec::new();
    let mut seen = BTreeSet::new();

    for root in roots {
        consider(root, &mut found, &mut seen);
        let Ok(entries) = std::fs::read_dir(root) else {
            continue;
        };
        for entry in entries.flatten() {
            if entry.file_type().map(|kind| kind.is_dir()).unwrap_or(false) {
                consider(&entry.path(), &mut found, &mut seen);
            }
        }
    }

    found
}

fn consider(dir: &Path, found: &mut Vec<PathBuf>, seen: &mut BTreeSet<String>) {
    if !dir.join(MANIFEST_FILE).is_file() {
        return;
    }
    let key = dir
        .to_string_lossy()
        .trim_end_matches(['\\', '/'])
        .to_ascii_lowercase();
    if seen.insert(key) {
        found.push(dir.to_path_buf());
    }
}

fn describe_search_locations() -> String {
    let mut places = vec!["this folder".to_string()];
    if std::env::current_exe()
        .ok()
        .and_then(|exe| exe.parent().map(Path::to_path_buf))
        .is_some()
    {
        places.push("the folder holding the exe".to_string());
    }
    if paths::home_dir().is_some() {
        places.push("Desktop and Downloads".to_string());
    }
    places.join(", ")
}

fn bullet_list(paths: &[PathBuf]) -> String {
    paths
        .iter()
        .map(|path| format!("  - {}", path.display()))
        .collect::<Vec<_>>()
        .join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::manifest::{CURRENT_VERSION, Manifest};

    fn temp_root(name: &str) -> PathBuf {
        let root =
            std::env::temp_dir().join(format!("war-discovery-{}-{name}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).unwrap();
        root
    }

    fn write_bundle(dir: &Path) {
        std::fs::create_dir_all(dir).unwrap();
        let manifest = Manifest {
            version: CURRENT_VERSION,
            ..Default::default()
        };
        manifest.save(&dir.join(MANIFEST_FILE)).unwrap();
    }

    #[test]
    fn finds_bundles_in_a_folder_and_its_children() {
        let root = temp_root("children");
        write_bundle(&root);
        write_bundle(&root.join("bundle-a"));
        std::fs::create_dir_all(root.join("not-a-bundle")).unwrap();

        let found = find_bundles_in(std::slice::from_ref(&root));

        assert_eq!(found.len(), 2);
        assert!(found.contains(&root));
        assert!(found.contains(&root.join("bundle-a")));

        std::fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn reports_nothing_when_there_are_no_bundles() {
        let root = temp_root("empty");
        std::fs::create_dir_all(root.join("some-folder")).unwrap();

        assert!(find_bundles_in(std::slice::from_ref(&root)).is_empty());

        std::fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn does_not_report_the_same_bundle_twice() {
        let root = temp_root("dedup");
        write_bundle(&root.join("bundle"));

        let found = find_bundles_in(&[root.clone(), root.join("bundle")]);

        assert_eq!(found.len(), 1);

        std::fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn ignores_missing_roots() {
        assert!(find_bundles_in(&[PathBuf::from(r"D:\does\not\exist")]).is_empty());
    }
}
