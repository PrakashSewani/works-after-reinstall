use std::borrow::Cow;
use std::collections::BTreeMap;
use std::io;

use anyhow::{Context, Result};
use winreg::enums::{HKEY_CURRENT_USER, KEY_READ, KEY_SET_VALUE, REG_DWORD, REG_EXPAND_SZ, REG_SZ};
use winreg::{RegKey, RegValue};

use crate::console::Console;
use crate::paths::{same_path, split_path_entries};
use crate::win::broadcast_setting_change;

const ENVIRONMENT_KEY: &str = "Environment";
const PATH_VALUE: &str = "Path";
const DEFAULT_PATH_ENTRY: &str = r"%USERPROFILE%\AppData\Local\Microsoft\WindowsApps";

const WINDOWS_DEFAULTS: &[&str] = &[
    "allusersprofile",
    "appdata",
    "computername",
    "comspec",
    "driverdata",
    "homedrive",
    "homepath",
    "localappdata",
    "logonserver",
    "number_of_processors",
    "onedrive",
    "pathext",
    "processor_architecture",
    "processor_identifier",
    "processor_level",
    "processor_revision",
    "programdata",
    "programfiles",
    "programfiles(x86)",
    "programw6432",
    "psmodulepath",
    "public",
    "sessionname",
    "systemdrive",
    "systemroot",
    "temp",
    "tmp",
    "userdomain",
    "userdomain_roamingprofile",
    "username",
    "userprofile",
    "windir",
];

pub fn capture() -> Result<(BTreeMap<String, String>, Vec<String>)> {
    let Ok(key) = open(KEY_READ) else {
        return Ok((BTreeMap::new(), Vec::new()));
    };

    let mut variables = BTreeMap::new();
    let mut path_entries = Vec::new();

    for (name, value) in key.enum_values().filter_map(|entry| entry.ok()) {
        if name.starts_with('=') {
            continue;
        }
        let Some(text) = decode(&value) else {
            continue;
        };
        if name.eq_ignore_ascii_case(PATH_VALUE) {
            path_entries = split_path_entries(&text)
                .into_iter()
                .filter(|entry| !is_default_path_entry(entry))
                .collect();
            continue;
        }
        if is_windows_default(&name) || text.trim().is_empty() {
            continue;
        }
        variables.insert(name, text);
    }

    Ok((variables, path_entries))
}

pub fn apply(
    variables: &BTreeMap<String, String>,
    additions: &[String],
    dry_run: bool,
    console: &Console,
) -> Result<()> {
    if variables.is_empty() && additions.is_empty() {
        console.line("  nothing to restore");
        return Ok(());
    }

    let key = open(if dry_run {
        KEY_READ
    } else {
        KEY_READ | KEY_SET_VALUE
    })
    .context(r"opening HKCU\Environment")?;

    let mut changed = false;

    for (name, value) in variables {
        let existing = key.get_raw_value(name).ok().and_then(|raw| decode(&raw));
        if existing.as_deref() == Some(value.as_str()) {
            console.line(format!("  {name} already set"));
            continue;
        }
        console.line(format!("  set {name} = {value}"));
        if dry_run {
            continue;
        }
        let raw = encode(value);
        key.set_raw_value(name, &raw)
            .with_context(|| format!(r"writing HKCU\Environment\{name}"))?;
        changed = true;
    }

    let existing_path = key.get_raw_value(PATH_VALUE).ok();
    let existing_expand = existing_path
        .as_ref()
        .map(|raw| raw.vtype == REG_EXPAND_SZ)
        .unwrap_or(false);
    let current = match existing_path.as_ref() {
        Some(raw) => decode(raw).with_context(|| {
            format!(r"HKCU\Environment\{PATH_VALUE} holds a value type this tool cannot merge")
        })?,
        None => String::new(),
    };

    let mut entries = split_path_entries(&current);
    let mut path_changed = false;

    for addition in additions {
        if entries.iter().any(|entry| same_path(entry, addition)) {
            console.line(format!("  Path already contains {addition}"));
            continue;
        }
        console.line(format!("  Path += {addition}"));
        entries.push(addition.clone());
        path_changed = true;
    }

    if path_changed && !dry_run {
        let joined = entries.join(";");
        let raw = encode_as(&joined, existing_expand);
        key.set_raw_value(PATH_VALUE, &raw)
            .with_context(|| format!(r"writing HKCU\Environment\{PATH_VALUE}"))?;
        changed = true;
    }

    if changed {
        broadcast_setting_change("Environment");
    } else {
        console.line("  nothing to change");
    }

    Ok(())
}

