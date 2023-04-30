use std::{
    collections::HashMap,
    fs::File,
    io::{self, BufRead},
    path::{Path, PathBuf},
};

use is_executable::IsExecutable;

use super::mime_type::MimeType;

#[derive(Debug, PartialEq, Eq, Hash, Clone)]
pub struct DesktopEntryId {
    desktop_entry: String,
}

impl DesktopEntryId {
    pub fn parse(desktop_entry: &str) -> anyhow::Result<DesktopEntryId> {
        if desktop_entry.ends_with(".desktop") {
            Ok(DesktopEntryId {
                desktop_entry: desktop_entry.to_string(),
            })
        } else {
            anyhow::bail!(
                "id: \"{}\" not a valid Gnome .desktop file name",
                desktop_entry
            )
        }
    }

    pub fn id(&self) -> &str {
        let desktop_idx = self.desktop_entry.find(".desktop").unwrap();
        &self.desktop_entry[0..desktop_idx]
    }
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
enum DesktopEntryType {
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
struct DesktopEntry {
    path: PathBuf,
    fields: HashMap<String, String>,
    mime_types: Vec<MimeType>,
    id: DesktopEntryId,
}

impl DesktopEntry {
    fn load<P>(path: P) -> anyhow::Result<DesktopEntry>
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

    fn name(&self) -> Option<&str> {
        self.fields.get("Name").map(|v| v.as_str())
    }

    fn id(&self) -> &DesktopEntryId {
        &self.id
    }

    fn localised_name(&self, locale: &str) -> Option<&str> {
        let field_name = format!("Name[{}]", locale);
        self.fields.get(&field_name).map(|v| v.as_str())
    }

    fn mime_types(&self) -> &Vec<MimeType> {
        &self.mime_types
    }

    fn icon(&self) -> Option<&str> {
        self.fields.get("Icon").map(|v| v.as_str())
    }

    fn executable_command(&self) -> Option<&str> {
        self.fields.get("Exec").map(|v| v.as_str())
    }

    /// Return the full path to the executable launched by executable_command(), or
    /// an error if the executable is missing, or exists, but is not executable.
    fn executable(&self) -> anyhow::Result<PathBuf> {
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

    fn entry_type(&self) -> Option<DesktopEntryType> {
        self.fields.get("Type").map(|t| DesktopEntryType::parse(t))
    }

    /// Returns true if this appears to be a valid desktop entry,
    /// e.g., has Name/Type/Exec/Icon fields, and the exec refers
    /// to some kind of executable
    fn appears_valid_application(&self) -> bool {
        self.name().is_some()
            && self.entry_type() == Some(DesktopEntryType::Application)
            && self.executable().is_ok()
    }
}

/// Return a vector of paths to the application dirs for the user.
pub fn desktop_entry_dirs() -> anyhow::Result<Vec<PathBuf>> {
    let xdg_data_dirs = std::env::var("XDG_DATA_DIRS")?
        .split(':')
        .map(|d| PathBuf::from(d))
        .collect::<Vec<_>>();
    let user_data_dir = PathBuf::from(std::env::var("HOME")?).join(".local/share");
    let mut data_dirs = vec![user_data_dir];
    data_dirs.extend(xdg_data_dirs);

    Ok(data_dirs
        .iter()
        .filter(|d| d.exists() && d.is_dir())
        .map(|d| d.join("applications"))
        .collect::<Vec<_>>())
}

/// Represents all the desktop entries in a particular scope, or specifically,
/// a location on the filesystem such as /usr/share/applications
struct DesktopEntryScope {
    application_entries: HashMap<DesktopEntryId, DesktopEntry>,
}

impl DesktopEntryScope {
    fn load<P>(dir: P) -> anyhow::Result<Self>
    where
        P: AsRef<Path>,
    {
        let mut application_entries = HashMap::new();
        let contents = std::fs::read_dir(dir)?;
        for file in contents.flatten() {
            let file_path = file.path();
            if let Some(extension) = file_path.extension() {
                let extension = extension.to_str().unwrap();
                if extension == "desktop" {
                    if let Ok(desktop_entry) = DesktopEntry::load(file_path) {
                        match desktop_entry.entry_type() {
                            Some(DesktopEntryType::Application) => {
                                application_entries
                                    .insert(desktop_entry.id().clone(), desktop_entry.clone());
                            }
                            _ => {}
                        }
                    }
                }
            }
        }

        Ok(Self {
            application_entries,
        })
    }
}

///////////////////////////////////////////////////////////////////////////////////////////////////////////////////////

#[cfg(test)]
mod tests {
    use super::*;

