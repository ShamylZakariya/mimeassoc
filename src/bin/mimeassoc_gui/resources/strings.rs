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
        format!(
            "{} is the only available handler for {}. It is assigned by the system, and can't be unassigned.",
            desktop_entry
                .name()
                .unwrap_or(desktop_entry.id().to_string().as_str()),
            mime_type
        )
    }

    /// Message shown in the MainWindowPage::Applications view, in the right-hand mime types
    /// listing, when a mime type is bound to the selected application at the system level
    /// and as such cannot be disabled by the user.
    pub fn application_is_system_default_handler_for_mimetype_long(
        desktop_entry: &DesktopEntry,
        mime_type: &MimeType,
    ) -> String {
        format!(
            "{} is the system default handler for {}",
            desktop_entry.name().unwrap_or(desktop_entry.id().id()),
            mime_type
        )
    }

    /// Message shown in the MainWindowPage::MimeTypes view in the detail view
    /// as subtitle for an Application which is the system default handler for the
    /// selected mime type.
    pub fn application_is_system_default_handler_for_mimetype_short(
        mime_type: &MimeType,
    ) -> String {
        format!("System default handler for {}", mime_type)
    }

    pub fn assign_no_application_list_item() -> &'static str {
        "None"
    }

    // Strings for dialogs

    pub fn reset_user_default_application_assignments_dialog_title() -> &'static str {
        "Reset your application handler assignments?"
    }

    pub fn reset_user_default_application_assignments_dialog_body() -> &'static str {
        "This will clear any application assignments you have made and reset to system defaults."
    }

    pub fn reset_user_default_application_assignments_dialog_action_proceed() -> &'static str {
        "Reset to System Defaults"
    }

    pub fn prune_orphaned_application_assignments_dialog_title() -> &'static str {
        "Clear orphaned application assignments?"
    }

    pub fn prune_orphaned_application_assignments_dialog_body() -> &'static str {
        "This will remove any left-over application assignments from uninstalled applications."
    }
    pub fn prune_orphaned_application_assignments_dialog_action_proceed() -> &'static str {
        "Clear"
    }

    // Strings for Error dialog

    pub fn error_dialog_title() -> &'static str {
        "Error"
    }

    pub fn error_dialog_body(message: &str, error: &str) -> String {
        format!("{}\n{}", message, error)
    }

    pub fn error_dialog_copy_to_clipboard() -> &'static str {
        "Copy to Clipboard"
    }

    // Strings for About box

    pub fn about_window_release_notes() -> &'static str {
        r#"<ul>
    <li>Nothing to see here folks, please disperse.</li>
</ul>"#
    }

    pub fn about_window_credits_title() -> &'static str {
        "Standing on the shoulders of giants"
    }

    pub fn about_window_credits() -> Vec<&'static str> {
        vec![
            "GTK https://www.gtk.org/",
            "GNOME https://www.gnome.org/",
            "Libadwaita https://gitlab.gnome.org/GNOME/libadwaita",
            "Workbench https://github.com/sonnyp/Workbench",
            "gtk-rs https://gtk-rs.org/",
            "And many more...",
        ]
    }

    // Etc

    pub fn cancel() -> &'static str {
        "Cancel"
    }
}
