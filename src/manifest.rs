use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};

pub const MANIFEST_FILE: &str = "works-after-reinstall.yaml";
pub const CURRENT_VERSION: u32 = 1;

#[derive(Debug, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Manifest {
    /// Manifest format version.
    #[serde(default = "current_version")]
    pub version: u32,
    /// When the bundle was captured.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub exported_at: Option<String>,
    /// Machine the bundle was captured from.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_machine: Option<String>,
    /// WinGet packages to install.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub apps: Vec<App>,
    /// Windows preferences to reapply.
    #[serde(default, skip_serializing_if = "WindowsSettings::is_empty")]
    pub windows: WindowsSettings,
    /// Environment variables and PATH entries to restore.
    #[serde(default, skip_serializing_if = "Environment::is_empty")]
    pub environment: Environment,
    /// Files to copy back into place.
    #[serde(default, skip_serializing_if = "Files::is_empty")]
    pub files: Files,
}

#[derive(Debug, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WindowsSettings {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub dark_mode: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub show_file_extensions: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub show_hidden_files: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub developer_mode: Option<bool>,
}

#[derive(Debug, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Environment {
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub variables: BTreeMap<String, String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub path: Vec<String>,
}

#[derive(Debug, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Files {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub copy: Vec<FileCopy>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FileCopy {
    /// Path inside the bundle.
    pub source: String,
    /// Destination on the machine, with %VARS% and ~ expanded on apply.
    pub destination: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum App {
    /// A package from the default WinGet source.
    Id(String),
    /// A package that needs a specific source or installer arguments.
    Detailed(AppDetail),
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct AppDetail {
    pub id: String,
    /// WinGet source the package came from, e.g. msstore.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,
    /// Installer arguments the package was originally installed with.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub arguments: Option<String>,
}

impl App {
    pub fn id(&self) -> &str {
        match self {
            App::Id(id) => id,
            App::Detailed(detail) => &detail.id,
        }
    }

    pub fn source(&self) -> Option<&str> {
        match self {
            App::Id(_) => None,
            App::Detailed(detail) => detail.source.as_deref(),
        }
    }

    pub fn arguments(&self) -> Option<&str> {
        match self {
            App::Id(_) => None,
            App::Detailed(detail) => detail.arguments.as_deref(),
        }
    }
}

impl Manifest {
    pub fn load(path: &Path) -> Result<Self> {
        let text =
            fs::read_to_string(path).with_context(|| format!("reading {}", path.display()))?;
        let manifest: Manifest =
            serde_yaml::from_str(&text).with_context(|| format!("parsing {}", path.display()))?;
        manifest.validate()?;
        Ok(manifest)
    }

    pub fn save(&self, path: &Path) -> Result<()> {
        let text = serde_yaml::to_string(self).context("serializing manifest")?;
        fs::write(path, text).with_context(|| format!("writing {}", path.display()))?;
        Ok(())
    }

    fn validate(&self) -> Result<()> {
        if self.version > CURRENT_VERSION {
            bail!(
                "manifest version {} is newer than this build understands (max {})",
                self.version,
                CURRENT_VERSION
            );
        }
        for app in &self.apps {
            if app.id().trim().is_empty() {
                bail!("apps contains an entry without a package identifier");
            }
        }
        for copy in &self.files.copy {
            if copy.source.trim().is_empty() {
                bail!("files.copy contains an entry without a source");
            }
            if copy.destination.trim().is_empty() {
                bail!("files.copy contains an entry without a destination");
            }
        }
        Ok(())
    }
}

impl WindowsSettings {
    pub fn is_empty(&self) -> bool {
        self.dark_mode.is_none()
            && self.show_file_extensions.is_none()
            && self.show_hidden_files.is_none()
            && self.developer_mode.is_none()
    }
}

impl Environment {
    pub fn is_empty(&self) -> bool {
        self.variables.is_empty() && self.path.is_empty()
    }
}

impl Files {
    pub fn is_empty(&self) -> bool {
        self.copy.is_empty()
    }
}

fn current_version() -> u32 {
    CURRENT_VERSION
}

#[cfg(test)]
mod tests {
    use super::*;

    const README_EXAMPLE: &str = r#"
apps:
  - Google.Chrome
  - Microsoft.VisualStudioCode
  - Git.Git
  - OpenJS.NodeJS
  - 7zip.7zip

windows:
  dark_mode: true
  show_file_extensions: true
  show_hidden_files: true
  developer_mode: true

environment:
  variables:
    EDITOR: code
    NODE_ENV: development

files:
  copy:
    - source: ".gitconfig"
      destination: "%USERPROFILE%\\.gitconfig"
"#;

    #[test]
    fn reads_the_documented_example() {
        let manifest: Manifest = serde_yaml::from_str(README_EXAMPLE).unwrap();
        assert_eq!(manifest.version, CURRENT_VERSION);
        assert_eq!(manifest.apps.len(), 5);
        assert_eq!(manifest.windows.dark_mode, Some(true));
        assert_eq!(manifest.windows.developer_mode, Some(true));
        assert_eq!(
            manifest.environment.variables.get("EDITOR"),
            Some(&"code".to_string())
        );
        assert_eq!(manifest.environment.variables.len(), 2);
        assert_eq!(manifest.files.copy.len(), 1);
        assert_eq!(
            manifest.files.copy[0].destination,
            r"%USERPROFILE%\.gitconfig"
        );
    }

    #[test]
    fn round_trips_without_losing_fields() {
        let manifest: Manifest = serde_yaml::from_str(README_EXAMPLE).unwrap();
        let text = serde_yaml::to_string(&manifest).unwrap();
        let again: Manifest = serde_yaml::from_str(&text).unwrap();
        assert_eq!(again.apps, manifest.apps);
        assert_eq!(again.windows.dark_mode, manifest.windows.dark_mode);
        assert_eq!(again.files.copy, manifest.files.copy);
    }

    #[test]
    fn empty_sections_are_omitted_from_output() {
        let manifest = Manifest {
            apps: vec![App::Id("Git.Git".into())],
            ..Default::default()
        };
        let text = serde_yaml::to_string(&manifest).unwrap();
        assert!(text.contains("apps:"));
        assert!(text.contains("- Git.Git"));
        assert!(!text.contains("environment:"));
        assert!(!text.contains("windows:"));
    }

    #[test]
    fn reads_apps_that_carry_a_source_and_arguments() {
        let manifest: Manifest = serde_yaml::from_str(
            r#"
apps:
  - Git.Git
  - id: Microsoft.VisualStudio.2022.BuildTools
    arguments: "--quiet --wait --add Microsoft.VisualStudio.Workload.VCTools"
  - id: XP89DCGQ3K6VLD
    source: msstore
"#,
        )
        .unwrap();

        assert_eq!(manifest.apps.len(), 3);
        assert_eq!(manifest.apps[0].id(), "Git.Git");
        assert_eq!(manifest.apps[0].source(), None);
        assert_eq!(
            manifest.apps[1].arguments(),
            Some("--quiet --wait --add Microsoft.VisualStudio.Workload.VCTools")
        );
        assert_eq!(manifest.apps[2].id(), "XP89DCGQ3K6VLD");
        assert_eq!(manifest.apps[2].source(), Some("msstore"));
    }

    #[test]
    fn writes_detailed_apps_back_in_detailed_form() {
        let manifest: Manifest = serde_yaml::from_str(
            r#"
apps:
  - id: XP89DCGQ3K6VLD
    source: msstore
"#,
        )
        .unwrap();
        let text = serde_yaml::to_string(&manifest).unwrap();
        assert!(text.contains("source: msstore"));
        assert!(text.contains("id: XP89DCGQ3K6VLD"));
    }

    #[test]
    fn rejects_unknown_fields() {
        let result = serde_yaml::from_str::<Manifest>("widnows:\n  dark_mode: true\n");
        assert!(result.is_err());
    }

    #[test]
    fn rejects_newer_manifest_versions() {
        let manifest = Manifest {
            version: CURRENT_VERSION + 1,
            ..Default::default()
        };
        assert!(manifest.validate().is_err());
    }

    #[test]
    fn spots_empty_settings_sections() {
        let settings = WindowsSettings {
            dark_mode: Some(true),
            ..Default::default()
        };
        assert!(!settings.is_empty());
        assert!(WindowsSettings::default().is_empty());
    }
}
