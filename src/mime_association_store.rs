use anyhow::bail;
use std::{
    collections::{HashMap, HashSet},
    fs::File,
    io::{self, BufRead, Write},
    ops::Deref,
    path::{Path, PathBuf},
};

use crate::{DesktopEntryStore, MimeType};

use super::desktop_entry::{DesktopEntry, DesktopEntryId};

#[derive(PartialEq, Eq)]
enum MimeAssociationsSections {
    AddedAssociations,
    DefaultApplications,
}

impl MimeAssociationsSections {
    fn try_parse(desc: &str) -> Option<Self> {
        let desc = desc.trim();
        if desc == "[Added Associations]" {
            Some(Self::AddedAssociations)
        } else if desc == "[Default Applications]" {
            Some(Self::DefaultApplications)
        } else {
            None
        }
    }

    fn to_string(&self) -> &'static str {
        match self {
            MimeAssociationsSections::AddedAssociations => "[Added Associations]",
            MimeAssociationsSections::DefaultApplications => "[Default Applications]",
        }
    }
}

#[derive(Default, PartialEq, Eq, Clone)]
pub struct MimeAssociationScope {
    file_path: PathBuf,
    is_user_customizable: bool,
    is_dirty: bool,
    added_associations: HashMap<MimeType, Vec<DesktopEntryId>>,
    default_applications: HashMap<MimeType, DesktopEntryId>,
}

impl MimeAssociationScope {
    fn load<P>(mimeapps_file_path: P) -> anyhow::Result<Self>
    where
        P: AsRef<Path>,
    {
        let mimeapps_file_path = mimeapps_file_path.as_ref();
        log::info!("MimeAssociationScope::load {:?}", mimeapps_file_path);

        let mimeapps_file = File::open(mimeapps_file_path)?;
        let permissions = mimeapps_file.metadata()?.permissions();
        let line_buffer = io::BufReader::new(mimeapps_file).lines();
        let mut added_associations = HashMap::new();
        let mut default_applications = HashMap::new();
        let mut current_section: Option<MimeAssociationsSections> = None;

        for line in line_buffer.flatten() {
            if let Some(section) = MimeAssociationsSections::try_parse(&line) {
                // catch [Section] directives in the list
                current_section = Some(section);
            } else if let Some(current_section) = &current_section {
                // if we have a current section, we can add associations to it.
                let trimmed_line = line.trim();

                if let Ok((mime_type, id)) = Self::parse_line(trimmed_line) {
                    match current_section {
                        MimeAssociationsSections::AddedAssociations => {
                            added_associations.insert(mime_type, id);
                        }
                        MimeAssociationsSections::DefaultApplications => {
                            if let Some(id) = id.first() {
                                default_applications.insert(mime_type, id.clone());
                            } else {
                                anyhow::bail!(
                                    "Line \"{}\" specified 0 DesktopEntryIds",
                                    trimmed_line
                                );
                            }
                        }
                    };
                } else if !trimmed_line.starts_with('#') && !trimmed_line.is_empty() {
                    // this line is not a section directive, MimeAssociation, or comment
                    anyhow::bail!(
                        "Unable to parse MimeAssociation from line: \"{}\"",
                        trimmed_line
                    );
                }
            }
        }

        // This file is user customizable iff it's in the user's dir and writable
        let home_dir = PathBuf::from(std::env::var("HOME")?);
        let is_user_customizable =
            mimeapps_file_path.starts_with(home_dir) && !permissions.readonly();

        Ok(MimeAssociationScope {
            file_path: PathBuf::from(mimeapps_file_path),
            is_user_customizable,
            is_dirty: false,
            added_associations,
            default_applications,
        })
    }

    fn reload(&mut self) -> anyhow::Result<()> {
        // Caveman reload: make a new object, move its values to self
        let mut associations = Self::load(&self.file_path)?;
        self.is_user_customizable = associations.is_user_customizable;
        self.is_dirty = false;
        self.added_associations = std::mem::take(&mut associations.added_associations);
        self.default_applications = std::mem::take(&mut associations.default_applications);

        Ok(())
    }

    /// Returns true if this MimeAssociationScope differs from the version on disk it was loaded from
    /// This is similar to the `is_dirty` but different. `is_dirty` is set when mutations have been made,
    /// but will not account for a series of transformations which result in the original state.
    #[allow(dead_code)]
    fn differs_from_file_representation(&self) -> bool {
        if let Ok(file_representation) = Self::load(&self.file_path) {
            self.default_applications != file_representation.default_applications
                || self.added_associations != file_representation.added_associations
        } else {
            true
        }
    }

