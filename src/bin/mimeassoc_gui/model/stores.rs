use mimeassoc::*;

enum HistoryEntry {
    DesktopEntryAssignment {
        mime_type: MimeType,
        previous_desktop_entry_id: Option<DesktopEntryId>,
        new_desktop_entry_id: DesktopEntryId,
    },
}

pub struct Stores {
    pub mime_associations_store: MimeAssociationStore,
    pub desktop_entry_store: DesktopEntryStore,
    pub mime_info_store: MimeTypeInfoStore,

    history: Vec<HistoryEntry>,
}

impl std::fmt::Debug for Stores {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // we need a better Debug print, but it depends on having it for mime_associations_store and desktop_entry_store, etc
        f.write_str("[Components]")
    }
}

impl Stores {
    pub fn new() -> anyhow::Result<Self> {
        Ok(Self {
            mime_associations_store: MimeAssociationStore::load(&mimeapps_lists_paths()?)?,
            desktop_entry_store: DesktopEntryStore::load(&desktop_entry_dirs()?)?,
            mime_info_store: MimeTypeInfoStore::load(&mimeinfo_paths()?)?,
            history: vec![],
        })
    }

    pub fn assign_application_to_mimetype(
        &mut self,
        desktop_entry_id: &DesktopEntryId,
        mime_type: &MimeType,
    ) -> anyhow::Result<()> {
        let previous_default_handler = self
            .mime_associations_store
            .default_application_for(mime_type)
            .cloned();

        self.assign_application_to_mimetype_no_history(desktop_entry_id, mime_type)?;

        self.history.push(HistoryEntry::DesktopEntryAssignment {
            mime_type: mime_type.clone(),
            previous_desktop_entry_id: previous_default_handler,
            new_desktop_entry_id: desktop_entry_id.clone(),
        });

        Ok(())
    }

    fn assign_application_to_mimetype_no_history(
        &mut self,
        desktop_entry_id: &DesktopEntryId,
        mime_type: &MimeType,
    ) -> anyhow::Result<()> {
        let Some(desktop_entry) = self.desktop_entry_store.get_desktop_entry(desktop_entry_id) else {
            anyhow::bail!("Unrecognized desktop entry id")
        };

        self.mime_associations_store
            .set_default_handler_for_mime_type(mime_type, desktop_entry)?;

        Ok(())
    }

    pub fn discard_uncommitted_changes(&mut self) -> anyhow::Result<()> {
        self.mime_associations_store.reload()?;

        // TODO: Push a copy of the old MimeAssociationStore's user scope into history???
        self.history.clear();

        Ok(())
    }

    pub fn reset_user_default_application_assignments(&mut self) -> anyhow::Result<()> {
        self.mime_associations_store.clear_assigned_applications()?;

        // TODO: Push a copy of the old MimeAssociationStore's user scope into history???
        self.history.clear();

        Ok(())
    }

    pub fn prune_orphaned_application_assignments(
        &mut self,
    ) -> anyhow::Result<Vec<DesktopEntryId>> {
        let result = self
            .mime_associations_store
            .prune_orphaned_application_assignments(&self.desktop_entry_store)?;

        // TODO: Push a copy of the old MimeAssociationStore's user scope into history???
        self.history.clear();

        Ok(result)
    }

    pub fn save(&mut self) -> anyhow::Result<()> {
        self.mime_associations_store.save()
    }

    pub fn is_dirty(&self) -> bool {
        self.mime_associations_store.is_dirty()
    }

    pub fn undo(&mut self) -> anyhow::Result<()> {
        if let Some(entry) = self.history.pop() {
            match entry {
                HistoryEntry::DesktopEntryAssignment {
                    mime_type,
                    previous_desktop_entry_id,
                    ..
                } => {
                    if let Some(previous_desktop_entry_id) = previous_desktop_entry_id {
                        self.assign_application_to_mimetype_no_history(
                            &previous_desktop_entry_id,
                            &mime_type,
                        )?;
                    } else {
                        self.mime_associations_store
                            .remove_assigned_applications_for(&mime_type)?;
                    }
                }
            }
        }

        Ok(())
    }
}
