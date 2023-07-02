mod imp;

use adw::subclass::prelude::*;
use adw::*;
use glib::Object;
use gtk::glib::{self};

use crate::model::*;

glib::wrapper! {
    pub struct ApplicationEntryListRow(ObjectSubclass<imp::ApplicationEntryListRow>)
    @extends gtk::Box, gtk::Widget,
    @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget, gtk::Orientable;
}

impl Default for ApplicationEntryListRow {
    fn default() -> Self {
        Self::new()
    }
}

impl ApplicationEntryListRow {
    pub fn new() -> Self {
        Object::builder().build()
    }

    pub fn bind(&self, application_entry: &ApplicationEntry) {
        let desktop_entry = &application_entry.desktop_entry();
        let content_label = self.imp().name_label.get();
        content_label.set_text(desktop_entry.name().unwrap_or("<Unnamed Application>"));

        let info_label = self.imp().info_label.get();
        info_label.set_text(&desktop_entry.id().to_string());
    }

    pub fn unbind(&self) {}
}
