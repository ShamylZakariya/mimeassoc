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
        Ok(Self {
            mime_associations_store: MimeAssociationStore::load(&mimeapps_lists_paths()?)?,
            desktop_entry_store: DesktopEntryStore::load(&desktop_entry_dirs()?)?,
            mime_info_store: MimeTypeInfoStore::load(&mimeinfo_paths()?)?,
        })
    }

    pub fn assign_application_to_mimetype(
        &mut self,
        desktop_entry_id: &DesktopEntryId,
        mime_type: &MimeType,
    ) -> anyhow::Result<()> {
        let Some(desktop_entry) = self.desktop_entry_store.get_desktop_entry(&desktop_entry_id) else {
            anyhow::bail!("Unrecognized desktop entry id")
        };

        self.mime_associations_store
            .set_default_handler_for_mime_type(mime_type, desktop_entry)
    }

    pub fn reload_mime_associations(&mut self) -> anyhow::Result<()> {
        self.mime_associations_store.reload()
    }

    pub fn reset_user_default_application_assignments(&mut self) -> anyhow::Result<()> {
        self.mime_associations_store.clear_assigned_applications()
    }

    pub fn prune_orphaned_application_assignments(
        &mut self,
    ) -> anyhow::Result<Vec<DesktopEntryId>> {
        self.mime_associations_store
            .prune_orphaned_application_assignments(&self.desktop_entry_store)
    }

    pub fn save(&mut self) -> anyhow::Result<()> {
        self.mime_associations_store.save()
    }
}
