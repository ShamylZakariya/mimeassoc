use mimeassoc::desktop_entry::*;
use mimeassoc::mime_type::*;
use mimeassoc::*;

pub struct Components {
    pub mime_db: MimeAssociations,
    pub app_db: DesktopEntries,
}

impl std::fmt::Debug for Components {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // we need a better Debug print, but it depends on having it for mime_db and app_db
        f.write_str("[Components]")
    }
}

impl Components {
    pub fn new() -> anyhow::Result<Self> {
        let desktop_entry_dirs = match desktop_entry_dirs() {
            Ok(desktop_entry_dirs) => desktop_entry_dirs,
            Err(e) => panic!("Unable to load desktop_entry_dirs: {:?}", e),
        };

        let mimeapps_lists = match mimeapps_lists_paths() {
            Ok(mimeapps_lists) => mimeapps_lists,
            Err(e) => panic!("Unable to load mimeapps_lists_paths: {:?}", e),
        };

        let mime_db = match MimeAssociations::load(&mimeapps_lists) {
            Ok(mimeassoc) => mimeassoc,
            Err(e) => panic!("Unable to load MimeAssociations: {:?}", e),
        };

        let app_db = match DesktopEntries::load(&desktop_entry_dirs) {
            Ok(desktop_entries) => desktop_entries,
            Err(e) => panic!("Unable to load DesktopEntries: {:?}", e),
        };

        Ok(Self { mime_db, app_db })
    }
}
