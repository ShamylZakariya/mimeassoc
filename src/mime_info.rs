use std::{collections::HashMap, fs::File, io::BufReader, path::Path};

use crate::mime_type::MimeType;

#[derive(Debug, PartialEq, Eq)]
pub struct MimeTypeInfo {
    mime_type: MimeType,

    // default comment, `en`
    comment: Option<String>,
    // comments for other languages, "en_GB", "de", etc.
    comments: HashMap<String, String>,
    generic_icon: Option<String>,
    glob_patterns: Vec<String>,
    aliases: Vec<MimeType>,
}
impl MimeTypeInfo {
    fn new(mime_type: &MimeType) -> Self {
        Self {
            mime_type: mime_type.clone(),
            comment: None,
            comments: HashMap::new(),
            generic_icon: None,
            glob_patterns: Vec::new(),
            aliases: Vec::new(),
        }
    }

    pub fn mime_type(&self) -> &MimeType {
        &self.mime_type
    }

    pub fn comment(&self) -> Option<&str> {
        self.comment.as_deref()
    }

    pub fn comment_languages(&self) -> Vec<&str> {
        let languages = self.comments.keys();
        languages.map(|k| k.as_str()).collect()
    }

    pub fn comment_language(&self, lang: &str) -> Option<&str> {
        self.comments.get(lang).map(|c| c.as_str())
    }

    pub fn generic_icon(&self) -> Option<&str> {
        self.generic_icon.as_deref()
    }

    pub fn extensions(&self) -> Vec<&str> {
        self.glob_patterns
            .iter()
            .filter_map(|glob| {
                let components = glob.split('.').collect::<Vec<_>>();
                components.get(1).copied()
            })
            .collect()
    }

    pub fn aliases(&self) -> Vec<&MimeType> {
        self.aliases.iter().collect()
    }
}

pub struct MimeTypeInfoStore {
    mime_types: HashMap<MimeType, MimeTypeInfo>,

    /// map of aliases to mime types stored in Self::mime_types. For example,
    /// "application/vnd.amazon.mobi8-ebook" has an alias "application/x-mobi8-ebook".
    /// That means, looking up aliases["application/x-mobi8-ebook"] gives us "application/vnd.amazon.mobi8-ebook"
    aliases: HashMap<MimeType, MimeType>,
}

impl MimeTypeInfoStore {
    fn load<P: AsRef<Path>>(mime_info_xml_paths: &[P]) -> anyhow::Result<Self> {
        let mut store = Self {
            mime_types: HashMap::new(),
            aliases: HashMap::new(),
        };

        for path in mime_info_xml_paths.iter() {
            Self::load_single_xml_into_store(path, &mut store)?;
        }

        store.resolve_aliases();
        Ok(store)
    }

    fn load_single_xml_into_store<P: AsRef<Path>>(
        freedesktop_org_xml_path: P,
        store: &mut Self,
    ) -> anyhow::Result<()> {
        let path = freedesktop_org_xml_path.as_ref();
        let file = File::open(path)?;
        let reader = BufReader::new(file);
        let parser = xml::EventReader::new(reader);

        // in-flight data to handle while sax parsing; this is ugly, but sax parsing always is
        let mut current_mime_type_info: Option<MimeTypeInfo> = None;
        let mut is_handling_comment = false;
        let mut current_comment_language: Option<String> = None;

        for e in parser {
            match e {
                Ok(xml::reader::XmlEvent::StartElement {
                    name, attributes, ..
                }) => match name.local_name.as_str() {
                    "mime-type" => {
                        let mime_type = Self::get_attribute_named(&attributes, "type")
                            .and_then(|attr| MimeType::parse(&attr.value).ok());

                        if let Some(mime_type) = mime_type {
                            current_mime_type_info.replace(MimeTypeInfo::new(&mime_type));
                        }
                    }

                    "comment" => {
                        is_handling_comment = true;
                        let language_attr = Self::get_attribute_named(&attributes, "lang");
                        if let Some(language_attr) = language_attr {
                            current_comment_language = Some(language_attr.value.trim().to_string());
                        } else {
                            current_comment_language = None;
                        }
                    }

                    "generic-icon" => {
                        if let Some(icon_name_attr) = Self::get_attribute_named(&attributes, "name")
                        {
                            if let Some(current_mime_type_info) = current_mime_type_info.as_mut() {
                                let icon_name = icon_name_attr.value.to_string();
                                current_mime_type_info.generic_icon = Some(icon_name);
                            }
                        }
                    }

                    "glob" => {
                        if let Some(glob_pattern) =
                            Self::get_attribute_named(&attributes, "pattern")
                        {
                            if let Some(current_mime_type_info) = current_mime_type_info.as_mut() {
                                let glob_pattern = glob_pattern.value.to_string();
                                current_mime_type_info.glob_patterns.push(glob_pattern);
                            }
                        }
                    }

                    "alias" => {
                        if let Some(alias_attr) = Self::get_attribute_named(&attributes, "type") {
                            if let Some(current_mime_type_info) = current_mime_type_info.as_mut() {
                                if let Ok(alias_mime_type) =
                                    MimeType::parse(alias_attr.value.trim())
                                {
                                    current_mime_type_info.aliases.push(alias_mime_type);
                                }
                            }
                        }
                    }

                    _ => {}
                },
                Ok(xml::reader::XmlEvent::Characters(characters)) => {
                    if is_handling_comment {
                        let comment = characters.trim().to_string();
                        if let Some(current_mime_type_info) = current_mime_type_info.as_mut() {
                            if let Some(language) = current_comment_language.take() {
                                current_mime_type_info.comments.insert(language, comment);
                            } else {
                                current_mime_type_info.comment = Some(comment);
                            }
                        }
                    }
                }
                Ok(xml::reader::XmlEvent::EndElement { name }) => match name.local_name.as_str() {
                    "mime-type" => {
                        if let Some(current_mime_type_info) = current_mime_type_info.take() {
                            let key = current_mime_type_info.mime_type.clone();
                            store.mime_types.insert(key, current_mime_type_info);
                        }
                    }
                    "comment" => {
                        is_handling_comment = false;
                    }
                    _ => {}
                },
                Err(e) => {
                    return Err(e.into());
                }
                _ => {}
            }
        }

        Ok(())
    }