    /// Persist changes to this MimeAsociationScope.
    fn save(&mut self) -> anyhow::Result<()> {
        if !self.is_user_customizable {
            anyhow::bail!(
                "MimeAssociationScope[{:?}] is not user customizable.",
                &self.file_path
            );
        }

        if self.is_dirty {
            // create a temp output file
            let temp_dir = self.file_path.parent().unwrap();
            let temp_file_path = temp_dir.join("mimeassoc.temp.list");
            self.write_to_path(&temp_file_path)?;

            // rename this file to our original
            std::fs::rename(&temp_file_path, &self.file_path)?;

            self.is_dirty = false;
        }
        Ok(())
    }

    fn parse_line(line: &str) -> anyhow::Result<(MimeType, Vec<DesktopEntryId>)> {
        let components = line.split('=').collect::<Vec<_>>();
        if components.len() != 2 {
            anyhow::bail!("A line from mimeapps.lst is expected to be in form \"mime/type=app.desktop\". Line \"{}\" was invalid", line);
        }

        let mime_type_component = components[0].trim();
        let id_components = components[1].trim();

        let mime_type = MimeType::parse(mime_type_component)?;

        if !id_components.contains(';') {
            return Ok((mime_type, vec![DesktopEntryId::parse(id_components)?]));
        }

        let mut ids = Vec::new();
        for id in id_components.split(';') {
            let id = id.trim();
            if !id.is_empty() {
                ids.push(DesktopEntryId::parse(id)?);
            }
        }

        Ok((mime_type, ids))
    }

    fn generate_added_associations_line(
        mime_type: &MimeType,
        desktop_entries: &[DesktopEntryId],
    ) -> String {
        let desktop_entry_strings = desktop_entries
            .iter()
            .map(|d| d.to_string())
            .collect::<Vec<_>>()
            .join(";");
        format!("{}={};", mime_type, desktop_entry_strings)
    }

    fn generate_default_application_line(
        mime_type: &MimeType,
        desktop_entry: &DesktopEntryId,
    ) -> String {
        format!("{}={}", mime_type, desktop_entry)
    }

    fn write(&self, output_file: &mut File) -> anyhow::Result<()> {
        // write the added associations
        if !self.added_associations.is_empty() {
            writeln!(
                output_file,
                "{}",
                MimeAssociationsSections::AddedAssociations.to_string()
            )?;

            let mut mime_types = self.added_associations.keys().collect::<Vec<_>>();
            mime_types.sort();

            for mime_type in mime_types {
                if let Some(desktop_entries) = self.added_associations.get(mime_type) {
                    writeln!(
                        output_file,
                        "{};",
                        Self::generate_added_associations_line(mime_type, desktop_entries)
                    )?;
                }
            }

            writeln!(output_file)?;
        }

        // write the default applications
        if !self.default_applications.is_empty() {
            writeln!(
                output_file,
                "{}",
                MimeAssociationsSections::DefaultApplications.to_string()
            )?;

            let mut mime_types = self.default_applications.keys().collect::<Vec<_>>();
            mime_types.sort();

            for mime_type in mime_types {
                if let Some(desktop_entry) = self.default_applications.get(mime_type) {
                    writeln!(
                        output_file,
                        "{}",
                        Self::generate_default_application_line(mime_type, desktop_entry)
                    )?;
                }
            }
        }

        Ok(())
    }

    fn write_to_path<P>(&self, path: P) -> anyhow::Result<()>
    where
        P: AsRef<Path>,
    {
        let mut output_file = File::create(path)?;
        self.write(&mut output_file)
    }
}

pub struct MimeAssociationStore {
    scopes: Vec<MimeAssociationScope>,
}

impl MimeAssociationStore {
    /// Load MimeAssocations in order of the provided paths. MimeAssocations earlier in
    /// the list will override ones later in the list.
    pub fn load<P>(mimeapps_file_paths: &[P]) -> anyhow::Result<Self>
    where
        P: AsRef<Path>,
    {
        let mut scopes = Vec::new();
        for file_path in mimeapps_file_paths.iter() {
            scopes.push(MimeAssociationScope::load(file_path)?);
        }

        Ok(Self { scopes })
    }

    /// Reload the mime associations passed in to `MimeAssociationStore::load` during construction.
    /// Effectively resets state, provided any changes to state weren't persisted via `MimeAssociationsStore::save`
    pub fn reload(&mut self) -> anyhow::Result<()> {
        for scope in self.scopes.iter_mut() {
            scope.reload()?;
        }

        Ok(())
    }

    /// Return zeroth scope; this is normally associated with the scope
    /// loaded from the user directory.
    /// TODO: Sanitize this - this is brittle. Having the user scope be zero
    /// is fine, as a convention, but easily misconfigured.
    pub fn get_user_scope(&self) -> Option<&MimeAssociationScope> {
        if let Some(scope) = self.scopes.get(0) {
            if scope.is_user_customizable {
                return Some(scope);
            }
        }
        None
    }

