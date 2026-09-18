use std::collections::BTreeMap;

use crate::manifest::{App, CURRENT_VERSION, Environment, Files, Manifest, WindowsSettings};
use crate::util::now_iso8601;

pub struct CatalogApp {
    pub category: &'static str,
    pub name: &'static str,
    pub id: &'static str,
    pub note: &'static str,
}

pub const CATEGORIES: [&str; 7] = [
    "Browsers",
    "Communication",
    "Development",
    "Runtimes",
    "Media",
    "Utilities",
    "Gaming",
];

/// Curated starter packages. Every id here is a real WinGet package id, so
/// `setup` can install them on a machine that has nothing exported.
pub const APPS: &[CatalogApp] = &[
    CatalogApp {
        category: "Browsers",
        name: "Chrome",
        id: "Google.Chrome",
        note: "Google's browser",
    },
    CatalogApp {
        category: "Browsers",
        name: "Firefox",
        id: "Mozilla.Firefox",
        note: "Mozilla's browser",
    },
    CatalogApp {
        category: "Browsers",
        name: "Brave",
        id: "Brave.Brave",
        note: "Chromium with built-in blocking",
    },
    CatalogApp {
        category: "Browsers",
        name: "Opera",
        id: "Opera.Opera",
        note: "",
    },
    CatalogApp {
        category: "Browsers",
        name: "Vivaldi",
        id: "Vivaldi.Vivaldi",
        note: "",
    },
    CatalogApp {
        category: "Communication",
        name: "Discord",
        id: "Discord.Discord",
        note: "",
    },
    CatalogApp {
        category: "Communication",
        name: "Slack",
        id: "SlackTechnologies.Slack",
        note: "",
    },
    CatalogApp {
        category: "Communication",
        name: "Zoom",
        id: "Zoom.Zoom",
        note: "",
    },
    CatalogApp {
        category: "Communication",
        name: "Teams",
        id: "Microsoft.Teams",
        note: "",
    },
    CatalogApp {
        category: "Communication",
        name: "Telegram",
        id: "Telegram.TelegramDesktop",
        note: "",
    },
    CatalogApp {
        category: "Development",
        name: "Git",
        id: "Git.Git",
        note: "version control",
    },
    CatalogApp {
        category: "Development",
        name: "VS Code",
        id: "Microsoft.VisualStudioCode",
        note: "editor",
    },
    CatalogApp {
        category: "Development",
        name: "Windows Terminal",
        id: "Microsoft.WindowsTerminal",
        note: "tabs, panes, good fonts",
    },
    CatalogApp {
        category: "Development",
        name: "PowerShell",
        id: "Microsoft.PowerShell",
        note: "PowerShell 7",
    },
    CatalogApp {
        category: "Development",
        name: "Node.js LTS",
        id: "OpenJS.NodeJS.LTS",
        note: "",
    },
    CatalogApp {
        category: "Development",
        name: "Python 3.13",
        id: "Python.Python.3.13",
        note: "adds python and the py launcher",
    },
    CatalogApp {
        category: "Development",
        name: "Rust",
        id: "Rustlang.Rustup",
        note: "rustup",
    },
    CatalogApp {
        category: "Development",
        name: "GitHub CLI",
        id: "GitHub.cli",
        note: "",
    },
    CatalogApp {
        category: "Development",
        name: "Docker Desktop",
        id: "Docker.DockerDesktop",
        note: "needs WSL2",
    },
    CatalogApp {
        category: "Development",
        name: "Postman",
        id: "Postman.Postman",
        note: "",
    },
    CatalogApp {
        category: "Development",
        name: "DBeaver",
        id: "DBeaver.DBeaver.Community",
        note: "database client",
    },
    CatalogApp {
        category: "Development",
        name: "JetBrains Toolbox",
        id: "JetBrains.Toolbox",
        note: "manages JetBrains IDEs",
    },
    CatalogApp {
        category: "Development",
        name: "Visual Studio 2022 Community",
        id: "Microsoft.VisualStudio.2022.Community",
        note: "large download",
    },
    CatalogApp {
        category: "Runtimes",
        name: ".NET 8 Desktop Runtime",
        id: "Microsoft.DotNet.DesktopRuntime.8",
        note: "needed by many apps",
    },
    CatalogApp {
        category: "Runtimes",
        name: "Java 21 (Temurin)",
        id: "EclipseAdoptium.Temurin.21.JDK",
        note: "",
    },
    CatalogApp {
        category: "Runtimes",
        name: "Visual C++ Redistributable",
        id: "Microsoft.VCRedist.2015+.x64",
        note: "",
    },
    CatalogApp {
        category: "Media",
        name: "VLC",
        id: "VideoLAN.VLC",
        note: "plays everything",
    },
    CatalogApp {
        category: "Media",
        name: "Spotify",
        id: "Spotify.Spotify",
        note: "",
    },
    CatalogApp {
        category: "Media",
        name: "OBS Studio",
        id: "OBSProject.OBSStudio",
        note: "screen recording",
    },
    CatalogApp {
        category: "Media",
        name: "GIMP",
        id: "GIMP.GIMP",
        note: "image editing",
    },
    CatalogApp {
        category: "Media",
        name: "Krita",
        id: "KDE.Krita",
        note: "digital painting",
    },
    CatalogApp {
        category: "Media",
        name: "Audacity",
        id: "Audacity.Audacity",
        note: "",
    },
    CatalogApp {
        category: "Media",
        name: "HandBrake",
        id: "HandBrake.HandBrake",
        note: "video conversion",
    },
    CatalogApp {
        category: "Media",
        name: "mpv.net",
        id: "mpv.net",
        note: "minimal video player",
    },
    CatalogApp {
        category: "Utilities",
        name: "7-Zip",
        id: "7zip.7zip",
        note: "archives",
    },
    CatalogApp {
        category: "Utilities",
        name: "Notepad++",
        id: "Notepad++.Notepad++",
        note: "",
    },
    CatalogApp {
        category: "Utilities",
        name: "PowerToys",
        id: "Microsoft.PowerToys",
        note: "Microsoft's power user tools",
    },
    CatalogApp {
        category: "Utilities",
        name: "Everything",
        id: "voidtools.Everything",
        note: "instant file search",
    },
    CatalogApp {
        category: "Utilities",
        name: "ShareX",
        id: "ShareX.ShareX",
        note: "screenshots",
    },
    CatalogApp {
        category: "Utilities",
        name: "Rufus",
        id: "Rufus.Rufus",
        note: "bootable USB drives",
    },
    CatalogApp {
        category: "Utilities",
        name: "WinDirStat",
        id: "WinDirStat.WinDirStat",
        note: "disk usage",
    },
    CatalogApp {
        category: "Utilities",
        name: "Bitwarden",
        id: "Bitwarden.Bitwarden",
        note: "password manager",
    },
    CatalogApp {
        category: "Utilities",
        name: "qBittorrent",
        id: "qBittorrent.qBittorrent",
        note: "",
    },
    CatalogApp {
        category: "Utilities",
        name: "LibreOffice",
        id: "TheDocumentFoundation.LibreOffice",
        note: "office suite",
    },
    CatalogApp {
        category: "Utilities",
        name: "Acrobat Reader",
        id: "Adobe.Acrobat.Reader.64-bit",
        note: "PDF",
    },
    CatalogApp {
        category: "Gaming",
        name: "Steam",
        id: "Valve.Steam",
        note: "",
    },
    CatalogApp {
        category: "Gaming",
        name: "Epic Games Launcher",
        id: "EpicGames.EpicGamesLauncher",
        note: "",
    },
    CatalogApp {
        category: "Gaming",
        name: "GOG Galaxy",
        id: "GOG.Galaxy",
        note: "",
    },
];

