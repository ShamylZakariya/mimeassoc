use mimeassoc::mime_type::MimeType;

mod imp;

use adw::subclass::prelude::*;
use glib::Object;
use gtk::glib;

glib::wrapper! {
    pub struct MimeTypeEntry(ObjectSubclass<imp::MimeTypeEntry>);
}

impl MimeTypeEntry {
    pub fn new(mime_type: &MimeType) -> Self {
        // why does Object::builder().property("mime_type", mime_type.to_string()).build() crash???
        // I get an exception that "mime_type" property isn't found on MimeTypeEntry

        let entry: MimeTypeEntry = Object::builder().build();
        entry.set_mime_type(mime_type.to_string());
        entry
    }

    /// Get the represented MimeType for this entry
    pub fn _get_mime_type(&self) -> MimeType {
        let mime_type_string = self.imp().mime_type.borrow();
        MimeType::parse(&mime_type_string).unwrap()
    }
}
