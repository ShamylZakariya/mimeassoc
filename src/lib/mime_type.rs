use std::{
    collections::HashMap,
    fmt::Display,
    fs::File,
    io::{self, BufRead},
    path::{Path, PathBuf},
};

use crate::lib::app_id::*;

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

enum MimeAssociationsSection {
    AddedAssociations,
    DefaultApplications,
}

impl MimeAssociationsSection {
    fn try_parse(desc: &str) -> Option<Self> {
        let desc = desc.trim();
        if desc == "[Added Associations]" {
            Some(MimeAssociationsSection::AddedAssociations)
        } else if desc == "[Default Applications]" {
            Some(MimeAssociationsSection::DefaultApplications)
        } else {
            None
        }
    }
}

#[derive(Default)]
pub struct MimeAssociations {
    file: PathBuf,
    is_writable: bool,
    added_associations: HashMap<MimeType, Vec<AppId>>,
    default_applications: HashMap<MimeType, AppId>,
}

impl MimeAssociations {
    fn new<P>(mimeapps_file_path: P, is_writable: bool) -> anyhow::Result<Self>
    where
        P: AsRef<Path>,
    {
        let mimeapps_file = File::open(mimeapps_file_path.as_ref())?;
        let line_buffer = io::BufReader::new(mimeapps_file).lines();
        let mut added_associations = HashMap::new();
        let mut default_applications = HashMap::new();
        let mut current_section: Option<MimeAssociationsSection> = None;

        for line in line_buffer.flatten() {
            if let Some(section) = MimeAssociationsSection::try_parse(&line) {
                // catch [Section] directives in the list
                current_section = Some(section);
            } else if let Some(current_section) = &current_section {
                // if we have a current section, we can add associations to it.
                let trimmed_line = line.trim();

                if let Ok((mime_type, app_ids)) = Self::parse_line(trimmed_line) {
                    match current_section {
                        MimeAssociationsSection::AddedAssociations => {
                            added_associations.insert(mime_type, app_ids);
                        }
                        MimeAssociationsSection::DefaultApplications => {
                            if let Some(app_id) = app_ids.first() {
                                default_applications.insert(mime_type, app_id.clone());
                            } else {
                                anyhow::bail!("Line \"{}\" specified 0 AppIds", trimmed_line);
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

        Ok(MimeAssociations {
            file: PathBuf::from(mimeapps_file_path.as_ref()),
            is_writable,
            added_associations,
            default_applications,
        })
    }

    fn parse_line(line: &str) -> anyhow::Result<(MimeType, Vec<AppId>)> {
        let components = line.split('=').collect::<Vec<_>>();
        if components.len() != 2 {
            anyhow::bail!("A line from mimeapps.lst is expected to be in form \"mime/type=app.desktop\". Line \"{}\" was invalid", line);
        }

        let mime_type_component = components[0].trim();
        let app_component = components[1].trim();

        let mime_type = MimeType::parse(mime_type_component)?;

        if !app_component.contains(';') {
            return Ok((mime_type, vec![AppId::parse(app_component)?]));
        }

        let mut app_ids = Vec::new();
        for app_id_component in app_component.split(';') {
            let trimmed_app_id_component = app_id_component.trim();
            if !trimmed_app_id_component.is_empty() {
                app_ids.push(AppId::parse(trimmed_app_id_component)?);
            }
        }

        Ok((mime_type, app_ids))
    }
}

struct MimeAssociationsCascade {
    associations: Vec<MimeAssociations>,
}

impl MimeAssociationsCascade {
    /// Load MimeAssocations in order of the provided paths. MimeAssocations later in
    /// the list will override ones earlier in the list.
    pub fn new<P>(mimeapps_file_paths: &[(P, bool)]) -> anyhow::Result<Self>
    where
        P: AsRef<Path>,
    {
        let mut associations = Vec::new();
        for (file_path, is_writable) in mimeapps_file_paths.iter() {
            associations.push(MimeAssociations::new(file_path, *is_writable)?);
        }

        Ok(Self { associations })
    }

    pub fn mime_types(&self) -> Vec<&MimeType> {
        let mut mime_types = Vec::new();
        for associations in self.associations.iter() {
            for (mime_type, _) in associations.default_applications.iter() {
                mime_types.push(mime_type);
            }
        }

        mime_types
    }

    pub fn default_application_for(&self, mime_type: &MimeType) -> Option<&AppId> {
        for association in self.associations.iter().rev() {
            if let Some(app_id) = association.default_applications.get(mime_type) {
                return Some(app_id);
            }
        }
        None
    }

    pub fn added_associations_for(&self, mime_type: &MimeType) -> Option<&Vec<AppId>> {
        for association in self.associations.iter().rev() {
            if let Some(app_id) = association.added_associations.get(mime_type) {
                return Some(app_id);
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
        assert!(MimeAssociations::new(test_sys_mimeapps_list(), false).is_ok());
        assert!(MimeAssociations::new(test_gnome_mimeapps_list(), false).is_ok());
        assert!(MimeAssociations::new(test_user_mimeapps_list(), false).is_ok());
    }

    #[test]
    fn mime_associations_load_expected_data() -> anyhow::Result<()> {
        let associations = MimeAssociations::new(test_user_mimeapps_list(), false)?;

        let png = MimeType::parse("image/png")?;
        let gimp = AppId::parse("org.gimp.GIMP.desktop")?;
        assert_eq!(&associations.added_associations[&png], &vec![gimp]);

        Ok(())
    }

    #[test]
    fn mime_associations_line_parser() -> anyhow::Result<()> {
        let baz_desktop = AppId::parse("baz.desktop")?;
        let qux_desktop = AppId::parse("qux.desktop")?;
        let zim_desktop = AppId::parse("zim.desktop")?;

        // single value with trailing semicolon
        let result = MimeAssociations::parse_line("foo/bar=baz.desktop;")?;
        assert_eq!(result.0, MimeType::parse("foo/bar")?);
        assert_eq!(result.1, vec![baz_desktop.clone()]);

        // whitespace
        let result = MimeAssociations::parse_line("   foo/bar=baz.desktop\n  ")?;
        assert_eq!(result.0, MimeType::parse("foo/bar")?);
        assert_eq!(result.1, vec![baz_desktop.clone()]);

        // multiple values
        let result = MimeAssociations::parse_line("foo/bar=baz.desktop;qux.desktop;zim.desktop;")?;
        assert_eq!(result.0, MimeType::parse("foo/bar")?);
        assert_eq!(result.1, vec![baz_desktop, qux_desktop, zim_desktop]);

        assert!(MimeAssociations::parse_line("foo/bar=baz").is_err());
        assert!(MimeAssociations::parse_line("foobar=baz.desktop;").is_err());

        Ok(())
    }

    #[test]
    fn mime_assocations_cascade_loads() -> anyhow::Result<()> {
        let _ = MimeAssociationsCascade::new(&[
            (test_sys_mimeapps_list(), false),
            (test_gnome_mimeapps_list(), false),
            (test_user_mimeapps_list(), false),
        ])?;

        Ok(())
    }

    #[test]
    fn mime_assocations_cascade_prefers_user_default_application_over_system_associations(
    ) -> anyhow::Result<()> {
        let cascade = MimeAssociationsCascade::new(&[
            (test_sys_mimeapps_list(), false),
            (test_gnome_mimeapps_list(), false),
            (test_user_mimeapps_list(), false),
        ])?;

        let html = MimeType::parse("text/html")?;
        let firefox = AppId::parse("org.mozilla.firefox.desktop")?;
        assert_eq!(cascade.default_application_for(&html), Some(&firefox));

        Ok(())
    }

    #[test]
    fn mime_assocations_cascade_loads_expected_added_associations() -> anyhow::Result<()> {
        let cascade = MimeAssociationsCascade::new(&[
            (test_sys_mimeapps_list(), false),
            (test_gnome_mimeapps_list(), false),
            (test_user_mimeapps_list(), false),
        ])?;

        let html = MimeType::parse("text/html")?;
        let firefox = AppId::parse("org.mozilla.firefox.desktop")?;
        let chrome = AppId::parse("google-chrome.desktop")?;
        let result = cascade.added_associations_for(&html);
        assert_eq!(result, Some(&vec![firefox, chrome]));

        Ok(())
    }
}
