use std::borrow::Cow;
use std::io;

use anyhow::{Context, Result, bail};
use winreg::enums::{HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE, KEY_READ, KEY_SET_VALUE, REG_DWORD};
use winreg::{HKEY, RegKey, RegValue};

use crate::console::Console;
use crate::manifest::WindowsSettings;
use crate::win::{broadcast_setting_change, is_admin};

const PERSONALIZE: &str = r"Software\Microsoft\Windows\CurrentVersion\Themes\Personalize";
const ADVANCED: &str = r"Software\Microsoft\Windows\CurrentVersion\Explorer\Advanced";
const APP_MODEL_UNLOCK: &str = r"Software\Microsoft\Windows\CurrentVersion\AppModelUnlock";

const APPS_USE_LIGHT_THEME: &str = "AppsUseLightTheme";
const SYSTEM_USES_LIGHT_THEME: &str = "SystemUsesLightTheme";
const HIDE_FILE_EXT: &str = "HideFileExt";
const HIDDEN: &str = "Hidden";
const ALLOW_DEVELOPMENT: &str = "AllowDevelopmentWithoutDevLicense";

pub fn capture() -> Result<WindowsSettings> {
    Ok(WindowsSettings {
        dark_mode: read_dword(HKEY_CURRENT_USER, PERSONALIZE, APPS_USE_LIGHT_THEME)
            .map(|value| value == 0),
        show_file_extensions: read_dword(HKEY_CURRENT_USER, ADVANCED, HIDE_FILE_EXT)
            .map(|value| value == 0),
        show_hidden_files: read_dword(HKEY_CURRENT_USER, ADVANCED, HIDDEN).map(|value| value == 1),
        developer_mode: read_dword(HKEY_LOCAL_MACHINE, APP_MODEL_UNLOCK, ALLOW_DEVELOPMENT)
            .map(|value| value == 1),
    })
}

pub fn summary(settings: &WindowsSettings) -> String {
    let mut parts = Vec::new();
    if let Some(value) = settings.dark_mode {
        parts.push(format!("dark mode {}", on_off(value)));
    }
    if let Some(value) = settings.show_file_extensions {
        parts.push(format!(
            "file extensions {}",
            if value { "shown" } else { "hidden" }
        ));
    }
    if let Some(value) = settings.show_hidden_files {
        parts.push(format!(
            "hidden files {}",
            if value { "shown" } else { "hidden" }
        ));
    }
    if let Some(value) = settings.developer_mode {
        parts.push(format!("developer mode {}", on_off(value)));
    }
    if parts.is_empty() {
        "nothing found".to_string()
    } else {
        parts.join(", ")
    }
}

pub fn apply(settings: &WindowsSettings, dry_run: bool, console: &Console) -> Result<()> {
    if settings.is_empty() {
        console.line("  nothing to apply");
        return Ok(());
    }

    if let Some(dark) = settings.dark_mode {
        console.line(format!("  dark mode -> {}", on_off(dark)));
        let light_theme = u32::from(!dark);
        write_dword(
            HKEY_CURRENT_USER,
            "HKCU",
            PERSONALIZE,
            APPS_USE_LIGHT_THEME,
            light_theme,
            dry_run,
        )?;
        write_dword(
            HKEY_CURRENT_USER,
            "HKCU",
            PERSONALIZE,
            SYSTEM_USES_LIGHT_THEME,
            light_theme,
            dry_run,
        )?;
    }

    if let Some(show) = settings.show_file_extensions {
        console.line(format!(
            "  file extensions -> {}",
            if show { "shown" } else { "hidden" }
        ));
        write_dword(
            HKEY_CURRENT_USER,
            "HKCU",
            ADVANCED,
            HIDE_FILE_EXT,
            u32::from(!show),
            dry_run,
        )?;
    }

    if let Some(show) = settings.show_hidden_files {
        console.line(format!(
            "  hidden files -> {}",
            if show { "shown" } else { "hidden" }
        ));
        write_dword(
            HKEY_CURRENT_USER,
            "HKCU",
            ADVANCED,
            HIDDEN,
            if show { 1 } else { 2 },
            dry_run,
        )?;
    }

    if let Some(enabled) = settings.developer_mode {
        console.line(format!("  developer mode -> {}", on_off(enabled)));
        write_dword(
            HKEY_LOCAL_MACHINE,
            "HKLM",
            APP_MODEL_UNLOCK,
            ALLOW_DEVELOPMENT,
            u32::from(enabled),
            dry_run,
        )?;
    }

    if !dry_run {
        broadcast_setting_change(ADVANCED);
        if settings.show_file_extensions.is_some() || settings.show_hidden_files.is_some() {
            console.line("  explorer may need a restart before these appear");
        }
    }

    Ok(())
}

fn read_dword(root: HKEY, path: &str, name: &str) -> Option<u32> {
    let key = RegKey::predef(root)
        .open_subkey_with_flags(path, KEY_READ)
        .ok()?;
    let raw = key.get_raw_value(name).ok()?;
    if raw.vtype != REG_DWORD || raw.bytes.len() < 4 {
        return None;
    }
    Some(u32::from_le_bytes([
        raw.bytes[0],
        raw.bytes[1],
        raw.bytes[2],
        raw.bytes[3],
    ]))
}

fn write_dword(
    root: HKEY,
    hive: &str,
    path: &str,
    name: &str,
    value: u32,
    dry_run: bool,
) -> Result<()> {
    if dry_run {
        return Ok(());
    }

    let (key, _) = RegKey::predef(root)
        .create_subkey_with_flags(path, KEY_SET_VALUE)
        .with_context(|| format!("opening {hive}\\{path} for writing"))?;

    let raw = RegValue {
        bytes: Cow::Owned(value.to_le_bytes().to_vec()),
        vtype: REG_DWORD,
    };

    if let Err(error) = key.set_raw_value(name, &raw) {
        if error.kind() == io::ErrorKind::PermissionDenied || error.raw_os_error() == Some(5) {
            if is_admin() {
                bail!("{hive}\\{path}\\{name} is locked down by policy on this machine");
            }
            bail!(
                "{hive}\\{path}\\{name} needs administrator rights - run the command from a terminal opened as Administrator"
            );
        }
        return Err(error).with_context(|| format!("writing {hive}\\{path}\\{name}"));
    }

    Ok(())
}

fn on_off(value: bool) -> &'static str {
    if value { "on" } else { "off" }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn summarizes_what_was_captured() {
        let settings = WindowsSettings {
            dark_mode: Some(true),
            show_file_extensions: Some(true),
            show_hidden_files: Some(false),
            developer_mode: Some(true),
        };
        assert_eq!(
            summary(&settings),
            "dark mode on, file extensions shown, hidden files hidden, developer mode on"
        );
    }

    #[test]
    fn summarizes_an_empty_capture() {
        assert_eq!(summary(&WindowsSettings::default()), "nothing found");
    }

    #[test]
    fn formats_booleans() {
        assert_eq!(on_off(true), "on");
        assert_eq!(on_off(false), "off");
    }
}
