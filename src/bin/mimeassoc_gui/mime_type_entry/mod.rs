mod imp;

use adw::subclass::prelude::*;
use glib::Object;
use gtk::{gio, glib};

use mimeassoc::mime_type::MimeType;
use crate::{application_entry::ApplicationEntry, components::Components};

glib::wrapper! {
    pub struct MimeTypeEntry(ObjectSubclass<imp::MimeTypeEntry>);
}

impl MimeTypeEntry {
    pub fn new(mime_type: &MimeType, components: &Components) -> Self {
        // TODO: why does Object::builder().property("mime_type", mime_type.to_string()).build() crash???
        // I get an exception that "mime_type" property isn't found on MimeTypeEntry
        // Consider identifying and fixing this, or just store an actual RefCell<MimeType>
        // since without properties working correctly, we can't use property binding anyway!

        let entry: MimeTypeEntry = Object::builder().build();
        entry.set_mime_type(mime_type.to_string());

        // Populate supported_applications
        let supported_applications = components
            .app_db
            .get_desktop_entries_for_mimetype(mime_type)
            .iter()
            .map(|de| ApplicationEntry::new(de.id()))
            .collect::<Vec<_>>();

        entry
            .get_supported_application_entries()
            .extend_from_slice(&supported_applications);

        entry
    }

    /// Get the applications which can open this mime type
    pub fn get_supported_application_entries(&self) -> gio::ListStore {
        self.imp().supported_applications.borrow().clone()
    }

    pub fn get_mime_type(&self) -> MimeType {
        let mime_type_string = self.imp().mime_type.borrow();
        MimeType::parse(&mime_type_string).unwrap()
    }
}
