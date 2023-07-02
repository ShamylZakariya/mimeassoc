use std::{
    cell::{OnceCell, RefCell},
    rc::Rc,
};

use adw::prelude::*;
use adw::subclass::prelude::*;
use glib::{ParamSpec, Properties, Value};
use gtk::glib;

use crate::model::*;

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
