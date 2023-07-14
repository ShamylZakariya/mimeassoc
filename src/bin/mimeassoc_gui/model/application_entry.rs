use std::cell::RefCell;
use std::rc::Rc;

use adw::subclass::prelude::*;
use glib::Object;
use gtk::{gio, glib, prelude::*};
use mimeassoc::*;

use crate::model::*;

mod imp {
    use super::*;
    use std::{
        cell::{OnceCell, RefCell},
        rc::Rc,
    };

    use glib::{ParamSpec, Properties, Value};
    use gtk::glib;

    #[derive(Properties, Default)]
    #[properties(wrapper_type = super::ApplicationEntry)]
    pub struct ApplicationEntry {
        pub stores: OnceCell<Rc<RefCell<MimeAssocStores>>>,

        #[property(get, set)]
        pub id: RefCell<String>,
    }

    // The central trait for subclassing a GObject
    #[glib::object_subclass]
    impl ObjectSubclass for ApplicationEntry {
        const NAME: &'static str = "ApplicationEntry";
        type Type = super::ApplicationEntry;
    }

    // Trait shared by all GObjects
    impl ObjectImpl for ApplicationEntry {
        fn constructed(&self) {
            Self::parent_constructed(self);
        }

        fn properties() -> &'static [ParamSpec] {
            Self::derived_properties()
        }

        fn set_property(&self, id: usize, value: &Value, pspec: &ParamSpec) {
            self.derived_set_property(id, value, pspec)
        }

        fn property(&self, id: usize, pspec: &ParamSpec) -> Value {
            self.derived_property(id, pspec)
        }
    }
}

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

    /// Get the mime type assignments to this application
    pub fn mime_type_assignments(&self) -> gio::ListStore {
        let desktop_entry = self.desktop_entry();
        let mime_types = desktop_entry.mime_types();
        let stores = self.stores();
        let mime_db = &stores.borrow().mime_associations_store;
        let supported_mime_types = mime_types
            .iter()
            .map(|mt| {
                let assigned = mime_db.assigned_application_for(mt) == Some(desktop_entry.id());
                MimeTypeAssignmentEntry::new(mt, assigned, stores.clone())
            })
            .collect::<Vec<_>>();

        let store = gio::ListStore::new(MimeTypeAssignmentEntry::static_type());
        store.extend_from_slice(&supported_mime_types);

        store
    }

    fn stores(&self) -> Rc<RefCell<MimeAssocStores>> {
        self.imp()
            .stores
            .get()
            .expect("Expect `setup_models()` to be called before calling `stores()`.")
            .clone()
    }
}
