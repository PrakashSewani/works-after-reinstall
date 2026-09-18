use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};

use anyhow::{Context, Result, bail};

use crate::console::Console;
use crate::manifest::{App, AppDetail};
use crate::util::now_iso8601;

const PACKAGES_SCHEMA: &str = "https://aka.ms/winget-packages.schema.2.0.json";

struct Source {
    name: &'static str,
    argument: &'static str,
    identifier: &'static str,
    kind: &'static str,
}

const DEFAULT_SOURCE: &Source = &Source {
    name: "winget",
    argument: "https://cdn.winget.microsoft.com/cache",
    identifier: "Microsoft.Winget.Source_8wekyb3d8bbwe",
    kind: "Microsoft.PreIndexed.Package",
};

const STORE_SOURCE: &Source = &Source {
    name: "msstore",
    argument: "https://storeedgefd.dsx.mp.microsoft.com/v9.0",
    identifier: "StoreEdgeFD",
    kind: "Microsoft.Rest",
};

pub fn export() -> Result<Vec<App>> {
    ensure_available()?;
    let file = temp_file("winget-export", "json");
    let output = Command::new("winget")
        .args(["export", "--output"])
        .arg(&file)
        .args(["--accept-source-agreements", "--disable-interactivity"])
        .output()
        .context("running winget export")?;

    let json = std::fs::read_to_string(&file).unwrap_or_default();
    let _ = std::fs::remove_file(&file);

    if json.trim().is_empty() {
        bail!(
            "winget export produced no package list{}",
            stderr_hint(&output)
        );
    }
    parse_apps(&json)
}

pub fn apply(apps: &[App], dry_run: bool, console: &Console) -> Result<()> {
    if apps.is_empty() {
        console.line("  nothing to install");
        return Ok(());
    }
    ensure_available()?;

    if dry_run {
        for app in apps {
            let arguments = app
                .arguments()
                .map(|arguments| format!(" with {arguments}"))
                .unwrap_or_default();
            console.line(format!(
                "  install {}{arguments} (source: {})",
                app.id(),
                source_for(app.source()).name
            ));
        }
        return Ok(());
    }

    let file = temp_file("winget-import", "json");
    std::fs::write(&file, import_json(apps)).context("writing temporary winget import file")?;
    console.line(format!("  installing {} packages with winget", apps.len()));

    let outcome = run_import(&file, console);
    let _ = std::fs::remove_file(&file);

    if !outcome? {
        bail!("winget import failed - review the output above for packages it could not install");
    }
    Ok(())
}

fn run_import(file: &Path, console: &Console) -> Result<bool> {
    let mut command = Command::new("winget");
    command.args(["import", "--import-file"]).arg(file).args([
        "--accept-package-agreements",
        "--accept-source-agreements",
        "--ignore-unavailable",
        "--no-upgrade",
        "--disable-interactivity",
    ]);

    match console {
        Console::Stdout => Ok(command.status().context("running winget import")?.success()),
        Console::Channel(_) => {
            let mut child = command
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .spawn()
                .context("running winget import")?;

            let stdout = child.stdout.take().map(|pipe| {
                let console = console.clone();
                std::thread::spawn(move || pump(pipe, console))
            });
            let stderr = child.stderr.take().map(|pipe| {
                let console = console.clone();
                std::thread::spawn(move || pump(pipe, console))
            });

            let status = child.wait().context("waiting for winget import")?;
            if let Some(handle) = stdout {
                let _ = handle.join();
            }
            if let Some(handle) = stderr {
                let _ = handle.join();
            }
            Ok(status.success())
        }
    }
}

fn pump<R: std::io::Read>(reader: R, console: Console) {
    use std::io::{BufReader, Read};

    let mut reader = BufReader::new(reader);
    let mut buffer = Vec::new();
    let mut byte = [0u8; 1];

    loop {
        match reader.read(&mut byte) {
            Ok(0) | Err(_) => break,
            Ok(_) => {}
        }
        if byte[0] == b'\r' || byte[0] == b'\n' {
            flush_line(&mut buffer, &console);
        } else {
            buffer.push(byte[0]);
        }
    }
    flush_line(&mut buffer, &console);
}

