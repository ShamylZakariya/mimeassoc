mod imp;

use adw::subclass::prelude::*;
use glib::Object;
use gtk::glib;
use mimeassoc::desktop_entry::{DesktopEntries, DesktopEntry, DesktopEntryId};

glib::wrapper! {
    pub struct ApplicationEntry(ObjectSubclass<imp::ApplicationEntry>);
}

impl ApplicationEntry {
    pub fn new(desktop_entry: &DesktopEntryId) -> Self {
        // TODO: See MimeTypeEntry::new() - same problem here with property binding
        let entry: ApplicationEntry = Object::builder().build();
        entry.set_desktop_entry_id(desktop_entry.to_string());
        entry
    }

    /// Get the represented MimeType for this entry
    pub fn get_desktop_entry_id(&self) -> DesktopEntryId {
        let desktop_entry_string = self.imp().desktop_entry_id.borrow();
        DesktopEntryId::parse(&desktop_entry_string).unwrap()
    }

    pub fn get_desktop_entry<'a>(&self, db: &'a DesktopEntries) -> Option<&'a DesktopEntry> {
        let id = self.get_desktop_entry_id();
        db.get_desktop_entry(&id)
    }
}
