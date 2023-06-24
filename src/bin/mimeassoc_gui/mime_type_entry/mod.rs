mod imp;

use adw::subclass::prelude::*;
use glib::Object;
use gtk::{gio, glib};

use crate::{application_entry::ApplicationEntry, stores::MimeAssocStores};
use mimeassoc::mime_type::MimeType;

glib::wrapper! {
    pub struct MimeTypeEntry(ObjectSubclass<imp::MimeTypeEntry>);
}

impl MimeTypeEntry {
    pub fn new(mime_type: &MimeType, stores: &MimeAssocStores) -> Self {
        let entry: MimeTypeEntry = Object::builder()
            .property("id", mime_type.to_string())
            .build();

        let supported_applications = stores
            .desktop_entry_store
            .get_desktop_entries_for_mimetype(mime_type)
            .iter()
            .map(|de| ApplicationEntry::new(de.id()))
            .collect::<Vec<_>>();

        entry
            .imp()
            .supported_application_entries
            .borrow()
            .extend_from_slice(&supported_applications);

        entry
    }

    pub fn mime_type(&self) -> MimeType {
        let mime_type_string = self.id();
        MimeType::parse(&mime_type_string).unwrap()
    }
}
