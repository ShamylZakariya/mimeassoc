mod imp;

use std::cell::RefCell;
use std::rc::Rc;

use adw::subclass::prelude::*;
use glib::Object;
use gtk::glib;
use mimeassoc::*;

use crate::model::*;

glib::wrapper! {
    pub struct ApplicationEntry(ObjectSubclass<imp::ApplicationEntry>);
}

impl ApplicationEntry {
    pub fn new(desktop_entry_id: &DesktopEntryId, stores: Rc<RefCell<MimeAssocStores>>) -> Self {
        let entry: ApplicationEntry = Object::builder()
            .property("id", desktop_entry_id.to_string())
            .build();

        entry.imp().stores.set(stores).unwrap();
        entry
    }

    /// Get DesktopEntryId for this entry for use with a `DesktopEntryStore`
    pub fn desktop_entry_id(&self) -> DesktopEntryId {
        let desktop_entry_id_raw = self.id();
        DesktopEntryId::parse(&desktop_entry_id_raw).unwrap()
    }

    /// Get the associated `DesktopEntry`
    pub fn desktop_entry(&self) -> Option<DesktopEntry> {
        let id = self.desktop_entry_id();
        let stores = self.stores();
        let desktop_entry_store = &stores.borrow().desktop_entry_store;

        desktop_entry_store.get_desktop_entry(&id).cloned()
    }

    fn stores(&self) -> Rc<RefCell<MimeAssocStores>> {
        self.imp()
            .stores
            .get()
            .expect("Expect `setup_models()` to be called before calling `stores()`.")
            .clone()
    }
}
