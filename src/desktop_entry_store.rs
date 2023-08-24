use std::{
    collections::HashMap,
    ops::Deref,
    path::{Path, PathBuf},
};

use super::{has_extension, mime_type::MimeType};
use super::{DesktopEntry, DesktopEntryId, DesktopEntryType};

/// Represents all the desktop entries in a particular scope, or specifically,
/// a location on the filesystem such as /usr/share/applications
struct DesktopEntryScope {
    directory: PathBuf,
    application_entries: HashMap<DesktopEntryId, DesktopEntry>,
}

impl DesktopEntryScope {
    fn load<P>(dir: P) -> anyhow::Result<Self>
    where
        P: AsRef<Path>,
    {
        let directory = dir.as_ref();
        log::info!("DesktopEntryScope::load {:?}", directory);

        let mut application_entries = HashMap::new();
        let contents = std::fs::read_dir(directory)?;
        for file in contents.flatten() {
            let file_path = file.path();
            if has_extension(&file_path, "desktop") {
                if let Ok(desktop_entry) = DesktopEntry::load(&file_path) {
                    if let Some(DesktopEntryType::Application) = desktop_entry.entry_type() {
                        application_entries
                            .insert(desktop_entry.id().clone(), desktop_entry.clone());
                    }
                }
            }
        }

        Ok(Self {
            directory: PathBuf::from(&directory),
            application_entries,
        })
    }

    fn application_entry(&self, id: &DesktopEntryId) -> Option<&DesktopEntry> {
        self.application_entries.get(id)
    }
}

pub struct DesktopEntryStore {
    scopes: Vec<DesktopEntryScope>,
}

impl DesktopEntryStore {
    /// Load desktop entries from the directory paths provided, with desktop entries in
    /// earlier dirs overriding those in later.
    pub fn load<P>(scope_paths: &[P]) -> anyhow::Result<Self>
    where
        P: AsRef<Path>,
    {
        let mut scopes = Vec::new();
        for path in scope_paths {
            scopes.push(DesktopEntryScope::load(path)?);
        }

        Ok(Self { scopes })
    }

    /// Return the directories used to populate each scope in this store, in preferential chain order
    pub fn sources(&self) -> Vec<&Path> {
        self.scopes.iter().map(|s| s.directory.deref()).collect()
    }

    /// Returns all desktop entries, with earlier scopes overriding later ones.
    /// E.g., if a user has a desktop entry which overrides a system one, the user
    /// one will override the system one
    pub fn desktop_entries(&self) -> Vec<&DesktopEntry> {
        let mut table: HashMap<&DesktopEntryId, &DesktopEntry> = HashMap::new();

        for scope in self.scopes.iter().rev() {
            for application in scope.application_entries.iter() {
                table.insert(application.0, application.1);
            }
        }

        let mut values = table.values().copied().collect::<Vec<_>>();
        values.sort();
        values
    }

    /// Return all unique desktop entry identifiers.
    pub fn desktop_entry_ids(&self) -> Vec<&DesktopEntryId> {
        self.desktop_entries()
            .iter()
            .map(|de| de.id())
            .collect::<Vec<_>>()
    }

    /// Lookup the DesktopEntry with the specified identifier, returning the one
    /// earliest in the list provided at construction time, e.g., with user entries
    /// overriding system.
    pub fn get_desktop_entry(&self, id: &DesktopEntryId) -> Option<&DesktopEntry> {
        for scope in self.scopes.iter() {
            if let Some(desktop_entry) = scope.application_entry(id) {
                return Some(desktop_entry);
            }
        }

        None
    }

    /// Look up the desktop entries which can open a specific mimetype
    pub fn get_desktop_entries_for_mimetype(&self, mime_type: &MimeType) -> Vec<&DesktopEntry> {
        let mut entries = vec![];
        for desktop_entry in self.desktop_entries() {
            if desktop_entry.can_open_mime_type(mime_type) {
                entries.push(desktop_entry);
            }
        }

        entries
    }
}

/// Covnenience function to "fuzzily" search for a DesktopEntry by name or id.
/// First, if `identifier` is a valid DesktopEntryId and in `entries`, returns it. CASE-SENSITIVE.
/// Second, if `identifier` can be turned into a DesktopEntryId by appending `.desktop`, and is in `entries`, returns it. CASE-SENSITIVE.
/// Third, performs a CASE-INSENSITIVE search for the first DesktopEntry with a matching name.
/// Finally, attempts to CASE-INSENSITIVE match the name to the id of each registered DesktopEntry, such that, e.g.,
/// org.gnome.Evince.desktop` would match "Evince", by using the token preceding ".desktop".
pub fn lookup_desktop_entry<'a>(
    entries: &'a DesktopEntryStore,
    identifier: &str,
) -> Option<&'a DesktopEntry> {
    if let Ok(desktop_entry_id) = DesktopEntryId::parse(identifier) {
        return entries.get_desktop_entry(&desktop_entry_id);
    }

    if !identifier.ends_with(".desktop") {
        if let Ok(desktop_entry_id) = DesktopEntryId::parse(&format!("{}.desktop", identifier)) {
            let desktop_entry = entries.get_desktop_entry(&desktop_entry_id);
            if desktop_entry.is_some() {
                return desktop_entry;
            }
        }
    }

    let desktop_entries = entries.desktop_entries();
    for entry in desktop_entries.iter() {
        if let Some(entry_name) = entry.name() {
            if entry_name.eq_ignore_ascii_case(identifier) {
                return Some(entry);
            }
        }
    }

    for desktop_entry in desktop_entries {
        let components = desktop_entry.id().id().split('.').collect::<Vec<_>>();
        let count = components.len();
        if count >= 2 && components[count - 2].eq_ignore_ascii_case(identifier) {
            return Some(desktop_entry);
        }
    }

    None
}