#[derive(Clone, Copy, Debug)]
pub struct StarterChoices {
    pub windows_settings: bool,
    pub environment: bool,
}

/// The manifest a fresh machine can be set up from, with no export involved.
pub fn starter_manifest(ids: &[String], choices: StarterChoices) -> Manifest {
    Manifest {
        version: CURRENT_VERSION,
        exported_at: Some(now_iso8601()),
        source_machine: None,
        apps: ids.iter().cloned().map(App::Id).collect(),
        windows: if choices.windows_settings {
            WindowsSettings {
                dark_mode: Some(true),
                show_file_extensions: Some(true),
                show_hidden_files: None,
                developer_mode: Some(true),
            }
        } else {
            WindowsSettings::default()
        },
        environment: if choices.environment {
            Environment {
                variables: BTreeMap::from([
                    ("EDITOR".to_string(), "code".to_string()),
                    ("DOTNET_CLI_TELEMETRY_OPTOUT".to_string(), "1".to_string()),
                ]),
                path: Vec::new(),
            }
        } else {
            Environment::default()
        },
        files: Files::default(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeSet;

    #[test]
    fn every_app_has_a_category_from_the_list() {
        let known: BTreeSet<&str> = CATEGORIES.into_iter().collect();
        for app in APPS {
            assert!(
                known.contains(app.category),
                "{} uses unknown category {}",
                app.id,
                app.category
            );
        }
    }

    #[test]
    fn app_ids_are_unique_and_look_like_package_ids() {
        let mut seen = BTreeSet::new();
        for app in APPS {
            assert!(seen.insert(app.id), "duplicate id {}", app.id);
            assert!(!app.id.contains(' '), "{} looks wrong", app.id);
            assert!(!app.name.is_empty());
        }
    }

    #[test]
    fn every_category_has_at_least_one_app() {
        for category in CATEGORIES {
            assert!(
                APPS.iter().any(|app| app.category == category),
                "{category} is empty"
            );
        }
    }

    #[test]
    fn starter_manifest_carries_the_chosen_apps_and_defaults() {
        let ids = vec!["Git.Git".to_string(), "VideoLAN.VLC".to_string()];
        let manifest = starter_manifest(
            &ids,
            StarterChoices {
                windows_settings: true,
                environment: true,
            },
        );

        assert_eq!(manifest.apps.len(), 2);
        assert_eq!(manifest.apps[0], App::Id("Git.Git".to_string()));
        assert_eq!(manifest.windows.show_file_extensions, Some(true));
        assert_eq!(manifest.windows.developer_mode, Some(true));
        assert_eq!(manifest.windows.show_hidden_files, None);
        assert_eq!(
            manifest.environment.variables.get("EDITOR"),
            Some(&"code".to_string())
        );
        assert!(manifest.files.copy.is_empty());
    }

    #[test]
    fn starter_manifest_can_leave_the_extras_out() {
        let manifest = starter_manifest(
            &[],
            StarterChoices {
                windows_settings: false,
                environment: false,
            },
        );

        assert!(manifest.windows.is_empty());
        assert!(manifest.environment.is_empty());
        assert!(manifest.apps.is_empty());
    }
}