fn flush_line(buffer: &mut Vec<u8>, console: &Console) {
    if buffer.is_empty() {
        return;
    }
    let text = String::from_utf8_lossy(buffer);
    let trimmed = text.trim();
    if !trimmed.is_empty() {
        console.line(format!("  {trimmed}"));
    }
    buffer.clear();
}

fn ensure_available() -> Result<()> {
    let available = Command::new("winget")
        .arg("--version")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|status| status.success())
        .unwrap_or(false);

    if !available {
        bail!(
            "winget is not available - install 'App Installer' from the Microsoft Store, then run the command again"
        );
    }
    Ok(())
}

fn source_for(name: Option<&str>) -> &'static Source {
    match name {
        Some(name) if name.eq_ignore_ascii_case(STORE_SOURCE.name) => STORE_SOURCE,
        _ => DEFAULT_SOURCE,
    }
}

fn parse_apps(json: &str) -> Result<Vec<App>> {
    let value: serde_json::Value =
        serde_json::from_str(json).context("parsing winget export output")?;

    let mut apps = Vec::new();
    for source in value
        .get("Sources")
        .and_then(|sources| sources.as_array())
        .into_iter()
        .flatten()
    {
        let source_name = source
            .get("SourceDetails")
            .and_then(|details| details.get("Name"))
            .and_then(|name| name.as_str())
            .unwrap_or(DEFAULT_SOURCE.name);

        for package in source
            .get("Packages")
            .and_then(|packages| packages.as_array())
            .into_iter()
            .flatten()
        {
            let Some(id) = package.get("PackageIdentifier").and_then(|id| id.as_str()) else {
                continue;
            };

            let arguments = package
                .get("InitialOverrideArguments")
                .and_then(|arguments| arguments.as_str())
                .map(str::to_string);
            let source = (!source_name.eq_ignore_ascii_case(DEFAULT_SOURCE.name))
                .then(|| source_name.to_string());

            if source.is_none() && arguments.is_none() {
                apps.push(App::Id(id.to_string()));
            } else {
                apps.push(App::Detailed(AppDetail {
                    id: id.to_string(),
                    source,
                    arguments,
                }));
            }
        }
    }

    apps.sort_by(|left, right| left.id().cmp(right.id()));
    apps.dedup_by(|left, right| left.id() == right.id());
    Ok(apps)
}

fn import_json(apps: &[App]) -> String {
    let mut sources = Vec::new();

    for source in [DEFAULT_SOURCE, STORE_SOURCE] {
        let packages: Vec<serde_json::Value> = apps
            .iter()
            .filter(|app| source_for(app.source()).name == source.name)
            .map(|app| {
                let mut package = serde_json::Map::new();
                package.insert(
                    "PackageIdentifier".to_string(),
                    serde_json::Value::String(app.id().to_string()),
                );
                if let Some(arguments) = app.arguments() {
                    package.insert(
                        "InitialOverrideArguments".to_string(),
                        serde_json::Value::String(arguments.to_string()),
                    );
                }
                serde_json::Value::Object(package)
            })
            .collect();

        if packages.is_empty() {
            continue;
        }

        sources.push(serde_json::json!({
            "Packages": packages,
            "SourceDetails": {
                "Argument": source.argument,
                "Identifier": source.identifier,
                "Name": source.name,
                "Type": source.kind,
            }
        }));
    }

    serde_json::json!({
        "$schema": PACKAGES_SCHEMA,
        "CreationDate": now_iso8601(),
        "Sources": sources,
    })
    .to_string()
}

fn temp_file(prefix: &str, extension: &str) -> PathBuf {
    std::env::temp_dir().join(format!("{prefix}-{}.{extension}", std::process::id()))
}

