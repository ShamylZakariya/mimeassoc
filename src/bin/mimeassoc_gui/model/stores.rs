use std::fmt::Debug;

use mimeassoc::*;

#[allow(dead_code)]
enum HistoryEntry {
    DesktopEntryAssignment {
        mime_type: MimeType,
        previous_desktop_entry_id: Option<DesktopEntryId>,
        new_desktop_entry_id: DesktopEntryId,
    },
    DesktopEntryUnassignment {
        mime_type: MimeType,
        previous_desktop_entry_id: Option<DesktopEntryId>,
    },
    DiscardUncommittedChanges {
        previous_user_scope: MimeAssociationScope,
    },
    ResetUserDefaultApplicationAssignments {
        previous_user_scope: MimeAssociationScope,
    },
    PruneOrphanedApplicationAssignments {
        previous_user_scope: MimeAssociationScope,
    },
}

impl Debug for HistoryEntry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::DesktopEntryAssignment {
                mime_type,
                previous_desktop_entry_id,
                new_desktop_entry_id,
            } => f
                .debug_struct("DesktopEntryAssignment")
                .field("mime_type", mime_type)
                .field("previous_desktop_entry_id", previous_desktop_entry_id)
                .field("new_desktop_entry_id", new_desktop_entry_id)
                .finish(),
            Self::DesktopEntryUnassignment {
                mime_type,
                previous_desktop_entry_id,
            } => f
                .debug_struct("DesktopEntryUnassignment")
                .field("mime_type", mime_type)
                .field("previous_desktop_entry_id", previous_desktop_entry_id)
                .finish(),
            Self::DiscardUncommittedChanges {
                previous_user_scope: _,
            } => f
                .debug_struct("DiscardUncommittedChanges")
                // .field("previous_user_scope", previous_user_scope)
                .finish(),
            Self::ResetUserDefaultApplicationAssignments {
                previous_user_scope: _,
            } => f
                .debug_struct("ResetUserDefaultApplicationAssignments")
                // .field("previous_user_scope", previous_user_scope)
                .finish(),
            Self::PruneOrphanedApplicationAssignments {
                previous_user_scope: _,
            } => f
                .debug_struct("PruneOrphanedApplicationAssignments")
                // .field("previous_user_scope", previous_user_scope)
                .finish(),
        }
    }
}

