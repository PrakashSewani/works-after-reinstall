use std::path::Path;

use anyhow::{Context, Result, bail};

use crate::cli::{Component, selected};
use crate::console::Console;
use crate::manifest::{CURRENT_VERSION, MANIFEST_FILE, Manifest};
use crate::paths;
use crate::{envvars, files, settings, util, winget};

pub fn export(
    output: &Path,
    include: &[String],
    skip: &[Component],
    console: &Console,
) -> Result<()> {
    if output.exists() && !output.is_dir() {
        bail!("{} exists and is not a directory", output.display());
    }
    std::fs::create_dir_all(output).with_context(|| format!("creating {}", output.display()))?;

    let machine = std::env::var("COMPUTERNAME").unwrap_or_default();
    if machine.is_empty() {
        console.line(format!("Exporting this machine into {}", output.display()));
    } else {
        console.line(format!("Exporting {machine} into {}", output.display()));
    }
    console.blank();

    let mut manifest = Manifest {
        version: CURRENT_VERSION,
        exported_at: Some(util::now_iso8601()),
        source_machine: (!machine.is_empty()).then_some(machine),
        ..Default::default()
    };

    if selected(&[], skip, Component::Apps) {
        console.section(Component::Apps);
        manifest.apps = winget::export().context("capturing installed applications")?;
        console.line(format!("  {} packages", manifest.apps.len()));
    }

    if selected(&[], skip, Component::Windows) {
        console.section(Component::Windows);
        manifest.windows = settings::capture()?;
        console.line(format!("  {}", settings::summary(&manifest.windows)));
    }

    if selected(&[], skip, Component::Environment) {
        console.section(Component::Environment);
        let (variables, path) = envvars::capture()?;
        manifest.environment.variables = variables;
        manifest.environment.path = path;
        if manifest.environment.variables.is_empty() {
            console.line("  no variables outside the Windows defaults");
        } else {
            let names = manifest
                .environment
                .variables
                .keys()
                .cloned()
                .collect::<Vec<_>>()
                .join(", ");
            console.line(format!(
                "  {} variables: {names}",
                manifest.environment.variables.len()
            ));
        }
        console.line(format!(
            "  {} PATH entries",
            manifest.environment.path.len()
        ));
    }

    if selected(&[], skip, Component::Files) {
        console.section(Component::Files);
        manifest.files.copy = files::capture(output, include)?;
        if manifest.files.copy.is_empty() {
            console.line("  no known config files found - add your own with --include");
        } else {
            for copy in &manifest.files.copy {
                console.line(format!("  {}", copy.source));
            }
        }
    }

    let manifest_path = output.join(MANIFEST_FILE);
    manifest.save(&manifest_path)?;

    console.blank();
    console.line(format!("Bundle ready: {}", manifest_path.display()));
    console.line(
        "Keep it where you can reach it after reinstalling Windows (USB drive, cloud storage, another machine).",
    );
    console.blank();
    console.line("After reinstalling, run:");
    console.line(format!(
        "  works-after-reinstall setup \"{}\"",
        output.display()
    ));
    Ok(())
}

pub fn setup(bundle: &Path, dry_run: bool, only: &[Component], console: &Console) -> Result<()> {
    let (manifest_path, bundle_root) = paths::resolve_bundle(bundle)?;
    let manifest = Manifest::load(&manifest_path)?;

    console.line(format!("Applying {}", manifest_path.display()));
    apply_manifest(&manifest, &bundle_root, dry_run, only, console)
}

/// Apply a manifest that did not necessarily come from a bundle on disk - the
/// interactive starter flow builds one in memory.
pub fn apply_manifest(
    manifest: &Manifest,
    bundle_root: &Path,
    dry_run: bool,
    only: &[Component],
    console: &Console,
) -> Result<()> {
    console.blank();
    if dry_run {
        console.line("Dry run - nothing will be changed.");
        console.blank();
    }

    let mut failures = Vec::new();

    if selected(only, &[], Component::Windows) {
        console.section(Component::Windows);
        if let Err(error) = settings::apply(&manifest.windows, dry_run, console) {
            failures.push(format!("windows: {error:#}"));
        }
    }

    if selected(only, &[], Component::Environment) {
        console.section(Component::Environment);
        if let Err(error) = envvars::apply(
            &manifest.environment.variables,
            &manifest.environment.path,
            dry_run,
            console,
        ) {
            failures.push(format!("environment: {error:#}"));
        }
    }

    if selected(only, &[], Component::Files) {
        console.section(Component::Files);
        if let Err(error) = files::apply(bundle_root, &manifest.files.copy, dry_run, console) {
            failures.push(format!("files: {error:#}"));
        }
    }

    if selected(only, &[], Component::Apps) {
        console.section(Component::Apps);
        if let Err(error) = winget::apply(&manifest.apps, dry_run, console) {
            failures.push(format!("apps: {error:#}"));
        }
    }

    if !failures.is_empty() {
        bail!(
            "setup finished with {} problem(s):\n  - {}",
            failures.len(),
            failures.join("\n  - ")
        );
    }

    console.blank();
    if dry_run {
        console.line("Dry run complete - nothing was changed.");
    } else {
        console.line("Done.");
        if selected(only, &[], Component::Environment) {
            console.line("Open a new terminal so the restored environment is picked up.");
        }
        if selected(only, &[], Component::Apps) || selected(only, &[], Component::Windows) {
            console.line("Some changes may need a restart before they take effect.");
        }
    }
    Ok(())
}
