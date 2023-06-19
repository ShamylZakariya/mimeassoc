use mimeassoc::desktop_entry::*;
use mimeassoc::mime_info::MimeTypeInfoStore;
use mimeassoc::mime_type::*;
use mimeassoc::*;

pub struct MimeAssocStores {
    pub mime_associations_store: MimeAssociationStore,
    pub desktop_entry_store: DesktopEntryStore,
    pub mime_info_store: MimeTypeInfoStore,
}

impl std::fmt::Debug for MimeAssocStores {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // we need a better Debug print, but it depends on having it for mime_associations_store and desktop_entry_store, etc
        f.write_str("[Components]")
    }
}

impl MimeAssocStores {
    pub fn new() -> anyhow::Result<Self> {
        let desktop_entry_dirs = match desktop_entry_dirs() {
            Ok(desktop_entry_dirs) => desktop_entry_dirs,
            Err(e) => panic!("Unable to load desktop_entry_dirs: {:?}", e),
        };

        let mimeapps_lists = match mimeapps_lists_paths() {
            Ok(mimeapps_lists) => mimeapps_lists,
            Err(e) => panic!("Unable to load mimeapps_lists_paths: {:?}", e),
        };

        let mime_info_paths = match mimeinfo_paths() {
            Ok(paths) => paths,
            Err(e) => panic!("Unable to load mimeinfo_paths: {:?}", e),
        };

        let mime_associations_store = match MimeAssociationStore::load(&mimeapps_lists) {
            Ok(mimeassoc) => mimeassoc,
            Err(e) => panic!("Unable to load Store: {:?}", e),
        };

        let desktop_entry_store = match DesktopEntryStore::load(&desktop_entry_dirs) {
            Ok(desktop_entries) => desktop_entries,
            Err(e) => panic!("Unable to load DesktopEntryStore: {:?}", e),
        };

        let mime_info_store = match MimeTypeInfoStore::load(&mime_info_paths) {
            Ok(mime_info_db) => mime_info_db,
            Err(e) => panic!("Unable to load MimeTypeInfoStore: {:?}", e),
        };

        Ok(Self {
            mime_associations_store,
            desktop_entry_store,
            mime_info_store,
        })
    }
}