    /// Return zeroth scope mutably; this is normally associated with the scope
    /// loaded from the user directory.
    /// TODO: Sanitize this - this is brittle. Having the user scope be zero
    /// is fine, as a convention, but easily misconfigured.
    pub fn get_user_scope_mut(&mut self) -> Option<&mut MimeAssociationScope> {
        if let Some(scope) = self.scopes.get_mut(0) {
            if scope.is_user_customizable {
                return Some(scope);
            }
        }
        None
    }

    /// Overwrite the user scope with the contents of the provided MimeAssociationScope, returning
    /// an error if this store has no user scope.
    pub fn overwrite_user_scope(
        &mut self,
        new_user_scope: &MimeAssociationScope,
    ) -> anyhow::Result<()> {
        if let Some(scope) = self.scopes.get_mut(0) {
            if scope.is_user_customizable {
                scope.added_associations = new_user_scope.added_associations.clone();
                scope.default_applications = new_user_scope.default_applications.clone();

                return Ok(());
            }
        }
        bail!("No user customizable user scope to overwrite")
    }

    /// Return all mimetypes represented, in no particular order.
    pub fn mime_types(&self) -> Vec<&MimeType> {
        let mut mime_types = HashSet::new();
        for scope in self.scopes.iter().rev() {
            for (mime_type, _) in scope.default_applications.iter() {
                mime_types.insert(mime_type);
            }
        }

        mime_types.into_iter().collect()
    }

    /// Return the sources used to create this store, in preferential chain order, e.g., user entries before system.
    pub fn sources(&self) -> Vec<&Path> {
        self.scopes.iter().map(|s| s.file_path.deref()).collect()
    }

    /// Returns the application assigned to the mime type in the user scope;
    /// For disambiguation, this does NOT go up the scope chain to find the system assigned
    /// application.
    pub fn user_assigned_application_for(&self, mime_type: &MimeType) -> Option<&DesktopEntryId> {
        if let Some(scope) = self.get_user_scope() {
            scope.default_applications.get(mime_type)
        } else {
            None
        }
    }

    /// Returns the assigned application to handle a given mime type. This is the application
    /// that will be used by the File manager to open a file.
    pub fn assigned_application_for(&self, mime_type: &MimeType) -> Option<&DesktopEntryId> {
        for scope in self.scopes.iter() {
            if let Some(id) = scope.default_applications.get(mime_type) {
                return Some(id);
            }
        }
        None
    }

    /// Returns the default (e.g., not considering the user's assignment) application to handle a given mime type.
    /// This is not necessarily what would be opened by the file manager; it is what would be used to open
    /// a file if we deleted the user's assignments.
    pub fn system_default_application_for(&self, mime_type: &MimeType) -> Option<&DesktopEntryId> {
        for scope in self.scopes.iter().skip(1) {
            if let Some(id) = scope.default_applications.get(mime_type) {
                return Some(id);
            }
        }
        None
    }

    /// Deletes the application assignment(s) for a given mime type. If the mime_type is a wildcard, will
    /// delete all matching assignments. E.g., image/* would delete assignment for image/png, image/tif, etc.
    /// Note: Changes won't be commited until `MimeAssociationsStore::save` is called.
    /// Returns an error if there is no user-customizable scope to edit.
    pub fn remove_assigned_applications_for(&mut self, mime_type: &MimeType) -> anyhow::Result<()> {
        let Some(scope) = self.get_user_scope_mut() else {
            anyhow::bail!("No customizable user scopes available");
        };

        if mime_type.is_minor_type_wildcard() {
            let mut keys_to_remove = vec![];
            for key in scope.default_applications.keys() {
                if mime_type.wildcard_match(key) {
                    keys_to_remove.push(key.clone());
                }
            }

            for key_to_remove in keys_to_remove {
                scope.default_applications.remove(&key_to_remove);
            }
            scope.is_dirty = true;
        } else if scope.default_applications.remove(mime_type).is_some() {
            scope.is_dirty = true;
        }

        Ok(())
    }

    /// Removes all application assignments in the user scope, effectively resetting the user's
    /// application assignments to system defaults.
    /// Note: Changes won't be commited until `MimeAssociationsStore::save` is called.
    /// Returns an error if there is no user-customizable scope to edit.
    pub fn clear_assigned_applications(&mut self) -> anyhow::Result<()> {
        let Some(scope) = self.get_user_scope_mut() else {
            anyhow::bail!("No customizable user scopes available");
        };

        scope.default_applications.clear();
        scope.is_dirty = true;

        Ok(())
    }

