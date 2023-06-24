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
        let mime_associations_store = MimeAssociationStore::load(&mimeapps_lists_paths()?)?;

        let desktop_entry_store = DesktopEntryStore::load(&desktop_entry_dirs()?)?;

        let mime_info_store = MimeTypeInfoStore::load(&mimeinfo_paths()?)?;

        Ok(Self {
            mime_associations_store,
            desktop_entry_store,
            mime_info_store,
        })
    }
}
