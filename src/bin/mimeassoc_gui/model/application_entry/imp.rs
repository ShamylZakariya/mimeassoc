use std::{
    cell::{OnceCell, RefCell},
    rc::Rc,
};

use adw::prelude::*;
use adw::subclass::prelude::*;
use glib::{ParamSpec, Properties, Value};
use gtk::{gio, glib};

use crate::model::*;

#[derive(Properties, Default)]
#[properties(wrapper_type = super::ApplicationEntry)]
pub struct ApplicationEntry {
    pub stores: OnceCell<Rc<RefCell<MimeAssocStores>>>,

    #[property(get, set)]
    pub id: RefCell<String>,

    #[property(get, set)]
    pub mime_type_assignments: RefCell<gio::ListStore>,
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
        self.mime_type_assignments
            .replace(gio::ListStore::new(MimeTypeAssignmentEntry::static_type()));
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
