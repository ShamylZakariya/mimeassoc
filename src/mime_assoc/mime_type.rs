use std::{
    collections::HashMap,
    fmt::Display,
    fs::File,
    io::{self, BufRead, Write},
    ops::Deref,
    path::{Path, PathBuf},
};

use super::desktop_entry::{DesktopEntry, DesktopEntryId};

#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct MimeType {
    pub id: String,
}

impl MimeType {
    pub fn parse(id: &str) -> anyhow::Result<Self> {
        let components = id.split('/').collect::<Vec<_>>();
        if components.len() != 2 {
            anyhow::bail!(
                "A mimetype is expected to contain exactly one `/`. id: \"{}\" is invalid.",
                id
            )
        }
        Ok(Self { id: id.to_string() })
    }

    pub fn major_type(&self) -> &str {
        let slash_pos = self.id.find('/').expect("Mimetype should contain a '/'");
        &self.id[0..slash_pos]
    }

    pub fn sub_type(&self) -> &str {
        let slash_pos = self.id.find('/').expect("Mimetype should contain a '/'");
        &self.id[slash_pos + 1..self.id.len()]
    }
}

impl Display for MimeType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.id)
    }
}

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

#[derive(Default, PartialEq, Eq)]
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
        let mimeapps_file = File::open(mimeapps_file_path)?;
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

        let is_user_customizable = mimeapps_file_path == super::user_mimeapps_list_path()?;

        Ok(MimeAssociationScope {
            file_path: PathBuf::from(mimeapps_file_path),
            is_user_customizable,
            is_dirty: false,
            added_associations,
            default_applications,
        })
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

pub struct MimeAssociations {
    scopes: Vec<MimeAssociationScope>,
}

impl MimeAssociations {
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

    /// Return all mimetypes represented, in no particular order.
    pub fn mime_types(&self) -> Vec<&MimeType> {
        let mut mime_types = Vec::new();
        for scope in self.scopes.iter().rev() {
            for (mime_type, _) in scope.default_applications.iter() {
                mime_types.push(mime_type);
            }
        }

        mime_types
    }

    /// Return the sources used to create this store, in preferential chain order, e.g., user entries before system.
    pub fn sources(&self) -> Vec<&Path> {
        self.scopes.iter().map(|s| s.file_path.deref()).collect()
    }

    pub fn default_application_for(&self, mime_type: &MimeType) -> Option<&DesktopEntryId> {
        for scope in self.scopes.iter() {
            if let Some(id) = scope.default_applications.get(mime_type) {
                return Some(id);
            }
        }
        None
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

        for scope in self.scopes.iter_mut() {
            if scope.is_user_customizable {
                scope
                    .default_applications
                    .insert(mime_type.clone(), desktop_entry.id().clone());
                scope.is_dirty = true;
                return Ok(());
            }
        }

        anyhow::bail!("No user customizable scopes in MimeAssociation scope chain");
    }

    /// Make the provided DesktopEnrtry the default handler for all its supported mimetypes.
    /// Will return an error if the desktop entry isn't a valid application, or if there are
    /// no user customizable scoped in the MimeAssociationScope chain
    pub fn make_desktop_entry_default_handler_of_its_supported_mime_types(
        &mut self,
        desktop_entry: &DesktopEntry,
    ) -> anyhow::Result<()> {
        for mime_type in desktop_entry.mime_types() {
            self.set_default_handler_for_mime_type(mime_type, desktop_entry)?;
        }
        Ok(())
    }

    /// Save any unpersisted changes to user customizable scopes
    pub fn save(&mut self) -> anyhow::Result<()> {
        for scope in self.scopes.iter_mut() {
            if scope.is_user_customizable && scope.is_dirty {
                scope.save()?;
            }
        }

        Ok(())
    }
}

///////////////////////////////////////////////////////////////////////////////////////////////////////////////////////

#[cfg(test)]
mod tests {
    use crate::mime_assoc::desktop_entry::DesktopEntries;

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

    fn create_test_entries_and_associations() -> anyhow::Result<(DesktopEntries, MimeAssociations)>
    {
        let entries = DesktopEntries::load(&[test_user_applications(), test_sys_applications()])?;

        let associations = MimeAssociations::load(&[
            test_user_mimeapps_list(),
            test_gnome_mimeapps_list(),
            test_sys_mimeapps_list(),
        ])?;
        Ok((entries, associations))
    }

    #[test]
    fn mime_type_parse() -> anyhow::Result<()> {
        assert!(MimeType::parse("foo/bar").is_ok());
        assert!(MimeType::parse("foobar").is_err());
        assert!(MimeType::parse("foo/bar/baz").is_err());

        let mime = MimeType::parse("foo/bar")?;
        assert_eq!(mime.major_type(), "foo");
        assert_eq!(mime.sub_type(), "bar");

        Ok(())
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
        let _ = MimeAssociations::load(&[
            test_sys_mimeapps_list(),
            test_gnome_mimeapps_list(),
            test_user_mimeapps_list(),
        ])?;

        Ok(())
    }

    #[test]
    fn mime_assocations_prefers_user_default_application_over_system_associations(
    ) -> anyhow::Result<()> {
        let associations = MimeAssociations::load(&[
            test_user_mimeapps_list(),
            test_gnome_mimeapps_list(),
            test_sys_mimeapps_list(),
        ])?;

        let html = MimeType::parse("text/html")?;
        let firefox = DesktopEntryId::parse("org.mozilla.firefox.desktop")?;
        assert_eq!(associations.default_application_for(&html), Some(&firefox));

        Ok(())
    }

    #[test]
    fn mime_assocations_loads_expected_added_associations() -> anyhow::Result<()> {
        let associations = MimeAssociations::load(&[
            test_user_mimeapps_list(),
            test_gnome_mimeapps_list(),
            test_sys_mimeapps_list(),
        ])?;

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
            associations.default_application_for(&image_tiff),
            Some(&evince_id)
        );

        // assign photopea
        associations.set_default_handler_for_mime_type(&image_tiff, &photopea)?;
        assert_eq!(
            associations.default_application_for(&image_tiff),
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