pub struct Stores {
    mime_associations_store: MimeAssociationStore,
    desktop_entry_store: DesktopEntryStore,
    mime_info_store: MimeInfoStore,

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
            mime_info_store: MimeInfoStore::load(&mimeinfo_paths()?)?,
            history: vec![],
        })
    }

    pub fn mime_associations_store(&self) -> &MimeAssociationStore {
        &self.mime_associations_store
    }

    pub fn desktop_entry_store(&self) -> &DesktopEntryStore {
        &self.desktop_entry_store
    }

    pub fn mime_info_store(&self) -> &MimeInfoStore {
        &self.mime_info_store
    }

    pub fn assign_application_to_mimetype(
        &mut self,
        mime_type: &MimeType,
        desktop_entry_id: &DesktopEntryId,
    ) -> anyhow::Result<()> {
        let previous_assigned_handler = self
            .mime_associations_store
            .assigned_application_for(mime_type)
            .cloned();

        let Some(desktop_entry) = self.desktop_entry_store.get_desktop_entry(desktop_entry_id) else {
            anyhow::bail!("Unrecognized desktop entry id")
        };

        self.mime_associations_store
            .set_default_handler_for_mime_type(mime_type, desktop_entry)?;

        self.history.push(HistoryEntry::DesktopEntryAssignment {
            mime_type: mime_type.clone(),
            previous_desktop_entry_id: previous_assigned_handler,
            new_desktop_entry_id: desktop_entry_id.clone(),
        });

        Ok(())
    }

    fn assign_application_to_mimetype_without_history(
        &mut self,
        mime_type: &MimeType,
        previous_desktop_entry: &DesktopEntryId,
    ) -> anyhow::Result<()> {
        let Some(desktop_entry) = self.desktop_entry_store.get_desktop_entry(previous_desktop_entry) else {
            anyhow::bail!("Unrecognized desktop entry id")
        };

        self.mime_associations_store
            .set_default_handler_for_mime_type(mime_type, desktop_entry)?;

        Ok(())
    }

    pub fn remove_assigned_application_from_mimetype(
        &mut self,
        mime_type: &MimeType,
    ) -> anyhow::Result<()> {
        let previous_assigned_handler = self
            .mime_associations_store
            .assigned_application_for(mime_type)
            .cloned();

        self.mime_associations_store
            .remove_assigned_applications_for(mime_type)?;

        self.history.push(HistoryEntry::DesktopEntryUnassignment {
            mime_type: mime_type.clone(),
            previous_desktop_entry_id: previous_assigned_handler,
        });

        Ok(())
    }

    pub fn discard_uncommitted_changes(&mut self) -> anyhow::Result<()> {
        if let Some(user_scope) = self.mime_associations_store.get_user_scope().cloned() {
            self.history.push(HistoryEntry::DiscardUncommittedChanges {
                previous_user_scope: user_scope,
            });
        }

        // attempt to reload; if there's an error pop the change, which will re-assign the user scope state
        if let Err(e) = self.mime_associations_store.reload() {
            self.undo()?;
            Err(e)
        } else {
            Ok(())
        }
    }

    pub fn reset_user_default_application_assignments(&mut self) -> anyhow::Result<()> {
        if let Some(user_scope) = self.mime_associations_store.get_user_scope().cloned() {
            self.history
                .push(HistoryEntry::ResetUserDefaultApplicationAssignments {
                    previous_user_scope: user_scope,
                });
        }

        // attempt to clear; if there's an error pop the change, which will re-assign the user scope state
        if let Err(e) = self.mime_associations_store.clear_assigned_applications() {
            self.undo()?;
            Err(e)
        } else {
            Ok(())
        }
    }

    pub fn prune_orphaned_application_assignments(
        &mut self,
    ) -> anyhow::Result<Vec<DesktopEntryId>> {
        if let Some(user_scope) = self.mime_associations_store.get_user_scope().cloned() {
            self.history
                .push(HistoryEntry::PruneOrphanedApplicationAssignments {
                    previous_user_scope: user_scope,
                });
        }

        match self
            .mime_associations_store
            .prune_orphaned_application_assignments(&self.desktop_entry_store)
        {
            Ok(result) => Ok(result),
            Err(e) => {
                // the pruning failed; restore previous user scope
                self.undo()?;
                Err(e)
            }
        }
    }

    pub fn save(&mut self) -> anyhow::Result<()> {
        self.mime_associations_store.save()
    }

    pub fn can_undo(&self) -> bool {
        !self.history.is_empty()
    }

    pub fn is_dirty(&self) -> bool {
        self.mime_associations_store.is_dirty()
    }

    pub fn undo(&mut self) -> anyhow::Result<()> {
        // pop one off the history stack, and apply the reversing change
        if let Some(entry) = self.history.pop() {
            match entry {
                HistoryEntry::DesktopEntryAssignment {
                    mime_type,
                    previous_desktop_entry_id,
                    ..
                } => {
                    if let Some(previous_desktop_entry_id) = previous_desktop_entry_id {
                        self.assign_application_to_mimetype_without_history(
                            &mime_type,
                            &previous_desktop_entry_id,
                        )?;
                    } else {
                        self.mime_associations_store
                            .remove_assigned_applications_for(&mime_type)?;
                    }
                }
                HistoryEntry::DesktopEntryUnassignment {
                    mime_type,
                    previous_desktop_entry_id,
                } => {
                    if let Some(previous_desktop_entry_id) = previous_desktop_entry_id {
                        self.assign_application_to_mimetype_without_history(
                            &mime_type,
                            &previous_desktop_entry_id,
                        )?;
                    } else {
                        self.mime_associations_store
                            .remove_assigned_applications_for(&mime_type)?;
                    }
                }

                HistoryEntry::DiscardUncommittedChanges {
                    previous_user_scope: user_scope,
                } => {
                    self.mime_associations_store
                        .overwrite_user_scope(&user_scope)?;
                }
                HistoryEntry::ResetUserDefaultApplicationAssignments {
                    previous_user_scope: user_scope,
                } => {
                    self.mime_associations_store
                        .overwrite_user_scope(&user_scope)?;
                }
                HistoryEntry::PruneOrphanedApplicationAssignments {
                    previous_user_scope: user_scope,
                } => {
                    self.mime_associations_store
                        .overwrite_user_scope(&user_scope)?;
                }
            }
        }

        Ok(())
    }

    pub fn debug_log_history_stack(&self) {
        log::debug!("\nhistory:\n{:#?}\n", self.history);
    }
}
