use std::cell::RefCell;

use adw::prelude::*;
use adw::subclass::prelude::*;
use glib::{ParamSpec, Properties, Value};
use gtk::{gio, glib};

#[derive(Properties, Default)]
#[properties(wrapper_type = super::ApplicationEntry)]
pub struct ApplicationEntry {
    #[property(get, set)]
    pub desktop_entry_id: RefCell<String>,
}

// The central trait for subclassing a GObject
#[glib::object_subclass]
impl ObjectSubclass for ApplicationEntry {
    const NAME: &'static str = "MimeAssocGUI_ApplicationEntry";
    type Type = super::ApplicationEntry;
}

// Trait shared by all GObjects
impl ObjectImpl for ApplicationEntry {
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