use mimeassoc::*;

pub struct Strings;

impl Strings {
    pub fn single_default_application_info_message(
        desktop_entry: &DesktopEntry,
        mime_type: &MimeType,
    ) -> String {
        format!("{} is the system default handler for {}, and no other application handling it is installed", desktop_entry.name().unwrap_or(desktop_entry.id().to_string().as_str()), mime_type)
    }
}