    /// Removes all application assignments in the user scope which
    /// 1: cannot be found in `desktop_entry_store`, or
    /// 2: when loaded to `DesktopEntry`, are invalid (e.g., no launchable binary can be found)
    /// This only affects the user scope.
    /// Note: Changes won't be commited until `MimeAssociationsStore::save` is called.
    /// Returns a vector of pruned DesktopEntryIds, or an error if there is no user-customizable scope to edit.
    pub fn prune_orphaned_application_assignments(
        &mut self,
        desktop_entry_store: &DesktopEntryStore,
    ) -> anyhow::Result<Vec<DesktopEntryId>> {
        let Some(scope) = self.get_user_scope_mut() else {
            anyhow::bail!("No customizable user scopes available");
        };

        let mut orphaned_ids = vec![];
        for desktop_entry_id in scope.default_applications.values() {
            if let Some(desktop_entry) = desktop_entry_store.get_desktop_entry(desktop_entry_id) {
                if !desktop_entry.appears_valid_application() {
                    orphaned_ids.push(desktop_entry_id.clone());
                }
            } else {
                orphaned_ids.push(desktop_entry_id.clone());
            }
        }

        scope
            .default_applications
            .retain(|_, id| !orphaned_ids.contains(id));

        Ok(orphaned_ids)
    }

    pub fn added_associations_for(&self, mime_type: &MimeType) -> Option<&Vec<DesktopEntryId>> {
        for scope in self.scopes.iter() {
            if let Some(id) = scope.added_associations.get(mime_type) {
                return Some(id);
            }
        }
        None
    }

    /// Make the provided DesktopEntry the default handler for the given mime type.
    /// Will return an error if the DesktopEntry isn't a valid application, or if it doesn't
    /// handle the specified mime type, or if there are no user customizable MimeAssociationScopes
    /// in the chain.
    /// Note: If the assigned application is the default application (as specified by the system, minus
    /// user customization) the entry will be removed from the user scope. E.g., this case is equivalent
    /// to calling `delete_assigned_application_for` for the mime type.
    /// Note: Changes won't be commited until `MimeAssociationsStore::save` is called.
    pub fn set_default_handler_for_mime_type(
        &mut self,
        mime_type: &MimeType,
        desktop_entry: &DesktopEntry,
    ) -> anyhow::Result<()> {
        if !desktop_entry.appears_valid_application() {
            anyhow::bail!(
                "DesktopEntry \"{}\" does not appear to be a valid launchable application",
                desktop_entry.id()
            );
        }

        if !desktop_entry.can_open_mime_type(mime_type) {
            anyhow::bail!(
                "DesktopEntry \"{}\" does not support mime type \"{}\"",
                desktop_entry.id(),
                mime_type
            );
        }

        if self.system_default_application_for(mime_type) == Some(desktop_entry.id()) {
            return self.remove_assigned_applications_for(mime_type);
        }

        let Some(scope) = self.get_user_scope_mut() else {
            anyhow::bail!("No customizable user scopes available");
        };

        let new_desktop_entry_id = desktop_entry.id().clone();
        let previous = scope
            .default_applications
            .insert(mime_type.clone(), new_desktop_entry_id.clone());

        if previous != Some(new_desktop_entry_id) {
            scope.is_dirty = true;
        }

        Ok(())
    }

    /// Make the provided DesktopEnrtry the default handler for all its supported mimetypes.
    /// Will return an error if the desktop entry isn't a valid application, or if there are
    /// no user customizable scoped in the MimeAssociationScope chain
    /// Note: Changes won't be commited until `MimeAssociationsStore::save` is called.
    pub fn make_desktop_entry_default_handler_of_its_supported_mime_types(
        &mut self,
        desktop_entry: &DesktopEntry,
    ) -> anyhow::Result<()> {
        for mime_type in desktop_entry.mime_types() {
            self.set_default_handler_for_mime_type(mime_type, desktop_entry)?;
        }
        Ok(())
    }

    /// Returns true if any user customizable scope is dirty
    pub fn is_dirty(&self) -> bool {
        for scope in self.scopes.iter() {
            if scope.is_user_customizable && scope.is_dirty {
                return true;
            }
        }
        false
    }

    /// Commit changes to user customizable scopes. This will write to the user's `mimeapps.list` file.
    pub fn save(&mut self) -> anyhow::Result<()> {
        for scope in self.scopes.iter_mut() {
            if scope.is_user_customizable && scope.is_dirty {
                scope.save()?;
            }
        }

        Ok(())
    }

