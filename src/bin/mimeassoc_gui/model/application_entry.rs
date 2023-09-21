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
        pub stores: OnceCell<Rc<RefCell<Stores>>>,

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
    pub fn new(desktop_entry_id: &DesktopEntryId, stores: Rc<RefCell<Stores>>) -> Self {
        let entry: ApplicationEntry = Object::builder()
            .property("id", desktop_entry_id.to_string())
            .build();

        entry.imp().stores.set(stores).unwrap();
        entry
    }

    /// Create an empty ApplicationEntry. This is used to represent the "None" selection in lists.
    pub fn none() -> Self {
        Object::builder().build()
    }

    /// Get the `DesktopEntryId` for thie `ApplicatioEntry`
    pub fn desktop_entry_id(&self) -> Option<DesktopEntryId> {
        let desktop_entry_id_raw = self.id();
        DesktopEntryId::parse(&desktop_entry_id_raw).ok()
    }

    /// Get the `DesktopEntry` for thie `ApplicatioEntry`
    pub fn desktop_entry(&self) -> Option<DesktopEntry> {
        if let Some(id) = self.desktop_entry_id() {
            let stores = self.stores();
            let stores = stores.borrow();
            let desktop_entry_store = stores.desktop_entry_store();

            desktop_entry_store.find_desktop_entry_with_id(&id).cloned()
        } else {
            None
        }
    }

    /// Creates and populates a ListStore of MimeTypeEntry representing
    /// all the mime types which this application reports to handle
    pub fn mime_type_assignments(&self) -> gio::ListStore {
        let mime_type_assignments_store = gio::ListStore::with_type(MimeTypeEntry::static_type());

        if let Some(desktop_entry) = self.desktop_entry() {
            let mut supported_mime_types = desktop_entry.mime_types().clone();
            supported_mime_types.sort();

            let stores = self.stores();
            let supported_mime_type_entries = supported_mime_types
                .iter()
                .map(|mt| MimeTypeEntry::new(mt, stores.clone()))
                .collect::<Vec<_>>();

            mime_type_assignments_store.extend_from_slice(&supported_mime_type_entries);
        }

        mime_type_assignments_store
    }

    fn stores(&self) -> Rc<RefCell<Stores>> {
        self.imp()
            .stores
            .get()
            .expect("Expect `setup_models()` to be called before calling `stores()`.")
            .clone()
    }
}