    pub fn get_info_for_mime_type(&self, mime_type: &MimeType) -> Option<&MimeTypeInfo> {
        if let Some(info) = self.mime_types.get(mime_type) {
            Some(info)
        } else if let Some(resolved_mime_type) = self.aliases.get(mime_type) {
            self.mime_types.get(resolved_mime_type)
        } else {
            None
        }
    }

    /// Find the first attribute with matching name, if any
    fn get_attribute_named<'a>(
        attributes: &'a [xml::attribute::OwnedAttribute],
        name: &str,
    ) -> Option<&'a xml::attribute::OwnedAttribute> {
        let results = attributes
            .iter()
            .filter(|a| a.name.local_name == name)
            .collect::<Vec<_>>();
        results.first().copied()
    }

    fn resolve_aliases(&mut self) {
        for mime_type_info in self.mime_types.iter() {
            for alias in mime_type_info.1.aliases.iter() {
                self.aliases.insert(alias.clone(), mime_type_info.0.clone());
            }
        }
    }
}

///////////////////////////////////////////////////////////////////////////////////////////////////////////////////////

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn path(p: &str) -> PathBuf {
        let cwd = std::env::current_dir().unwrap();
        cwd.join(p)
    }

    fn tiny_freedesktop_org_xml_path() -> PathBuf {
        path("test-data/usr/share/mime/packages/tiny_freedesktop.org.xml")
    }

    fn code_workspace_xml_path() -> PathBuf {
        path("test-data/usr/share/mime/packages/code-workspace.xml")
    }

    fn full_freedesktop_org_xml_path() -> PathBuf {
        path("test-data/usr/share/mime/packages/freedesktop.org.xml")
    }

    fn loads_and_contains_expected_data<P: AsRef<Path>>(
        freedesktop_org_xml_path: P,
    ) -> anyhow::Result<()> {
        let store = MimeTypeInfoStore::load(&[freedesktop_org_xml_path])?;

        // look up atari 7800 ROM mime type
        let atari_7800_mime_type = MimeType::parse("application/x-atari-7800-rom")?;
        let atari_7800_info = store
            .get_info_for_mime_type(&atari_7800_mime_type)
            .expect("Expect \"application/vnd.amazon.mobi8-ebook\" to be in the info store");

        assert_eq!(atari_7800_info.mime_type, atari_7800_mime_type);
        assert_eq!(atari_7800_info.comment(), Some("Atari 7800 ROM"));
        assert_eq!(
            atari_7800_info.comment_language("ko"),
            Some("아타리 7800 롬")
        );
        assert_eq!(
            atari_7800_info.generic_icon(),
            Some("application-x-executable")
        );

        let extensions = atari_7800_info.extensions();
        assert_eq!(extensions, ["a78"]);

        // Look up amazon mobi mime type
        let mobi_mime_type = MimeType::parse("application/vnd.amazon.mobi8-ebook")?;
        let mobi_info = store
            .get_info_for_mime_type(&mobi_mime_type)
            .expect("Expect \"application/vnd.amazon.mobi8-ebook\" to be in the info store");

        assert_eq!(mobi_info.mime_type, mobi_mime_type);
        assert_eq!(mobi_info.comment(), Some("Kindle book document"));
        assert_eq!(
            mobi_info.comment_language("fr"),
            Some("document livre Kindle")
        );
        assert!(mobi_info.generic_icon().is_none());

        let extensions = mobi_info.extensions();
        assert!(extensions.contains(&"azw3"));
        assert!(extensions.contains(&"kfx"));

        // mobi has an alias
        let mobi_alias_mime_type = MimeType::parse("application/x-mobi8-ebook")?;
        let mobi_alias_info = store
            .get_info_for_mime_type(&mobi_alias_mime_type)
            .expect("Expect to look up resolved mime type for alias");

        assert_eq!(mobi_info, mobi_alias_info);

        Ok(())
    }

    #[test]
    fn loads_tiny_freedesktop_org_xml() -> anyhow::Result<()> {
        loads_and_contains_expected_data(tiny_freedesktop_org_xml_path())
    }

    #[test]
    fn loads_full_freedesktop_org_xml() -> anyhow::Result<()> {
        loads_and_contains_expected_data(full_freedesktop_org_xml_path())
    }

    #[test]
    fn merges_multiple_sources() -> anyhow::Result<()> {
        let sources = vec![tiny_freedesktop_org_xml_path(), code_workspace_xml_path()];
        let store = MimeTypeInfoStore::load(&sources)?;

        let mobi_mime_type = MimeType::parse("application/vnd.amazon.mobi8-ebook")?;
        let code_workspace_mime_type = MimeType::parse("application/x-code-workspace")?;

        assert!(store.get_info_for_mime_type(&mobi_mime_type).is_some());
        assert!(store
            .get_info_for_mime_type(&code_workspace_mime_type)
            .is_some());

        Ok(())
    }
}
