use std::cell::RefCell;

use adw::prelude::*;
use adw::subclass::prelude::*;
use glib::{ParamSpec, Properties, Value};
use gtk::{gio, glib};

use crate::application_entry::ApplicationEntry;

#[derive(Properties, Default)]
#[properties(wrapper_type = super::MimeTypeEntry)]
pub struct MimeTypeEntry {
    #[property(get, set)]
    pub mime_type: RefCell<String>,

    #[property(get, set)]
    pub supported_applications: RefCell<gio::ListStore>,
}

// The central trait for subclassing a GObject
#[glib::object_subclass]
impl ObjectSubclass for MimeTypeEntry {
    const NAME: &'static str = "MimeAssocGUI_MimeTypeEntry";
    type Type = super::MimeTypeEntry;
}

// Trait shared by all GObjects
impl ObjectImpl for MimeTypeEntry {
    fn constructed(&self) {
        Self::parent_constructed(&self);

        self.supported_applications
            .replace(gio::ListStore::new(ApplicationEntry::static_type()));
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
