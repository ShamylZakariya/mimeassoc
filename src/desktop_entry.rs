use serde::Serialize;
use std::{
    collections::HashMap,
    fmt::Display,
    fs::File,
    io::{self, BufRead},
    path::{Path, PathBuf},
};

use is_executable::IsExecutable;

use super::MimeType;

#[derive(Debug, PartialEq, Eq, Hash, Clone, Serialize)]
pub struct DesktopEntryId(String);

impl DesktopEntryId {
    pub fn parse(desktop_entry: &str) -> anyhow::Result<DesktopEntryId> {
        if desktop_entry.ends_with(".desktop") {
            Ok(DesktopEntryId(desktop_entry.to_string()))
        } else {
            anyhow::bail!(
                "id: \"{}\" not a valid Gnome .desktop file name",
                desktop_entry
            )
        }
    }

    pub fn id(&self) -> &str {
        &self.0
    }
}

impl PartialOrd for DesktopEntryId {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        self.0.partial_cmp(&other.0)
    }
}

impl Ord for DesktopEntryId {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.0.cmp(&other.0)
    }
}

impl Display for DesktopEntryId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum DesktopEntryType {
    Application,
    Other,
}

impl DesktopEntryType {
    fn parse(text: &str) -> Self {
        let text = text.trim();
        match text {
            "Application" => Self::Application,
            _ => Self::Other,
        }
    }
}

enum DesktopEntrySections {
    // For now, we only care about "[Desktop Entry]". The other fields aren't relevant
    DesktopEntry,
}

impl DesktopEntrySections {
    fn appears_to_be_desktop_entry_line(line: &str) -> bool {
        let line = line.trim();
        line.starts_with('[') && line.ends_with(']')
    }

