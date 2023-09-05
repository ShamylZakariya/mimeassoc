use std::cell::RefCell;
use std::rc::Rc;

use adw::subclass::prelude::*;
use glib::Object;
use gtk::glib::{self, *};

use crate::model::*;
use mimeassoc::*;

mod imp {
    use super::*;

    use std::cell::OnceCell;

    #[derive(Properties, Default)]
    #[properties(wrapper_type = super::MimeTypeEntry)]
    pub struct MimeTypeEntry {
        pub stores: OnceCell<Rc<RefCell<Stores>>>,

        #[property(get, set)]
        pub id: RefCell<String>,
    }

    // The central trait for subclassing a GObject
    #[glib::object_subclass]
    impl ObjectSubclass for MimeTypeEntry {
        const NAME: &'static str = "MimeTypeEntry";
        type Type = super::MimeTypeEntry;
    }

    // Trait shared by all GObjects
    impl ObjectImpl for MimeTypeEntry {
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
    pub struct MimeTypeEntry(ObjectSubclass<imp::MimeTypeEntry>);
}

impl MimeTypeEntry {
    pub fn new(mime_type: &MimeType, stores: Rc<RefCell<Stores>>) -> Self {
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

    /// Creates and populates a ListStore of ApplicationEntry representing
    /// all the applications which support opening this MimeType
    pub fn supported_application_entries(&self) -> gtk::gio::ListStore {
        let mime_type = self.mime_type();
        let stores = self.stores();
        let borrowed_stores = stores.borrow();
        let desktop_entry_store = borrowed_stores.desktop_entry_store();

        let mut desktop_entries = desktop_entry_store.get_desktop_entries_for_mimetype(&mime_type);
        desktop_entries.sort_by(|a, b| a.cmp_by_name_alpha_inensitive(b));

        let application_entries = desktop_entries
            .iter()
            .map(|de| ApplicationEntry::new(de.id(), stores.clone()))
            .collect::<Vec<_>>();

        let store = gtk::gio::ListStore::with_type(ApplicationEntry::static_type());
        store.extend_from_slice(&application_entries);

        store
    }

    fn stores(&self) -> Rc<RefCell<stores::Stores>> {
        self.imp()
            .stores
            .get()
            .expect("Expect `setup_models()` to be called before calling `stores()`.")
            .clone()
    }
}
