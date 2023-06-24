mod imp;

use glib::Object;
use gtk::glib;
use mimeassoc::desktop_entry::{DesktopEntry, DesktopEntryId, DesktopEntryStore};

glib::wrapper! {
    pub struct ApplicationEntry(ObjectSubclass<imp::ApplicationEntry>);
}

impl ApplicationEntry {
    pub fn new(desktop_entry_id: &DesktopEntryId) -> Self {
        Object::builder()
            .property("id", desktop_entry_id.to_string())
            .build()
    }

    /// Get the represented MimeType for this entry
    pub fn desktop_entry_id(&self) -> DesktopEntryId {
        let desktop_entry_id_raw = self.id();
        DesktopEntryId::parse(&desktop_entry_id_raw).unwrap()
    }

    pub fn desktop_entry<'a>(&self, db: &'a DesktopEntryStore) -> Option<&'a DesktopEntry> {
        let id = self.desktop_entry_id();
        db.get_desktop_entry(&id)
    }
}
