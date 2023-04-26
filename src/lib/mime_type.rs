use std::{
    collections::HashMap,
    fmt::Display,
    fs::File,
    io::{self, BufRead},
    path::{Path, PathBuf},
};

use crate::lib::app_id::*;

#[derive(Debug, PartialEq, Eq)]
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

#[derive(Default)]
pub struct MimeAssociations {
    file: PathBuf,
    is_writable: bool,
    store: HashMap<String, MimeType>,
}

impl MimeAssociations {
    fn new<P>(mimeapps_file_path: P, is_writable: bool) -> anyhow::Result<Self>
    where
        P: AsRef<Path>,
    {
        let mimeapps_file = File::open(mimeapps_file_path.as_ref())?;
        let line_buffer = io::BufReader::new(mimeapps_file).lines();
        let mut store = HashMap::new();
        for line in line_buffer {
            if let Ok(line) = line {
                let (id, mime_type) = Self::parse_line(&line)?;
                store.insert(id, mime_type);
            }
        }

        Ok(MimeAssociations {
            file: PathBuf::from(mimeapps_file_path.as_ref()),
            is_writable,
            store,
        })
    }

    fn parse_line(line: &str) -> anyhow::Result<(String, MimeType)> {
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

        Ok((app_id.id, mime_type))
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
            for (_, mime_type) in associations.store.iter() {
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

    fn path(p: &str) -> PathBuf {
        let cwd = std::env::current_dir().unwrap();
        cwd.join(p)
    }

    #[test]
    fn mime_type_parse() {
        assert!(MimeType::parse("foo/bar").is_ok());
        assert!(MimeType::parse("foobar").is_err());
        assert!(MimeType::parse("foo/bar/baz").is_err());

        let mime = MimeType::parse("foo/bar").unwrap();
        assert_eq!(mime.major_type(), "foo");
        assert_eq!(mime.sub_type(), "bar");
    }

    #[test]
    fn mime_associations_load() {
        let test_path = path("test-data/usr/share/applications/mimeapps.list");

        // this test is failing because we're not processing the [Default Applications] and [Added Associations] sections

        assert!(MimeAssociations::new(test_path, false).is_ok());
    }

    #[test]
    fn mime_associations_line_parser() {
        let result = MimeAssociations::parse_line("foo/bar=baz.desktop;");
        assert!(result.is_ok());
        let result = result.unwrap();
        assert_eq!(result.0, "baz.desktop");
        assert_eq!(result.1, MimeType::parse("foo/bar").unwrap());

        let result = MimeAssociations::parse_line("   foo/bar=baz.desktop\n  ");
        assert!(result.is_ok());
        let result = result.unwrap();
        assert_eq!(result.0, "baz.desktop");
        assert_eq!(result.1, MimeType::parse("foo/bar").unwrap());

        assert!(MimeAssociations::parse_line("foo/bar=baz").is_err());
        assert!(MimeAssociations::parse_line("foobar=baz.desktop;").is_err());
    }
}
