mod imp;

use std::cell::RefCell;
use std::rc::Rc;

use adw::prelude::*;
use adw::subclass::prelude::*;
use glib::Object;
use gtk::glib;

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

    pub fn bind(&self, application_entry: &ApplicationEntry, stores: Rc<RefCell<MimeAssocStores>>) {
        self.set_spacing(12);

        self.bind_labels(application_entry, stores.clone());
    }

    pub fn unbind(&self) {}

    fn bind_labels(
        &self,
        application_entry: &ApplicationEntry,
        stores: Rc<RefCell<MimeAssocStores>>,
    ) {
        let desktop_entry_store = &stores.borrow().desktop_entry_store;
        let desktop_entry = &application_entry
            .desktop_entry(&desktop_entry_store)
            .expect("Expected a DesktopEntry");
        let content_label = self.imp().name_label.get();
        content_label.set_text(desktop_entry.name().unwrap_or("<Unnamed Application>"));

        let info_label = self.imp().info_label.get();
        info_label.set_text(&desktop_entry.id().to_string());
    }
}
