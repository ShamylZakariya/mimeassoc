mod imp;

use glib::Object;
use gtk::glib;
use mimeassoc::*;

glib::wrapper! {
    pub struct ApplicationEntry(ObjectSubclass<imp::ApplicationEntry>);
}

impl ApplicationEntry {
    pub fn new(desktop_entry_id: &DesktopEntryId) -> Self {
        Object::builder()
            .property("id", desktop_entry_id.to_string())
            .build()
    }

    /// Get DesktopEntryId for this entry for use with a `DesktopEntryStore`
    pub fn desktop_entry_id(&self) -> DesktopEntryId {
        let desktop_entry_id_raw = self.id();
        DesktopEntryId::parse(&desktop_entry_id_raw).unwrap()
    }

    /// Get the associated `DesktopEntry`
    pub fn desktop_entry<'a>(&self, db: &'a DesktopEntryStore) -> Option<&'a DesktopEntry> {
        let id = self.desktop_entry_id();
        db.get_desktop_entry(&id)
    }
}
