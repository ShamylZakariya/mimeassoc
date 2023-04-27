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
    added_associations: HashMap<MimeType, AppId>,
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

        for line in line_buffer {
            if let Ok(line) = line {
                if let Some(section) = MimeAssociationsSection::try_parse(&line) {
                    // catch [Section] directives in the list
                    current_section = Some(section);
                } else if let Some(current_section) = &current_section {
                    // if we have a current section, we can add associations to it.
                    let trimmed_line = line.trim();

                    if let Ok((mime_type, app_id)) = Self::parse_line(&trimmed_line) {
                        match current_section {
                            MimeAssociationsSection::AddedAssociations => {
                                added_associations.insert(mime_type, app_id)
                            }
                            MimeAssociationsSection::DefaultApplications => {
                                default_applications.insert(mime_type, app_id)
                            }
                        };
                    } else if !trimmed_line.starts_with("#") && !trimmed_line.is_empty() {
                        // this line is not a section directive, MimeAssociation, or comment
                        anyhow::bail!(
                            "Unable to parse MimeAssociation from line: \"{}\"",
                            trimmed_line
                        );
                    }
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

    fn parse_line(line: &str) -> anyhow::Result<(MimeType, AppId)> {
        let components = line.split('=').collect::<Vec<_>>();
        if components.len() != 2 {
            anyhow::bail!("A line from mimeapps.lst is expected to be in form \"mime/type=app.desktop\". Line \"{}\" was invalid", line);
        }

        let mime_type_component = components[0].trim();
        let app_component = components[1].trim();

        let mime_type = MimeType::parse(mime_type_component)?;

        let app_id = if app_component.ends_with(';') {
            AppId::parse(&app_component[0..app_component.len() - 1])
        } else {
            AppId::parse(app_component)
        }?;

        Ok((mime_type, app_id))
    }
}

struct MimeAssociationsStore {
    associations: Vec<MimeAssociations>,
}

impl MimeAssociationsStore {
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
        assert_eq!(&associations.added_associations[&png], &gimp);

        Ok(())
    }

    #[test]
    fn mime_associations_line_parser() -> anyhow::Result<()> {
        let result = MimeAssociations::parse_line("foo/bar=baz.desktop;")?;
        assert_eq!(result.0, MimeType::parse("foo/bar")?);
        assert_eq!(result.1, AppId::parse("baz.desktop")?);

        let result = MimeAssociations::parse_line("   foo/bar=baz.desktop\n  ")?;
        assert_eq!(result.0, MimeType::parse("foo/bar")?);
        assert_eq!(result.1, AppId::parse("baz.desktop")?);

        assert!(MimeAssociations::parse_line("foo/bar=baz").is_err());
        assert!(MimeAssociations::parse_line("foobar=baz.desktop;").is_err());

        Ok(())
    }
}
