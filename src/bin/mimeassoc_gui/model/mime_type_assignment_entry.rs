use std::cell::RefCell;
use std::rc::Rc;

use adw::prelude::*;
use adw::subclass::prelude::*;
use glib::Object;
use gtk::glib::{self, *};

use crate::model::*;
use mimeassoc::*;

mod imp {
    use super::*;
    use std::cell::OnceCell;

    #[derive(Properties, Default)]
    #[properties(wrapper_type = mime_type_assignment_entry::MimeTypeAssignmentEntry)]
    pub struct MimeTypeAssignmentEntry {
        pub stores: OnceCell<Rc<RefCell<MimeAssocStores>>>,

        #[property(get, set)]
        pub id: RefCell<String>,

        #[property(get, set)]
        pub assigned: RefCell<bool>,
    }

    impl MimeTypeAssignmentEntry {
        fn ready(&self) -> bool {
            self.stores.get().is_some()
        }
    }

    // The central trait for subclassing a GObject
    #[glib::object_subclass]
    impl ObjectSubclass for MimeTypeAssignmentEntry {
        const NAME: &'static str = "MimeTypeAssignmentEntry";
        type Type = mime_type_assignment_entry::MimeTypeAssignmentEntry;
    }

    // Trait shared by all GObjects
    impl ObjectImpl for MimeTypeAssignmentEntry {
        fn constructed(&self) {
            Self::parent_constructed(self);
        }

        fn properties() -> &'static [ParamSpec] {
            Self::derived_properties()
        }

        fn set_property(&self, id: usize, value: &Value, pspec: &ParamSpec) {
            if self.ready() {
                println!(
                    "MimeTypeAssignmentEntry[{}]::set_property id:{} value: {:?} pspec: {:?}",
                    self.id.borrow(),
                    id,
                    value,
                    pspec
                );
            }

            // id 1 is "id", 2 is "assigned", carrying "gbool", makes sense

            self.derived_set_property(id, value, pspec)
        }

        fn property(&self, id: usize, pspec: &ParamSpec) -> Value {
            self.derived_property(id, pspec)
        }
    }
}

glib::wrapper! {
    pub struct MimeTypeAssignmentEntry(ObjectSubclass<imp::MimeTypeAssignmentEntry>);
}

impl MimeTypeAssignmentEntry {
    pub fn new(
        mime_type: &MimeType,
        is_handled: bool,
        stores: Rc<RefCell<MimeAssocStores>>,
    ) -> Self {
        let entry: MimeTypeAssignmentEntry = Object::builder()
            .property("id", mime_type.to_string())
            .property("assigned", is_handled)
            .build();

        entry.imp().stores.set(stores).unwrap();
        entry
    }

    pub fn mime_type(&self) -> MimeType {
        let mime_type_string = self.id();
        MimeType::parse(&mime_type_string).unwrap()
    }

    fn _stores(&self) -> Rc<RefCell<stores::MimeAssocStores>> {
        self.imp()
            .stores
            .get()
            .expect("Expect `setup_models()` to be called before calling `stores()`.")
            .clone()
    }
}
