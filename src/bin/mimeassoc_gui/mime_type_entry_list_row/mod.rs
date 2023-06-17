mod imp;

use adw::prelude::*;
use adw::subclass::prelude::*;
use glib::Object;
use gtk::glib;

use crate::application_entry::ApplicationEntry;
use crate::mime_type_entry::MimeTypeEntry;

glib::wrapper! {
    pub struct MimeTypeEntryListRow(ObjectSubclass<imp::MimeTypeEntryListRow>)
    @extends gtk::Box, gtk::Widget,
    @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget, gtk::Orientable;
}

impl Default for MimeTypeEntryListRow {
    fn default() -> Self {
        Self::new()
    }
}

impl MimeTypeEntryListRow {
    pub fn new() -> Self {
        Object::builder().build()
    }

    pub fn bind(&self, mime_type_entry: &MimeTypeEntry) {
        self.set_spacing(12);

        let content_label = self.imp().content_label.get();
        let applications_combobox = self.imp().applications_combo_box.get();

        content_label.set_text(&mime_type_entry.get_mime_type().to_string());

        let applications = mime_type_entry.get_supported_application_entries();

        applications_combobox.remove_all();
        for i in 0_u32..applications.n_items() {
            if let Some(entry) = applications.item(i) {
                let entry = entry
                    .downcast_ref::<ApplicationEntry>()
                    .expect("Expect applications list store to contain ApplicationEntry");
                let desktop_entry = entry.get_desktop_entry_id();
                applications_combobox.append(None, &desktop_entry.to_string());
            }
        }
    }

    pub fn unbind(&self) {
        // nothing yet, but would make sense to disconnect callbacks
    }
}