    fn path(p: &str) -> PathBuf {
        let cwd = std::env::current_dir().unwrap();
        cwd.join(p)
    }

    fn test_sys_gedit() -> PathBuf {
        path("test-data/usr/share/applications/org.gnome.gedit.desktop")
    }

    fn test_sys_weather() -> PathBuf {
        path("test-data/usr/share/applications/org.gnome.Weather.desktop")
    }

    fn test_user_photopea() -> PathBuf {
        path("test-data/local/share/applications/photopea.desktop")
    }

    fn test_user_invalid() -> PathBuf {
        path("test-data/local/share/applications/invalid.desktop")
    }

    fn test_user_applications() -> PathBuf {
        path("test-data/local/share/applications")
    }

    fn test_sys_applications() -> PathBuf {
        path("test-data/usr/share/applications")
    }

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
            "org.foo.Bar"
        );
        assert_eq!(DesktopEntryId::parse("Baz.desktop")?.id(), "Baz");

        Ok(())
    }

    #[test]
    fn desktop_entry_parses_valid_files() -> anyhow::Result<()> {
        DesktopEntry::load(test_sys_gedit())?;
        DesktopEntry::load(test_sys_weather())?;
        DesktopEntry::load(test_user_photopea())?;

        Ok(())
    }

    #[test]
    fn desktop_entry_rejects_invalid_files() {
        assert!(DesktopEntry::load(test_user_invalid()).is_err());
        assert!(DesktopEntry::load(path("not/a/valid/path/to/a/desktop/file.desktop")).is_err());
    }

    #[test]
    fn desktop_entry_parses_correctly() -> anyhow::Result<()> {
        let gedit = DesktopEntry::load(test_sys_gedit())?;

        assert_eq!(gedit.name(), Some("gedit"));
        assert_eq!(gedit.localised_name("es"), Some("gedit"));
        assert_eq!(gedit.localised_name("pa"), Some("ਜੀ-ਸੰਪਾਦਕ"));
        assert_eq!(gedit.mime_types(), &vec![MimeType::parse("text/plain")?]);
        assert_eq!(gedit.executable_command(), Some("gedit %U"));
        assert_eq!(gedit.icon(), Some("org.gnome.gedit"));
        assert!(gedit.appears_valid_application());
        Ok(())
    }

    #[test]
    fn data_dirs_returns_nonempty_vec() -> anyhow::Result<()> {
        let dirs = desktop_entry_dirs()?;
        assert!(!dirs.is_empty());
        Ok(())
    }

    #[test]
    fn data_dirs_contain_desktop_entries() -> anyhow::Result<()> {
        let mut count = 0;
        for dir in desktop_entry_dirs()? {
            let contents = std::fs::read_dir(dir)?;
            for file in contents.flatten() {
                let file_path = file.path();
                if let Some(extension) = file_path.extension() {
                    let extension = extension.to_str().unwrap();
                    if extension == "desktop" {
                        count += 1;
                    }
                }
            }
        }

        if count > 0 {
            Ok(())
        } else {
            anyhow::bail!("desktop_entry_dirs() had no desktop entries.")
        }
    }

    #[test]
    fn desktop_entry_scope_loads_desktop_entries() -> anyhow::Result<()> {
        let sys_scope = DesktopEntryScope::load(test_sys_applications())?;
        assert!(!sys_scope.application_entries.is_empty());

        let user_scope = DesktopEntryScope::load(test_user_applications())?;
        assert!(!user_scope.application_entries.is_empty());

        Ok(())
    }
}
