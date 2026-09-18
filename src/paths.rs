use std::path::{Path, PathBuf};

use anyhow::{Result, bail};

use crate::manifest::MANIFEST_FILE;

pub fn home_dir() -> Option<PathBuf> {
    if let Ok(profile) = std::env::var("USERPROFILE")
        && !profile.is_empty()
    {
        return Some(PathBuf::from(profile));
    }
    let drive = std::env::var("HOMEDRIVE").ok()?;
    let path = std::env::var("HOMEPATH").ok()?;
    if drive.is_empty() && path.is_empty() {
        return None;
    }
    Some(PathBuf::from(format!("{drive}{path}")))
}

pub fn expand_vars(input: &str) -> PathBuf {
    PathBuf::from(expand(input, &|name| std::env::var(name).ok()))
}

pub fn expand(input: &str, lookup: &dyn Fn(&str) -> Option<String>) -> String {
    let chars: Vec<char> = input.chars().collect();
    let mut out = String::with_capacity(input.len());
    let mut index = 0;

    while index < chars.len() {
        if chars[index] == '%' {
            if let Some(end) = chars[index + 1..].iter().position(|&c| c == '%') {
                let name: String = chars[index + 1..index + 1 + end].iter().collect();
                if !name.is_empty()
                    && let Some(value) = lookup(&name)
                {
                    out.push_str(&value);
                    index += end + 2;
                    continue;
                }
            }
            out.push('%');
            index += 1;
        } else if chars[index] == '~'
            && index == 0
            && matches!(chars.get(1), None | Some('\\') | Some('/'))
        {
            match lookup("USERPROFILE") {
                Some(home) => out.push_str(&home),
                None => out.push('~'),
            }
            index += 1;
        } else {
            out.push(chars[index]);
            index += 1;
        }
    }

    out
}

pub fn split_path_entries(value: &str) -> Vec<String> {
    value
        .split(';')
        .map(|entry| entry.trim())
        .filter(|entry| !entry.is_empty())
        .map(|entry| entry.to_string())
        .collect()
}

pub fn same_path(left: &str, right: &str) -> bool {
    let normalize = |value: &str| {
        expand_vars(value)
            .to_string_lossy()
            .replace('/', "\\")
            .trim_end_matches('\\')
            .to_ascii_lowercase()
    };
    normalize(left) == normalize(right)
}

pub fn resolve_bundle(arg: &Path) -> Result<(PathBuf, PathBuf)> {
    if arg.is_file() {
        let root = arg
            .parent()
            .filter(|parent| !parent.as_os_str().is_empty())
            .map(Path::to_path_buf)
            .unwrap_or_else(|| PathBuf::from("."));
        return Ok((arg.to_path_buf(), root));
    }
    if arg.is_dir() {
        let manifest = arg.join(MANIFEST_FILE);
        if manifest.is_file() {
            return Ok((manifest, arg.to_path_buf()));
        }
        bail!(
            "{} does not contain {} - pass the bundle directory or a manifest file",
            arg.display(),
            MANIFEST_FILE
        );
    }
    bail!("bundle not found: {}", arg.display());
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    fn lookup() -> HashMap<&'static str, &'static str> {
        HashMap::from([
            ("USERPROFILE", r"C:\Users\tester"),
            ("APPDATA", r"C:\Users\tester\AppData\Roaming"),
        ])
    }

    fn expand_with_test_vars(input: &str) -> String {
        let vars = lookup();
        expand(input, &|name| {
            vars.iter()
                .find(|(key, _)| key.eq_ignore_ascii_case(name))
                .map(|(_, value)| value.to_string())
        })
    }

    #[test]
    fn expands_percent_variables_case_insensitively() {
        assert_eq!(
            expand_with_test_vars(r"%USERPROFILE%\.gitconfig"),
            r"C:\Users\tester\.gitconfig"
        );
        assert_eq!(
            expand_with_test_vars("%appdata%\\Code\\User"),
            r"C:\Users\tester\AppData\Roaming\Code\User"
        );
    }

    #[test]
    fn expands_leading_tilde() {
        assert_eq!(
            expand_with_test_vars("~/.ssh/config"),
            r"C:\Users\tester/.ssh/config"
        );
        assert_eq!(expand_with_test_vars("~/"), r"C:\Users\tester/");
    }

    #[test]
    fn leaves_unknown_variables_untouched() {
        assert_eq!(
            expand_with_test_vars("%NOT_A_REAL_VAR%\\bin"),
            "%NOT_A_REAL_VAR%\\bin"
        );
        assert_eq!(expand_with_test_vars("50%"), "50%");
    }

    #[test]
    fn splices_expanded_values() {
        assert_eq!(
            expand_with_test_vars(r"%USERPROFILE%\bin;%APPDATA%\npm"),
            r"C:\Users\tester\bin;C:\Users\tester\AppData\Roaming\npm"
        );
    }

    #[test]
    fn splits_path_entries_and_drops_blanks() {
        assert_eq!(
            split_path_entries(r"C:\a;; C:\b ;"),
            vec![r"C:\a".to_string(), r"C:\b".to_string()]
        );
    }

    #[test]
    fn compares_paths_ignoring_case_and_trailing_separator() {
        assert!(same_path(r"C:\Tools\bin", r"c:\tools\bin\"));
        assert!(!same_path(r"C:\Tools\bin", r"C:\Tools\other"));
    }

    #[test]
    fn compares_paths_ignoring_separator_style() {
        assert!(same_path(r"C:\Tools\bin", "C:/Tools/bin"));
    }
}
