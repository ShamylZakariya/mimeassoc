use mimeassoc::*;

pub struct Strings;

impl Strings {
    /// Message shown in the MainWindowPage::MimeTypes view, beneath the application
    /// listing when only one application is installed supporting this mimetype
    /// and it is the system default.
    pub fn single_default_application_info_message(
        desktop_entry: &DesktopEntry,
        mime_type: &MimeType,
    ) -> String {
        format!("{} is the system default handler for {}, and no other application handling it is installed", desktop_entry.name().unwrap_or(desktop_entry.id().to_string().as_str()), mime_type)
    }

    /// Message shown in the MainWindowPage::Applications view, in the right-hand mime types
    /// listing, when a mime type is bound to the selected application at the system level
    /// and as such cannot be disabled by the user.
    pub fn application_is_system_default_handler_for_mimetype(
        desktop_entry: &DesktopEntry,
        mime_type: &MimeType,
    ) -> String {
        format!(
            "{} is the system default handler for {}",
            desktop_entry.name().unwrap_or(desktop_entry.id().id()),
            mime_type
        )
    }
}