    /// Find matching mimetypes for a wildcard. If the passed-in mime-type is
    /// not a wildcard, find the first match in storage.
    pub fn find_matching_mimetypes(&self, mime_type: &MimeType) -> Vec<&MimeType> {
        let mut matches = HashSet::new();
        if mime_type.is_minor_type_wildcard() {
            for m in self.mime_types() {
                if mime_type.wildcard_match(m) {
                    matches.insert(m);
                }
            }
        } else {
            for m in self.mime_types() {
                if m == mime_type {
                    matches.insert(m);
                    break;
                }
            }
        }
        matches.into_iter().collect()
    }
}

/////////////////////////////////////////////////////////////////////

#[cfg(test)]
mod tests {
    use super::*;

    fn test_sys_mimeapps_list() -> PathBuf {
        path("test-data/usr/share/applications/mimeapps.list")
    }

    fn test_gnome_mimeapps_list() -> PathBuf {
        path("test-data/usr/share/applications/gnome-mimeapps.list")
    }

    fn test_user_mimeapps_list() -> PathBuf {
        path("test-data/config/mimeapps.list")
    }

    fn test_user_applications() -> PathBuf {
        path("test-data/local/share/applications")
    }

    fn test_sys_applications() -> PathBuf {
        path("test-data/usr/share/applications")
    }

    fn path(p: &str) -> PathBuf {
        let cwd = std::env::current_dir().unwrap();
        cwd.join(p)
    }

    fn delete_file<P>(path: P)
    where
        P: AsRef<Path>,
    {
        let path = path.as_ref();
        if path.exists() {
            let _ = std::fs::remove_file(&path);
        }
    }

    /// Creates a MimeAssociationStore with the first scope user editable, the others not
    fn create_test_associations() -> anyhow::Result<MimeAssociationStore> {
        let mut associations = MimeAssociationStore::load(&[
            test_user_mimeapps_list(),
            test_gnome_mimeapps_list(),
            test_sys_mimeapps_list(),
        ])?;

        // we need to make first scope user writable for testing
        associations.scopes[0].is_user_customizable = true;
        associations.scopes[1].is_user_customizable = false;
        associations.scopes[2].is_user_customizable = false;

        Ok(associations)
    }

    /// Creates a DesktopEntryStore, and MimeAssociationStore with the first scope user editable, the others not
    fn create_test_entries_and_associations(
    ) -> anyhow::Result<(DesktopEntryStore, MimeAssociationStore)> {
        let entries =
            DesktopEntryStore::load(&[test_user_applications(), test_sys_applications()])?;

        let associations = create_test_associations()?;
        Ok((entries, associations))
    }

    #[test]
    fn mime_associations_load() {
        assert!(MimeAssociationScope::load(test_sys_mimeapps_list()).is_ok());
        assert!(MimeAssociationScope::load(test_gnome_mimeapps_list()).is_ok());
        assert!(MimeAssociationScope::load(test_user_mimeapps_list()).is_ok());
    }

    #[test]
    fn mime_associations_load_expected_data() -> anyhow::Result<()> {
        let associations = MimeAssociationScope::load(test_user_mimeapps_list())?;

        let png = MimeType::parse("image/png")?;
        let gimp = DesktopEntryId::parse("org.gimp.GIMP.desktop")?;
        assert_eq!(&associations.added_associations[&png], &vec![gimp]);

        Ok(())
    }

    #[test]
    fn mime_associations_line_parser() -> anyhow::Result<()> {
        let baz_desktop = DesktopEntryId::parse("baz.desktop")?;
        let qux_desktop = DesktopEntryId::parse("qux.desktop")?;
        let zim_desktop = DesktopEntryId::parse("zim.desktop")?;

        // single value with trailing semicolon
        let result = MimeAssociationScope::parse_line("foo/bar=baz.desktop;")?;
        assert_eq!(result.0, MimeType::parse("foo/bar")?);
        assert_eq!(result.1, vec![baz_desktop.clone()]);

        // whitespace
        let result = MimeAssociationScope::parse_line("   foo/bar=baz.desktop\n  ")?;
        assert_eq!(result.0, MimeType::parse("foo/bar")?);
        assert_eq!(result.1, vec![baz_desktop.clone()]);

        // multiple values
        let result =
            MimeAssociationScope::parse_line("foo/bar=baz.desktop;qux.desktop;zim.desktop;")?;
        assert_eq!(result.0, MimeType::parse("foo/bar")?);
        assert_eq!(result.1, vec![baz_desktop, qux_desktop, zim_desktop]);

        assert!(MimeAssociationScope::parse_line("foo/bar=baz").is_err());
        assert!(MimeAssociationScope::parse_line("foobar=baz.desktop;").is_err());

        Ok(())
    }

    #[test]
    fn mime_assocations_loads() -> anyhow::Result<()> {
        let _ = create_test_associations()?;
        Ok(())
    }