fn open(perms: u32) -> io::Result<RegKey> {
    RegKey::predef(HKEY_CURRENT_USER).open_subkey_with_flags(ENVIRONMENT_KEY, perms)
}

fn is_windows_default(name: &str) -> bool {
    let lower = name.to_ascii_lowercase();
    WINDOWS_DEFAULTS.contains(&lower.as_str())
}

fn is_default_path_entry(entry: &str) -> bool {
    same_path(entry, DEFAULT_PATH_ENTRY)
}

fn encode(value: &str) -> RegValue<'static> {
    let expand = value.contains('%');
    encode_as(value, expand)
}

fn encode_as(value: &str, expand: bool) -> RegValue<'static> {
    let mut bytes = Vec::with_capacity(value.len() * 2 + 2);
    for unit in value.encode_utf16() {
        bytes.extend_from_slice(&unit.to_le_bytes());
    }
    bytes.extend_from_slice(&[0, 0]);
    RegValue {
        bytes: Cow::Owned(bytes),
        vtype: if expand { REG_EXPAND_SZ } else { REG_SZ },
    }
}

fn decode(raw: &RegValue) -> Option<String> {
    match raw.vtype {
        REG_SZ | REG_EXPAND_SZ => Some(decode_utf16(&raw.bytes)),
        REG_DWORD if raw.bytes.len() >= 4 => Some(
            u32::from_le_bytes([raw.bytes[0], raw.bytes[1], raw.bytes[2], raw.bytes[3]])
                .to_string(),
        ),
        _ => None,
    }
}

fn decode_utf16(bytes: &[u8]) -> String {
    let mut units: Vec<u16> = bytes
        .as_chunks::<2>()
        .0
        .iter()
        .map(|pair| u16::from_le_bytes(*pair))
        .collect();
    while units.last() == Some(&0) {
        units.pop();
    }
    String::from_utf16_lossy(&units)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trips_plain_values() {
        let raw = encode("code");
        assert_eq!(raw.vtype, REG_SZ);
        assert_eq!(decode(&raw), Some("code".to_string()));
    }

    #[test]
    fn keeps_expandable_values_expandable() {
        let raw = encode(r"%USERPROFILE%\.cargo\bin");
        assert_eq!(raw.vtype, REG_EXPAND_SZ);
        assert_eq!(decode(&raw), Some(r"%USERPROFILE%\.cargo\bin".to_string()));
    }

    #[test]
    fn round_trips_unicode() {
        let raw = encode(r"C:\Users\tester\café");
        assert_eq!(decode(&raw), Some(r"C:\Users\tester\café".to_string()));
    }

    #[test]
    fn reads_dword_values_as_text() {
        let raw = RegValue {
            bytes: Cow::Owned(7u32.to_le_bytes().to_vec()),
            vtype: REG_DWORD,
        };
        assert_eq!(decode(&raw), Some("7".to_string()));
    }

    #[test]
    fn ignores_types_it_cannot_represent() {
        let raw = RegValue {
            bytes: Cow::Owned(vec![1, 2, 3, 4]),
            vtype: winreg::enums::REG_BINARY,
        };
        assert_eq!(decode(&raw), None);
    }

    #[test]
    fn filters_windows_defaults_case_insensitively() {
        assert!(is_windows_default("TEMP"));
        assert!(is_windows_default("OneDrive"));
        assert!(is_windows_default("PSModulePath"));
        assert!(!is_windows_default("EDITOR"));
        assert!(!is_windows_default("JAVA_HOME"));
    }

    #[test]
    fn recognizes_the_default_windows_apps_path_entry() {
        assert!(is_default_path_entry(
            r"%USERPROFILE%/AppData/Local/Microsoft/WindowsApps"
        ));
        assert!(!is_default_path_entry(r"%USERPROFILE%\.cargo\bin"));
    }

    #[test]
    fn uses_existing_path_type_when_merging() {
        let raw = encode_as(r"C:\tools\bin", false);
        assert_eq!(raw.vtype, REG_SZ);
        let raw = encode_as(r"C:\tools\bin", true);
        assert_eq!(raw.vtype, REG_EXPAND_SZ);
    }
}
