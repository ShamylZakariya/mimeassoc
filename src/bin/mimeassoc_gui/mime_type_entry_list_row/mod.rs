mod imp;

use std::cell::RefCell;
use std::rc::Rc;

use adw::prelude::*;
use adw::subclass::prelude::*;
use glib::Object;
use gtk::glib;

use crate::application_entry::ApplicationEntry;
use crate::components::Components;
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

    pub fn bind(&self, mime_type_entry: &MimeTypeEntry, components: Rc<RefCell<Components>>) {
        self.set_spacing(12);

        let content_label = self.imp().content_label.get();
        let applications_combobox = self.imp().applications_combo_box.get();

        content_label.set_text(&mime_type_entry.get_mime_type().to_string());

        let applications = mime_type_entry.get_supported_application_entries();
        let mime_type = &mime_type_entry.get_mime_type();

        applications_combobox.remove_all();
        for i in 0_u32..applications.n_items() {
            if let Some(entry) = applications.item(i) {
                let entry = entry
                    .downcast_ref::<ApplicationEntry>()
                    .expect("Expect applications list store to contain ApplicationEntry");
                let desktop_entry_id = entry.get_desktop_entry_id();
                let id = desktop_entry_id.to_string();
                let display_name = entry
                    .get_desktop_entry(&components.borrow().app_db)
                    .expect("Expect to get DesktopEntry from DesktopEntryId")
                    .name()
                    .unwrap_or(&id)
                    .to_string();

                let is_assigned = if let Some(assigned_desktop_entry) = components
                    .borrow()
                    .mime_db
                    .assigned_application_for(mime_type)
                {
                    assigned_desktop_entry == &desktop_entry_id
                } else {
                    false
                };

                applications_combobox.append(Some(&id), &display_name);

                if is_assigned {
                    applications_combobox.set_active_id(Some(&id));
                }
            }
        }

        match applications.n_items() {
            0 => {
                // no applications support this mime type. Note, we should not be showing
                // any mimetypes in this state, but it's worth handling in case things change.
                applications_combobox.set_visible(false);
                applications_combobox.set_sensitive(false);
            }
            1 => {
                // only one application supports this mime type, disable the control
                applications_combobox.set_visible(true);
                applications_combobox.set_sensitive(false);
            }
            _ => {
                // multiple applications support this mime type, show and enable control
                applications_combobox.set_visible(true);
                applications_combobox.set_sensitive(true);
            }
        }
    }

    pub fn unbind(&self) {
        // nothing yet, but would make sense to disconnect callbacks
    }
}