    #[test]
    fn assigned_application_prefers_user_default_application_over_system_associations(
    ) -> anyhow::Result<()> {
        let associations = create_test_associations()?;
        let html = MimeType::parse("text/html")?;
        let firefox_id = DesktopEntryId::parse("org.mozilla.firefox.desktop")?;
        assert_eq!(
            associations.assigned_application_for(&html),
            Some(&firefox_id)
        );

        Ok(())
    }

    #[test]
    fn default_application_skips_user_associations() -> anyhow::Result<()> {
        let associations = create_test_associations()?;

        let image_bmp = MimeType::parse("image/bmp")?;
        let eog_id = DesktopEntryId::parse("org.gnome.eog.desktop")?;
        assert_eq!(
            associations.system_default_application_for(&image_bmp),
            Some(&eog_id)
        );

        Ok(())
    }

    #[test]
    fn reload_works() -> anyhow::Result<()> {
        let (entries, mut associations) = create_test_entries_and_associations()?;

        // we need to make first scope user writable for testing
        associations.scopes[0].is_user_customizable = true;

        // we need to make first scope user writable for testing
        associations.scopes[0].is_user_customizable = true;

        let photopea_id = DesktopEntryId::parse("photopea.desktop")?;
        let photopea = entries.get_desktop_entry(&photopea_id).unwrap();
        let evince_id = DesktopEntryId::parse("org.gnome.Evince.desktop")?;
        let image_tiff = MimeType::parse("image/tiff")?;

        // we're going to verify Evince is set to image/tiff
        assert_eq!(
            associations.assigned_application_for(&image_tiff),
            Some(&evince_id)
        );

        // assign photopea
        associations.set_default_handler_for_mime_type(&image_tiff, &photopea)?;
        assert_eq!(
            associations.assigned_application_for(&image_tiff),
            Some(&photopea_id)
        );

        // reload - after this, Evince should be the handler again
        associations.reload()?;

        assert_eq!(
            associations.assigned_application_for(&image_tiff),
            Some(&evince_id)
        );

        Ok(())
    }

    #[test]
    fn remove_assigned_application_works() -> anyhow::Result<()> {
        let mut associations = create_test_associations()?;

        let image_bmp = MimeType::parse("image/bmp")?;
        let eog_id = DesktopEntryId::parse("org.gnome.eog.desktop")?;
        let photopea_id = DesktopEntryId::parse("photopea.desktop")?;

        assert_eq!(
            associations.assigned_application_for(&image_bmp),
            Some(&photopea_id)
        );

        associations.remove_assigned_applications_for(&image_bmp)?;

        assert_eq!(
            associations.assigned_application_for(&image_bmp),
            Some(&eog_id)
        );

        Ok(())
    }

    #[test]
    fn remove_all_assigned_applications_works() -> anyhow::Result<()> {
        let mut associations = create_test_associations()?;

        // initial state: we should have default applications assigned
        assert!(!associations.scopes[0].default_applications.is_empty());

        // clear
        associations
            .clear_assigned_applications()
            .expect("Expect to clear all assigned applications");

        // post state: no default applications and scope is dirty
        assert!(associations.scopes[0].default_applications.is_empty());
        assert!(associations.scopes[0].is_dirty);

        Ok(())
    }

    #[test]
    fn non_mutating_assignment_doesnt_dirty_store() -> anyhow::Result<()> {
        let (desktop_entry_store, mut associations) = create_test_entries_and_associations()?;

        let image_bmp = MimeType::parse("image/bmp")?;
        let photopea_id = DesktopEntryId::parse("photopea.desktop")?;

        assert_eq!(
            associations.assigned_application_for(&image_bmp),
            Some(&photopea_id)
        );

        // photopea is already assigned to image/bmp so this assignment should not flag store as dirty
        let photopea = desktop_entry_store.get_desktop_entry(&photopea_id).unwrap();
        associations.set_default_handler_for_mime_type(&image_bmp, &photopea)?;
        assert!(!associations.is_dirty());

        Ok(())
    }

    #[test]
    fn removing_assignment_dirties_store() -> anyhow::Result<()> {
        let (_desktop_entry_store, mut associations) = create_test_entries_and_associations()?;
        let image_bmp = MimeType::parse("image/bmp")?;

        associations.remove_assigned_applications_for(&image_bmp)?;
        assert!(associations.is_dirty());

        Ok(())
    }

