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
    use super::*;

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
}
