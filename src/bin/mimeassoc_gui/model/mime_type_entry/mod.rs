mod imp;

use std::cell::RefCell;
use std::rc::Rc;

use adw::subclass::prelude::*;
use glib::Object;
use gtk::glib;

use crate::model::*;
use mimeassoc::*;

glib::wrapper! {
    pub struct MimeTypeEntry(ObjectSubclass<imp::MimeTypeEntry>);
}

impl MimeTypeEntry {
    pub fn new(mime_type: &MimeType, stores: Rc<RefCell<MimeAssocStores>>) -> Self {
        let entry: MimeTypeEntry = Object::builder()
            .property("id", mime_type.to_string())
            .build();

        entry.imp().stores.set(stores).unwrap();
        entry
    }

    pub fn mime_type(&self) -> MimeType {
        let mime_type_string = self.id();
        MimeType::parse(&mime_type_string).unwrap()
    }

    /// If `MimeTypeEntry::supported_application_entries` will be used (for example, by the Mime Associations pane)
    /// this needs to be called to populate that list. Otherwise that list is empty by default.
    pub fn update_supported_applications(&self) {
        let mime_type = self.mime_type();
        let stores = self.stores();
        let desktop_entry_store = &stores.borrow().desktop_entry_store;

        let supported_applications = desktop_entry_store
            .get_desktop_entries_for_mimetype(&mime_type)
            .iter()
            .map(|de| ApplicationEntry::new(de.id(), stores.clone()))
            .collect::<Vec<_>>();

        self.imp()
            .supported_application_entries
            .borrow()
            .extend_from_slice(&supported_applications);
    }

    fn stores(&self) -> Rc<RefCell<stores::MimeAssocStores>> {
        self.imp()
            .stores
            .get()
            .expect("Expect `setup_models()` to be called before calling `stores()`.")
            .clone()
    }
}
