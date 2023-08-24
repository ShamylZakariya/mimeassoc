use serde::Serialize;
use std::fmt::Display;

#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize)]
pub struct MimeType(String);

impl MimeType {
    pub fn parse(id: &str) -> anyhow::Result<Self> {
        let components = id.split('/').collect::<Vec<_>>();
        if components.len() != 2 {
            anyhow::bail!(
                "A mimetype is expected to contain exactly one `/`. id: \"{}\" is invalid.",
                id
            )
        }
        Ok(Self(id.to_string()))
    }

    pub fn id(&self) -> &str {
        &self.0
    }

    pub fn major_type(&self) -> &str {
        let slash_pos = self.0.find('/').expect("Mimetype should contain a '/'");
        &self.0[0..slash_pos]
    }

    pub fn minor_type(&self) -> &str {
        let slash_pos = self.0.find('/').expect("Mimetype should contain a '/'");
        &self.0[slash_pos + 1..self.0.len()]
    }

    /// True if the minor type is `*`, e.g., `image/*`
    pub fn is_minor_type_wildcard(&self) -> bool {
        self.minor_type() == "*"
    }

    /// True if this MimeType has a wildcard minor type, and the passed-in MimeType matches
    pub fn wildcard_match(&self, other: &MimeType) -> bool {
        if self.is_minor_type_wildcard() {
            self.major_type() == other.major_type()
        } else {
            false
        }
    }
}

impl Display for MimeType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

///////////////////////////////////////////////////////////////////////////////////////////////////////////////////////

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mime_type_parse() -> anyhow::Result<()> {
        assert!(MimeType::parse("foo/bar").is_ok());
        assert!(MimeType::parse("foobar").is_err());
        assert!(MimeType::parse("foo/bar/baz").is_err());

        let mime = MimeType::parse("foo/bar")?;
        assert_eq!(mime.major_type(), "foo");
        assert_eq!(mime.minor_type(), "bar");

        Ok(())
    }

    #[test]
    fn mime_type_components_work() -> anyhow::Result<()> {
        let image_png = MimeType::parse("image/png")?;
        assert_eq!(image_png.major_type(), "image");
        assert_eq!(image_png.minor_type(), "png");
        assert!(!image_png.is_minor_type_wildcard());

        let image_star = MimeType::parse("image/*")?;
        assert_eq!(image_star.major_type(), "image");
        assert_eq!(image_star.minor_type(), "*");
        assert!(image_star.is_minor_type_wildcard());

        assert!(image_star.wildcard_match(&image_png));
        assert!(!image_png.wildcard_match(&image_star));

        Ok(())
    }
}
