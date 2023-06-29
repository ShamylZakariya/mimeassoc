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

    /// Get the `DesktopEntryId` for thie `ApplicatioEntry`
    pub fn desktop_entry_id(&self) -> DesktopEntryId {
        let desktop_entry_id_raw = self.id();
        DesktopEntryId::parse(&desktop_entry_id_raw).unwrap()
    }

    /// Get the `DesktopEntry` for thie `ApplicatioEntry`
    pub fn desktop_entry(&self) -> DesktopEntry {
        let id = self.desktop_entry_id();
        let stores = self.stores();
        let desktop_entry_store = &stores.borrow().desktop_entry_store;

        desktop_entry_store.get_desktop_entry(&id).cloned().expect(
            "Expect all ApplicationEntry instances to be backed by a valid DesktopEntry instance",
        )
    }

    /// If `ApplicationEntry::supported_mime_types` will be used (as in the Applications Pane) this
    /// needs to be called to populate that list
    pub fn update_supported_mime_types(&self) {
        let desktop_entry = self.desktop_entry();
        let mime_types = desktop_entry.mime_types();
        let stores = self.stores();
        let mime_db = &stores.borrow().mime_associations_store;
        let new_supported_mime_types = mime_types
            .iter()
            .map(|mt| {
                let assigned = mime_db.assigned_application_for(mt) == Some(desktop_entry.id());
                MimeTypeAssignmentEntry::new(mt, assigned, stores.clone())
            })
            .collect::<Vec<_>>();

        let supported_mime_types = self.mime_type_assignments();
        supported_mime_types.remove_all();
        supported_mime_types.extend_from_slice(&new_supported_mime_types);
    }

    fn stores(&self) -> Rc<RefCell<MimeAssocStores>> {
        self.imp()
            .stores
            .get()
            .expect("Expect `setup_models()` to be called before calling `stores()`.")
            .clone()
    }
}
