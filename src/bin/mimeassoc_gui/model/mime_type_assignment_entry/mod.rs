mod imp;

use std::cell::RefCell;
use std::rc::Rc;

use adw::subclass::prelude::*;
use glib::Object;
use gtk::glib;

use crate::model::*;
use mimeassoc::*;

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