fn stderr_hint(output: &Output) -> String {
    let stderr = String::from_utf8_lossy(&output.stderr);
    let stderr = stderr.trim();
    if stderr.is_empty() {
        String::new()
    } else {
        format!(": {stderr}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const EXPORT_SAMPLE: &str = r#"{
        "$schema": "https://aka.ms/winget-packages.schema.2.0.json",
        "Sources": [
            {
                "Packages": [
                    { "PackageIdentifier": "Microsoft.VisualStudioCode" },
                    { "PackageIdentifier": "Git.Git" },
                    { "PackageIdentifier": "Git.Git" }
                ],
                "SourceDetails": {
                    "Argument": "https://cdn.winget.microsoft.com/cache",
                    "Identifier": "Microsoft.Winget.Source_8wekyb3d8bbwe",
                    "Name": "winget",
                    "Type": "Microsoft.PreIndexed.Package"
                }
            },
            {
                "Packages": [
                    {
                        "InitialOverrideArguments": "--quiet --add Microsoft.VisualStudio.Workload.VCTools",
                        "PackageIdentifier": "Microsoft.VisualStudio.2022.BuildTools"
                    },
                    { "PackageIdentifier": "XP89DCGQ3K6VLD" }
                ],
                "SourceDetails": {
                    "Argument": "https://storeedgefd.dsx.mp.microsoft.com/v9.0",
                    "Identifier": "StoreEdgeFD",
                    "Name": "msstore",
                    "Type": "Microsoft.Rest"
                }
            }
        ]
    }"#;

    fn ids(apps: &[App]) -> Vec<&str> {
        apps.iter().map(App::id).collect()
    }

    #[test]
    fn reads_package_ids_from_winget_export() {
        let apps = parse_apps(EXPORT_SAMPLE).unwrap();
        assert_eq!(
            ids(&apps),
            vec![
                "Git.Git",
                "Microsoft.VisualStudio.2022.BuildTools",
                "Microsoft.VisualStudioCode",
                "XP89DCGQ3K6VLD"
            ]
        );
    }

    #[test]
    fn keeps_plain_packages_plain() {
        let apps = parse_apps(EXPORT_SAMPLE).unwrap();
        assert_eq!(apps[0], App::Id("Git.Git".to_string()));
    }

    #[test]
    fn keeps_store_packages_linked_to_their_source() {
        let apps = parse_apps(EXPORT_SAMPLE).unwrap();
        assert_eq!(apps[3].source(), Some("msstore"));
        assert_eq!(apps[2].source(), None);
    }

    #[test]
    fn keeps_installer_arguments() {
        let apps = parse_apps(EXPORT_SAMPLE).unwrap();
        assert_eq!(
            apps[1].arguments(),
            Some("--quiet --add Microsoft.VisualStudio.Workload.VCTools")
        );
    }

    #[test]
    fn tolerates_empty_export_files() {
        assert!(parse_apps(r#"{ "Sources": [] }"#).unwrap().is_empty());
    }

    #[test]
    fn rejects_invalid_json() {
        assert!(parse_apps("not json").is_err());
    }

    #[test]
    fn rebuilds_an_import_file_with_both_sources() {
        let apps = parse_apps(EXPORT_SAMPLE).unwrap();
        let value: serde_json::Value = serde_json::from_str(&import_json(&apps)).unwrap();
        assert_eq!(value["$schema"], PACKAGES_SCHEMA);
        assert_eq!(value["Sources"][0]["SourceDetails"]["Name"], "winget");
        assert_eq!(
            value["Sources"][0]["Packages"][0]["PackageIdentifier"],
            "Git.Git"
        );
        assert_eq!(value["Sources"][1]["SourceDetails"]["Name"], "msstore");
        assert_eq!(
            value["Sources"][1]["Packages"][0]["PackageIdentifier"],
            "Microsoft.VisualStudio.2022.BuildTools"
        );
        assert_eq!(
            value["Sources"][1]["Packages"][0]["InitialOverrideArguments"],
            "--quiet --add Microsoft.VisualStudio.Workload.VCTools"
        );
    }

    #[test]
    fn omits_the_store_source_when_nothing_comes_from_it() {
        let apps = vec![App::Id("Git.Git".to_string())];
        let value: serde_json::Value = serde_json::from_str(&import_json(&apps)).unwrap();
        assert_eq!(value["Sources"].as_array().unwrap().len(), 1);
        assert_eq!(value["Sources"][0]["SourceDetails"]["Name"], "winget");
    }

    #[test]
    fn treats_unknown_sources_as_the_default_one() {
        assert_eq!(source_for(None).name, "winget");
        assert_eq!(source_for(Some("msstore")).name, "msstore");
        assert_eq!(source_for(Some("MSSTORE")).name, "msstore");
        assert_eq!(source_for(Some("something-else")).name, "winget");
    }
}