    #[test]
    fn mutating_assignment_dirties_store() -> anyhow::Result<()> {
        let (desktop_entry_store, mut associations) = create_test_entries_and_associations()?;
        let image_bmp = MimeType::parse("image/bmp")?;

        // Assigning evince to image/bmp should mutate store as photopea is current handler
        let eog_id = DesktopEntryId::parse("org.gnome.eog.desktop")?;
        let eog = desktop_entry_store.get_desktop_entry(&eog_id).unwrap();
        associations.set_default_handler_for_mime_type(&image_bmp, &eog)?;
        assert!(associations.is_dirty());

        Ok(())
    }

    #[test]
    fn prune_orphaned_application_assignments_workd() -> anyhow::Result<()> {
        let mut associations = create_test_associations()?;

        let desktop_entry_store =
            DesktopEntryStore::load(&[test_user_applications(), test_sys_applications()])?;

        // add some invalid assignments
        let fake_pdf_assignment = (
            MimeType::parse("application/pdf").unwrap(),
            DesktopEntryId::parse("org.adobe.not-actually-acrobat.desktop").unwrap(),
        );

        let fake_psd_assignment = (
            MimeType::parse("application/psd").unwrap(),
            DesktopEntryId::parse("org.adobe.not-actually-photoshop.desktop").unwrap(),
        );

        associations.scopes[0]
            .default_applications
            .insert(fake_pdf_assignment.0, fake_pdf_assignment.1.clone());

        associations.scopes[0]
            .default_applications
            .insert(fake_psd_assignment.0, fake_psd_assignment.1.clone());

        let result = associations
            .prune_orphaned_application_assignments(&desktop_entry_store)
            .unwrap();

        assert!(
            result.contains(&fake_pdf_assignment.1),
            "Expect result to contain our fake pdf app"
        );
        assert!(
            result.contains(&fake_psd_assignment.1),
            "Expect result to contain our fake psd app"
        );

        Ok(())
    }

    #[test]
    fn remove_assigned_application_works_with_wildcard_mimetypes() -> anyhow::Result<()> {
        let mut associations = create_test_associations()?;

        let image_bmp = MimeType::parse("image/bmp")?;
        let image_png = MimeType::parse("image/png")?;
        let image_pdf = MimeType::parse("image/pdf")?;
        let image_star = MimeType::parse("image/*")?;

        assert_eq!(
            associations
                .assigned_application_for(&image_bmp)
                .unwrap()
                .id(),
            "photopea.desktop"
        );

        assert_eq!(
            associations
                .assigned_application_for(&image_png)
                .unwrap()
                .id(),
            "org.gimp.GIMP.desktop"
        );

        assert_eq!(
            associations
                .assigned_application_for(&image_pdf)
                .unwrap()
                .id(),
            "org.gnome.Evince.desktop"
        );

        associations.remove_assigned_applications_for(&image_star)?;

        let user_scope = associations.get_user_scope().unwrap();
        assert!(!user_scope.default_applications.contains_key(&image_bmp));
        assert!(!user_scope.default_applications.contains_key(&image_png));
        assert!(!user_scope.default_applications.contains_key(&image_pdf));

        Ok(())
    }

    #[test]
    fn mimeassocations_wildcard_lookup_works() -> anyhow::Result<()> {
        let associations = create_test_associations()?;

        let image_star = MimeType::parse("image/*")?;
        let image_bmp = MimeType::parse("image/bmp")?;
        let image_png = MimeType::parse("image/png")?;

        let results = associations.find_matching_mimetypes(&image_star);
        assert!(results.len() > 0);
        assert!(results.contains(&&image_bmp));
        assert!(results.contains(&&image_png));

        let non_wildcard_results = associations.find_matching_mimetypes(&image_bmp);
        assert!(non_wildcard_results.len() == 1);
        assert!(non_wildcard_results.contains(&&image_bmp));

        Ok(())
    }

    #[test]
    fn assigning_system_default_application_for_mimetype_is_equivalent_to_deleting_from_user_scope(
    ) -> anyhow::Result<()> {
        let (entries, mut associations) = create_test_entries_and_associations()?;

        // we need to make first scope user writable for testing
        associations.scopes[0].is_user_customizable = true;

        let image_bmp = MimeType::parse("image/bmp")?;
        let eog_id = DesktopEntryId::parse("org.gnome.eog.desktop")?;
        let eog = entries.get_desktop_entry(&eog_id).unwrap();
        let photopea_id = DesktopEntryId::parse("photopea.desktop")?;

        assert_eq!(
            associations.assigned_application_for(&image_bmp),
            Some(&photopea_id)
        );

        associations.set_default_handler_for_mime_type(&image_bmp, &eog)?;

        assert_eq!(
            associations.assigned_application_for(&image_bmp),
            Some(&eog_id)
        );

        let user_scope = associations.get_user_scope().unwrap();
        assert!(!user_scope.default_applications.contains_key(&image_bmp));

        Ok(())
    }