    fn try_parse(desc: &str) -> Option<Self> {
        let desc = desc.trim();
        if desc == "[Desktop Entry]" {
            Some(Self::DesktopEntry)
        } else {
            None
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DesktopEntry {
    path: PathBuf,
    fields: HashMap<String, String>,
    mime_types: Vec<MimeType>,
    id: DesktopEntryId,
}

impl DesktopEntry {
    pub fn load<P>(path: P) -> anyhow::Result<DesktopEntry>
    where
        P: AsRef<Path>,
    {
        let path = path.as_ref();
        let file = File::open(path)?;
        let line_buffer = io::BufReader::new(file).lines();
        let mut desktop_entry_section: Option<DesktopEntrySections> = None;
        let mut fields = HashMap::new();
        let mut mime_types: Vec<MimeType> = vec![];

        for line in line_buffer.flatten() {
            let line = line.trim();
            if line.starts_with('#') || line.is_empty() {
                continue;
            }

            if DesktopEntrySections::appears_to_be_desktop_entry_line(line) {
                desktop_entry_section = DesktopEntrySections::try_parse(line);
            } else if desktop_entry_section.is_some() {
                let field_components = line.splitn(2, '=').collect::<Vec<_>>();
                if field_components.len() != 2 {
                    anyhow::bail!("Desktop entry field entries should be in form \"Name=value\", line is malformed: \"{}\"", line);
                }
                let field_name = field_components[0].trim();
                let field_value = field_components[1].trim();
                if field_name == "MimeType" {
                    let mime_type_strings = field_value.split(';').collect::<Vec<_>>();
                    for mime_type_str in mime_type_strings {
                        if let Ok(mime_type) = MimeType::parse(mime_type_str) {
                            mime_types.push(mime_type)
                        }
                    }
                } else {
                    fields.insert(field_name.to_owned(), field_value.to_owned());
                }
            }
        }

        if fields.is_empty() {
            anyhow::bail!(
                "DesktopEntry \"{:?}\" parsed but contained no [Desktop Entry] fields.",
                path
            );
        }

        let id = if let Some(file_name) = path.file_name() {
            if let Some(file_name) = file_name.to_str() {
                DesktopEntryId::parse(file_name)?
            } else {
                anyhow::bail!(
                    "Unable to extract DesktopEntryId from desktop entry \"{:?}\"",
                    path
                )
            }
        } else {
            anyhow::bail!(
                "Unable to extract DesktopEntryId from desktop entry \"{:?}\"",
                path
            )
        };

        Ok(Self {
            path: PathBuf::from(path),
            fields,
            mime_types,
            id,
        })
    }

    pub fn name(&self) -> Option<&str> {
        self.fields.get("Name").map(|v| v.as_str())
    }

    pub fn cmp_by_name_alpha_inensitive(&self, other: &DesktopEntry) -> core::cmp::Ordering {
        let name = self.name().unwrap_or(&self.id.0).to_lowercase();
        let other_name = other.name().unwrap_or(&other.id.0).to_lowercase();
        name.cmp(&other_name)
    }

    pub fn id(&self) -> &DesktopEntryId {
        &self.id
    }

    pub fn localised_name(&self, locale: &str) -> Option<&str> {
        let field_name = format!("Name[{}]", locale);
        self.fields.get(&field_name).map(|v| v.as_str())
    }

    pub fn mime_types(&self) -> &Vec<MimeType> {
        &self.mime_types
    }

    pub fn can_open_mime_type(&self, mime_type: &MimeType) -> bool {
        for mt in self.mime_types.iter() {
            if mt == mime_type {
                return true;
            }
        }
        false
    }

    pub fn icon(&self) -> Option<&str> {
        self.fields.get("Icon").map(|v| v.as_str())
    }

    pub fn executable_command(&self) -> Option<&str> {
        self.fields.get("Exec").map(|v| v.as_str())
    }

    /// Return the full path to the executable launched by executable_command(), or
    /// an error if the executable is missing, or exists, but is not executable.
    pub fn executable(&self) -> anyhow::Result<PathBuf> {
        if let Some(executable) = self.executable_command() {
            let executable = if let Some(first_space_idx) = executable.find(' ') {
                &executable[0..first_space_idx]
            } else {
                executable
            };

            let executable_path = if executable.contains('/') {
                PathBuf::from(executable)
            } else {
                which::which(executable)?
            };

            if !executable_path.exists() {
                anyhow::bail!("Executable at \"{:?}\" is missing", executable_path);
            }

            if !executable_path.is_executable() {
                anyhow::bail!("Executable at \"{:?}\" is not executable", executable_path);
            } else {
                Ok(executable_path)
            }
        } else {
            anyhow::bail!(
                "No executable command specified for desktop entry at \"{:?}\"",
                self.path
            );
        }
    }

    pub fn entry_type(&self) -> Option<DesktopEntryType> {
        self.fields.get("Type").map(|t| DesktopEntryType::parse(t))
    }

    /// Returns true if this appears to be a valid desktop entry,
    /// e.g., has Name/Type/Exec/Icon fields, and the exec refers
    /// to some kind of executable
    pub fn appears_valid_application(&self) -> bool {
        self.name().is_some()
            && self.entry_type() == Some(DesktopEntryType::Application)
            && self.executable().is_ok()
    }
}

impl Ord for DesktopEntry {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.id.cmp(&other.id)
    }
}

impl PartialOrd for DesktopEntry {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_valid_desktop_entry_ids() {
        assert!(DesktopEntryId::parse("org.foo.Bar.desktop").is_ok());
        assert!(DesktopEntryId::parse("Baz.desktop").is_ok());
    }

    #[test]
    fn rejects_invalid_desktop_entry_ids() {
        assert!(DesktopEntryId::parse("org.foo.Bar").is_err());
        assert!(DesktopEntryId::parse("Baz").is_err());
    }

    #[test]
    fn parses_desktop_entry_id() -> anyhow::Result<()> {
        assert_eq!(
            DesktopEntryId::parse("org.foo.Bar.desktop")?.id(),
            "org.foo.Bar.desktop"
        );
        assert_eq!(DesktopEntryId::parse("Baz.desktop")?.id(), "Baz.desktop");

        Ok(())
    }
}
