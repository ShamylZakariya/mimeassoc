#[derive(Debug, PartialEq, Eq, Hash, Clone)]
pub struct AppId {
    desktop_entry: String,
}

impl AppId {
    pub fn parse(desktop_entry: &str) -> anyhow::Result<AppId> {
        if desktop_entry.ends_with(".desktop") {
            Ok(AppId {
                desktop_entry: desktop_entry.to_string(),
            })
        } else {
            anyhow::bail!(
                "id: \"{}\" not a valid Gnome .desktop file name",
                desktop_entry
            )
        }
    }

    pub fn id(&self) -> &str {
        let desktop_idx = self.desktop_entry.find(".desktop").unwrap();
        &self.desktop_entry[0..desktop_idx]
    }
}

//////////////////////////////////////////////////////////////////////////////////////////////////////////////////////

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_valid_app_ids() {
        assert!(AppId::parse("org.foo.Bar.desktop").is_ok());
        assert!(AppId::parse("Baz.desktop").is_ok());
    }

    #[test]
    fn rejects_invalid_app_ids() {
        assert!(AppId::parse("org.foo.Bar").is_err());
        assert!(AppId::parse("Baz").is_err());
    }

    #[test]
    fn parses_app_id() -> anyhow::Result<()> {
        assert_eq!(AppId::parse("org.foo.Bar.desktop")?.id(), "org.foo.Bar");
        assert_eq!(AppId::parse("Baz.desktop")?.id(), "Baz");

        Ok(())
    }
}
