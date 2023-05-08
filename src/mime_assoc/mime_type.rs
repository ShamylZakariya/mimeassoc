use std::{
    collections::HashMap,
    fmt::Display,
    fs::File,
    io::{self, BufRead, Write},
    path::{Path, PathBuf},
};

use super::desktop_entry::DesktopEntryId;

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
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
    file: PathBuf,
    is_writable: bool,
    added_associations: HashMap<MimeType, Vec<DesktopEntryId>>,
    default_applications: HashMap<MimeType, DesktopEntryId>,
}

impl MimeAssociationScope {
    fn load<P>(mimeapps_file_path: P, is_writable: bool) -> anyhow::Result<Self>
    where
        P: AsRef<Path>,
    {
        let mimeapps_file = File::open(mimeapps_file_path.as_ref())?;
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

        Ok(MimeAssociationScope {
            file: PathBuf::from(mimeapps_file_path.as_ref()),
            is_writable,
            added_associations,
            default_applications,
        })
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

    fn write<P>(&self, path: P) -> anyhow::Result<()>
    where
        P: AsRef<Path>,
    {
        let output_file = File::create(path)?;

        // write the added associations
        if !self.added_associations.is_empty() {
            writeln!(
                &output_file,
                "{}",
                MimeAssociationsSections::AddedAssociations.to_string()
            )?;
            for (mime_type, desktop_entries) in self.added_associations.iter() {
                writeln!(
                    &output_file,
                    "{};",
                    Self::generate_added_associations_line(mime_type, desktop_entries)
                )?;
            }
            writeln!(&output_file)?;
        }

        // write the default applications
        if !self.default_applications.is_empty() {
            writeln!(
                &output_file,
                "{}",
                MimeAssociationsSections::DefaultApplications.to_string()
            )?;
            for (mime_type, desktop_entry) in self.default_applications.iter() {
                writeln!(
                    &output_file,
                    "{}",
                    Self::generate_default_application_line(mime_type, desktop_entry)
                )?;
            }
        }

        Ok(())
    }
}

pub struct MimeAssociations {
    scopes: Vec<MimeAssociationScope>,
}

impl MimeAssociations {
    /// Load MimeAssocations in order of the provided paths. MimeAssocations earlier in
    /// the list will override ones later in the list.
    pub fn load<P>(mimeapps_file_paths: &[(P, bool)]) -> anyhow::Result<Self>
    where
        P: AsRef<Path>,
    {
        let mut scopes = Vec::new();
        for (file_path, is_writable) in mimeapps_file_paths.iter() {
            scopes.push(MimeAssociationScope::load(file_path, *is_writable)?);
        }

        Ok(Self { scopes })
    }

    pub fn mime_types(&self) -> Vec<&MimeType> {
        let mut mime_types = Vec::new();
        for scope in self.scopes.iter().rev() {
            for (mime_type, _) in scope.default_applications.iter() {
                mime_types.push(mime_type);
            }
        }

        mime_types
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
}

///////////////////////////////////////////////////////////////////////////////////////////////////////////////////////

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
        assert!(MimeAssociationScope::load(test_sys_mimeapps_list(), false).is_ok());
        assert!(MimeAssociationScope::load(test_gnome_mimeapps_list(), false).is_ok());
        assert!(MimeAssociationScope::load(test_user_mimeapps_list(), false).is_ok());
    }

    #[test]
    fn mime_associations_load_expected_data() -> anyhow::Result<()> {
        let associations = MimeAssociationScope::load(test_user_mimeapps_list(), false)?;

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
            (test_sys_mimeapps_list(), false),
            (test_gnome_mimeapps_list(), false),
            (test_user_mimeapps_list(), false),
        ])?;

        Ok(())
    }

    #[test]
    fn mime_assocations_prefers_user_default_application_over_system_associations(
    ) -> anyhow::Result<()> {
        let associations = MimeAssociations::load(&[
            (test_user_mimeapps_list(), false),
            (test_gnome_mimeapps_list(), false),
            (test_sys_mimeapps_list(), false),
        ])?;

        let html = MimeType::parse("text/html")?;
        let firefox = DesktopEntryId::parse("org.mozilla.firefox.desktop")?;
        assert_eq!(associations.default_application_for(&html), Some(&firefox));

        Ok(())
    }

    #[test]
    fn mime_assocations_loads_expected_added_associations() -> anyhow::Result<()> {
        let associations = MimeAssociations::load(&[
            (test_user_mimeapps_list(), false),
            (test_gnome_mimeapps_list(), false),
            (test_sys_mimeapps_list(), false),
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

        let input_mimeassociations = MimeAssociationScope::load(&input_path, true)?;
        input_mimeassociations.write(&output_path)?;

        let copy_mimeassociations = MimeAssociationScope::load(&output_path, true)?;

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
}