    #[test]
    fn mime_assocations_loads_expected_added_associations() -> anyhow::Result<()> {
        let associations = create_test_associations()?;

        let html = MimeType::parse("text/html")?;
        let firefox = DesktopEntryId::parse("org.mozilla.firefox.desktop")?;
        let chrome = DesktopEntryId::parse("google-chrome.desktop")?;
        let result = associations.added_associations_for(&html);
        assert_eq!(result, Some(&vec![firefox, chrome]));

        Ok(())
    }

    // serialization test

    #[test]
    fn added_associations_line_roundtrip_works() -> anyhow::Result<()> {
        let input = "image/png=org.gimp.GIMP.desktop;";
        let (mime_type, desktop_entries) = MimeAssociationScope::parse_line(&input)?;
        let output =
            MimeAssociationScope::generate_added_associations_line(&mime_type, &desktop_entries);
        assert_eq!(input, &output);

        let input = "x-scheme-handler/https=org.mozilla.firefox.desktop;google-chrome.desktop;";
        let (mime_type, desktop_entries) = MimeAssociationScope::parse_line(&input)?;
        let output =
            MimeAssociationScope::generate_added_associations_line(&mime_type, &desktop_entries);
        assert_eq!(input, &output);
        Ok(())
    }

    #[test]
    fn default_applications_line_roundtrip_works() -> anyhow::Result<()> {
        let input = "text/html=org.mozilla.firefox.desktop";
        let (mime_type, desktop_entries) = MimeAssociationScope::parse_line(&input)?;
        let output = MimeAssociationScope::generate_default_application_line(
            &mime_type,
            &desktop_entries[0],
        );
        assert_eq!(input, &output);
        Ok(())
    }

    #[test]
    fn mimeassociationscope_roundtrip_works() -> anyhow::Result<()> {
        let input_path = path("test-data/config/mimeapps.list");
        let output_path = path("test-data/config/mimeapps.list.copy");
        delete_file(&output_path);

        let input_mimeassociations = MimeAssociationScope::load(&input_path)?;
        input_mimeassociations.write_to_path(&output_path)?;

        let copy_mimeassociations = MimeAssociationScope::load(&output_path)?;

        assert_eq!(
            input_mimeassociations.added_associations,
            copy_mimeassociations.added_associations
        );
        assert_eq!(
            input_mimeassociations.default_applications,
            copy_mimeassociations.default_applications
        );

        delete_file(&output_path);

        Ok(())
    }

    #[test]
    fn make_default_handler_works_for_valid_usecases() -> anyhow::Result<()> {
        let (entries, mut associations) = create_test_entries_and_associations()?;

        // we need to make first scope user writable for testing
        associations.scopes[0].is_user_customizable = true;

        let photopea_id = DesktopEntryId::parse("photopea.desktop")?;
        let photopea = entries.get_desktop_entry(&photopea_id).unwrap();
        let evince_id = DesktopEntryId::parse("org.gnome.Evince.desktop")?;
        let image_tiff = MimeType::parse("image/tiff")?;

        // we're going to verify Evince is set to image/tiff
        assert_eq!(
            associations.assigned_application_for(&image_tiff),
            Some(&evince_id)
        );

        // assign photopea
        associations.set_default_handler_for_mime_type(&image_tiff, &photopea)?;
        assert_eq!(
            associations.assigned_application_for(&image_tiff),
            Some(&photopea_id)
        );

        Ok(())
    }

    #[test]
    fn make_default_handler_errors_for_unsupported_mimetypes() -> anyhow::Result<()> {
        let (entries, mut associations) = create_test_entries_and_associations()?;

        // we need to make first scope user writable for testing
        associations.scopes[0].is_user_customizable = true;

        let photopea_id = DesktopEntryId::parse("photopea.desktop")?;
        let photopea = entries.get_desktop_entry(&photopea_id).unwrap();

        // photopea doesn't support inode/directory
        let inode_directory = MimeType::parse("inode/directory")?;

        assert!(associations
            .set_default_handler_for_mime_type(&inode_directory, &photopea)
            .is_err());

        Ok(())
    }

    #[test]
    fn make_default_handler_errors_without_writeable_scope() -> anyhow::Result<()> {
        let (entries, mut associations) = create_test_entries_and_associations()?;
        associations.scopes[0].is_user_customizable = false;

        let photopea_id = DesktopEntryId::parse("photopea.desktop")?;
        let photopea = entries.get_desktop_entry(&photopea_id).unwrap();
        let image_tiff = MimeType::parse("image/tiff")?;

        // assignment should fail since no writable scope is set
        assert!(associations
            .set_default_handler_for_mime_type(&image_tiff, &photopea)
            .is_err());

        Ok(())
    }
}
