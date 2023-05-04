use std::path::{Path, PathBuf};

#[allow(dead_code)]
pub mod desktop_entry;

#[allow(dead_code)]
pub mod mime_type;

/// Return a vector of paths to the application dirs for the user.
pub fn desktop_entry_dirs() -> anyhow::Result<Vec<PathBuf>> {
    let xdg_data_dirs = std::env::var("XDG_DATA_DIRS")?
        .split(':')
        .map(PathBuf::from)
        .collect::<Vec<_>>();
    let user_data_dir = PathBuf::from(std::env::var("HOME")?).join(".local/share");
    let mut data_dirs = vec![user_data_dir];
    data_dirs.extend(xdg_data_dirs);

    Ok(data_dirs
        .iter()
        .map(|d| d.join("applications"))
        .filter(|d| is_valid_desktop_entry_dir(d))
        .collect::<Vec<_>>())
}

fn is_valid_desktop_entry_dir<P>(path: P) -> bool
where
    P: AsRef<Path>,
{
    let path = path.as_ref();
    if path.exists() && path.is_dir() {
        // verify this dir contains .desktop entries
        if let Ok(contents) = std::fs::read_dir(path) {
            for file in contents.flatten() {
                if has_extension(file.path(), "desktop") {
                    return true;
                }
            }
        }
    }

    false
}

/// Return a vector of paths to the mimeapps.list files for the user
/// in system order
pub fn mimeapps_lists_paths() -> anyhow::Result<Vec<PathBuf>> {
    // ~/.config/mimeapps.list followed by the desktop_entry_dirs (with mimeapps.list appended), adding
    // each that exists.

    let mut dirs = vec![PathBuf::from(std::env::var("HOME")?).join(".config")];
    dirs.extend(desktop_entry_dirs()?);

    let mut mimeapps_list_paths = vec![];
    for dir in dirs.iter() {
        let mimeapps_list_path = dir.join("mimeapps.list");
        if mimeapps_list_path.exists() && mimeapps_list_path.is_file() {
            mimeapps_list_paths.push(mimeapps_list_path);
        }
    }

    Ok(mimeapps_list_paths)
}

fn has_extension<P>(path: P, extension: &str) -> bool
where
    P: AsRef<Path>,
{
    let path = path.as_ref();
    if let Some(e) = path.extension() {
        if let Some(e) = e.to_str() {
            return e == extension;
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::{
        desktop_entry::{DesktopEntries, DesktopEntry, DesktopEntryId},
        mime_type::{MimeAssociations, MimeType},
        *,
    };

    #[test]
    fn desktop_entry_dirs_returns_nonempty_vec() -> anyhow::Result<()> {
        let dirs = desktop_entry_dirs()?;
        assert!(!dirs.is_empty());

        for dir in dirs.iter() {
            assert!(dir.exists());
            assert!(dir.is_dir());
            assert!(!dir.is_file());

            let contents = std::fs::read_dir(dir)?.flatten();
            let mut count = 0;
            for file in contents {
                if has_extension(file.path(), "desktop") {
                    count += 1;
                }
            }
            assert!(
                count > 0,
                "Expected to find .desktop files in all dirs returned by desktop_entry_dirs()"
            );
        }

        Ok(())
    }

    #[test]
    fn mimeapps_lists_returns_nonempty_vec() -> anyhow::Result<()> {
        let paths = mimeapps_lists_paths()?;
        assert!(!paths.is_empty());

        for path in paths.iter() {
            assert!(path.exists());
            assert!(path.is_file());
            assert!(!path.is_dir());
        }

        Ok(())
    }

    fn path(p: &str) -> PathBuf {
        let cwd = std::env::current_dir().unwrap();
        cwd.join(p)
    }

    fn test_mimeapps_lists_paths() -> Vec<(PathBuf, bool)> {
        vec![
            (path("test-data/config/mimeapps.list"), true),
            (
                path("test-data/usr/share/applications/gnome-mimeapps.list"),
                false,
            ),
            (
                path("test-data/usr/share/applications/mimeapps.list"),
                false,
            ),
        ]
    }

    fn test_desktop_entry_dirs() -> Vec<PathBuf> {
        vec![
            path("test-data/local/share/applications"),
            path("test-data/usr/share/applications"),
        ]
    }

    fn default_application_for_mime_type<'a>(
        mime_type: &MimeType,
        mime_associations: &'a MimeAssociations,
        desktop_entries: &'a DesktopEntries,
    ) -> Option<&'a DesktopEntry> {
        if let Some(desktop_entry_id) = mime_associations.default_application_for(&mime_type) {
            if let Some(desktop_entry) = desktop_entries.get_desktop_entry(desktop_entry_id) {
                return Some(desktop_entry);
            }
        }
        None
    }

    #[test]
    fn makes_correct_mime_to_desktop_associations() -> anyhow::Result<()> {
        let mime_associations = MimeAssociations::load(&test_mimeapps_lists_paths())?;
        let desktop_entries = DesktopEntries::load(&test_desktop_entry_dirs())?;

        let text_plain = MimeType::parse("text/plain")?;
        let audio_m4a = MimeType::parse("audio/m4a")?;
        let image_bmp = MimeType::parse("image/bmp")?;
        let image_tiff = MimeType::parse("image/tiff")?;
        let image_pdf = MimeType::parse("image/pdf")?;

        // totem is assigned to audio/m4a in /usr/share/applications/mimeapps.list
        let totem_id = DesktopEntryId::parse("org.gnome.Totem.desktop")?;
        // text editor is assigned to text/plain in /usr/share/applications/gnome-mimeapps.list, overriding inherited
        let text_editor = DesktopEntryId::parse("org.gnome.TextEditor.desktop")?;
        // photopea is assigned in config/mimeapps.list overriding inherited
        let photopea_id = DesktopEntryId::parse("photopea.desktop")?;
        // evince is assigned to image/tiff in /usr/share/applications/mimeapps.list, and image/pdf in config/mimeapps.list
        let evince_id = DesktopEntryId::parse("org.gnome.Evince.desktop")?;

        // assert mime -> desktop entry ids work as expected
        assert_eq!(
            mime_associations.default_application_for(&text_plain),
            Some(&text_editor)
        );
        assert_eq!(
            mime_associations.default_application_for(&image_bmp),
            Some(&photopea_id)
        );
        assert_eq!(
            mime_associations.default_application_for(&audio_m4a),
            Some(&totem_id)
        );
        assert_eq!(
            mime_associations.default_application_for(&image_tiff),
            Some(&evince_id)
        );

        assert_eq!(
            mime_associations.default_application_for(&image_pdf),
            Some(&evince_id)
        );

        // now look these up in desktop entries and verify
        assert_eq!(
            default_application_for_mime_type(&text_plain, &mime_associations, &desktop_entries)
                .unwrap()
                .id(),
            &text_editor
        );
        assert_eq!(
            default_application_for_mime_type(&image_bmp, &mime_associations, &desktop_entries)
                .unwrap()
                .id(),
            &photopea_id
        );
        assert_eq!(
            default_application_for_mime_type(&audio_m4a, &mime_associations, &desktop_entries)
                .unwrap()
                .id(),
            &totem_id
        );
        assert_eq!(
            default_application_for_mime_type(&image_tiff, &mime_associations, &desktop_entries)
                .unwrap()
                .id(),
            &evince_id
        );
        assert_eq!(
            default_application_for_mime_type(&image_pdf, &mime_associations, &desktop_entries)
                .unwrap()
                .id(),
            &evince_id
        );

        Ok(())
    }
}