/////////////////////////////////////////////////////////////////////

#[cfg(test)]
mod tests {
    use super::*;

    fn path(p: &str) -> PathBuf {
        let cwd = std::env::current_dir().unwrap();
        cwd.join(p)
    }

    fn test_user_applications() -> PathBuf {
        path("test-data/local/share/applications")
    }

    fn test_sys_applications() -> PathBuf {
        path("test-data/usr/share/applications")
    }

    #[test]
    fn desktop_entry_scope_loads_desktop_entries() -> anyhow::Result<()> {
        let sys_scope = DesktopEntryScope::load(test_sys_applications())?;
        assert!(!sys_scope.application_entries.is_empty());

        let user_scope = DesktopEntryScope::load(test_user_applications())?;
        assert!(!user_scope.application_entries.is_empty());

        Ok(())
    }

    #[test]
    fn desktop_entries_loads_single_scopes() -> anyhow::Result<()> {
        assert!(!DesktopEntryStore::load(&[test_sys_applications()])?
            .desktop_entries()
            .is_empty());
        assert!(!DesktopEntryStore::load(&[test_user_applications()])?
            .desktop_entries()
            .is_empty());

        Ok(())
    }

    #[test]
    fn desktop_entries_loads_multiple_scopes() -> anyhow::Result<()> {
        assert!(
            !DesktopEntryStore::load(&[test_user_applications(), test_sys_applications()])?
                .desktop_entries()
                .is_empty()
        );

        Ok(())
    }

    #[test]
    fn desktop_entries_later_scopes_shadow_earlier() -> anyhow::Result<()> {
        let entries =
            DesktopEntryStore::load(&[test_user_applications(), test_sys_applications()])?;

        let weather_id = DesktopEntryId::parse("org.gnome.Weather.desktop")?;
        let weather = entries.get_desktop_entry(&weather_id);
        assert!(weather.is_some());
        let weather = weather.unwrap();
        assert_eq!(weather.icon(), Some("OverriddenWeatherIconId"));

        Ok(())
    }

    #[test]
    fn desktop_entries_looks_up_correct_openers_for_mime_type() -> anyhow::Result<()> {
        let entries =
            DesktopEntryStore::load(&[test_user_applications(), test_sys_applications()])?;

        let photopea_id = DesktopEntryId::parse("photopea.desktop")?;
        let photopea = entries.get_desktop_entry(&photopea_id).unwrap();

        let image_bmp = MimeType::parse("image/bmp")?;
        let image_jpeg = MimeType::parse("image/jpeg")?;
        assert!(entries
            .get_desktop_entries_for_mimetype(&image_bmp)
            .contains(&photopea));
        assert!(entries
            .get_desktop_entries_for_mimetype(&image_jpeg)
            .contains(&photopea));

        // evince comes from sys applications, and it ALSO handles tiff
        let evince_id = DesktopEntryId::parse("org.gnome.Evince.desktop")?;
        let evince = entries.get_desktop_entry(&evince_id).unwrap();
        let image_tiff = MimeType::parse("image/tiff")?;
        assert!(entries
            .get_desktop_entries_for_mimetype(&image_tiff)
            .contains(&photopea));
        assert!(entries
            .get_desktop_entries_for_mimetype(&image_tiff)
            .contains(&evince));

        let gedit_id = DesktopEntryId::parse("org.gnome.gedit.desktop")?;
        let gedit = entries.get_desktop_entry(&gedit_id).unwrap();
        let texteditor_id = DesktopEntryId::parse("org.gnome.TextEditor.desktop")?;
        let texteditor = entries.get_desktop_entry(&texteditor_id).unwrap();
        let text_plain = MimeType::parse("text/plain")?;
        assert!(entries
            .get_desktop_entries_for_mimetype(&text_plain)
            .contains(&gedit),);
        assert!(entries
            .get_desktop_entries_for_mimetype(&text_plain)
            .contains(&texteditor),);

        Ok(())
    }

    #[test]
    fn fuzzy_lookup_works() -> anyhow::Result<()> {
        let entries =
            DesktopEntryStore::load(&[test_user_applications(), test_sys_applications()])?;

        // reference
        let photopea_id = DesktopEntryId::parse("photopea.desktop")?;
        let photopea = entries.get_desktop_entry(&photopea_id).unwrap();
        let evince_id = DesktopEntryId::parse("org.gnome.Evince.desktop")?;
        let evince = entries.get_desktop_entry(&evince_id).unwrap();

        assert_eq!(
            lookup_desktop_entry(&entries, "photopea.desktop"),
            Some(photopea)
        );
        assert_eq!(lookup_desktop_entry(&entries, "Photopea"), Some(photopea));
        assert_eq!(lookup_desktop_entry(&entries, "pHOToPea"), Some(photopea));

        assert_eq!(
            lookup_desktop_entry(&entries, "org.gnome.Evince.desktop"),
            Some(evince)
        );
        assert_eq!(lookup_desktop_entry(&entries, "Evince"), Some(evince));
        assert_eq!(lookup_desktop_entry(&entries, "evince"), Some(evince));
        assert_eq!(
            lookup_desktop_entry(&entries, "Document Viewer"),
            Some(evince)
        );
        assert_eq!(
            lookup_desktop_entry(&entries, "document VIEWER"),
            Some(evince)
        );

        Ok(())
    }
}
