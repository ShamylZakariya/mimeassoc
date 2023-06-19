mod imp;

use std::cell::RefCell;
use std::rc::Rc;

use adw::prelude::*;
use adw::subclass::prelude::*;
use glib::Object;
use gtk::glib::{self, clone};
use mimeassoc::desktop_entry::DesktopEntryId;
use mimeassoc::mime_type::MimeType;

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
                    .get_desktop_entry(&components.borrow().desktop_entry_store)
                    .expect("Expect to get DesktopEntry from DesktopEntryId")
                    .name()
                    .unwrap_or(&id)
                    .to_string();

                let is_assigned = if let Some(assigned_desktop_entry) = components
                    .borrow()
                    .mime_associations_store
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

        let callback_id = applications_combobox.connect_changed(clone!(@weak mime_type_entry, @weak components, @weak self as window => move |a|{
            if let Some(active_id) = a.active_id() {
                let active_id = active_id.as_str();
                let mime_type = mime_type_entry.get_mime_type();
                let desktop_entry_id = DesktopEntryId::parse(active_id).expect("Expected valid desktop entry id");
                window.assign_handler_for_mimetype(&desktop_entry_id, &mime_type, components);
            }
        }));

        self.imp()
            .combobox_changed_handler_id
            .replace(Some(callback_id));
    }

    pub fn unbind(&self) {
        let applications_combobox = self.imp().applications_combo_box.get();
        applications_combobox.remove_all();

        if let Some(bar) = self.imp().combobox_changed_handler_id.take() {
            applications_combobox.disconnect(bar);
        }
    }

    fn assign_handler_for_mimetype(
        &self,
        desktop_entry_id: &DesktopEntryId,
        mime_type: &MimeType,
        components: Rc<RefCell<Components>>,
    ) {
        let desktop_entry = components
            .borrow()
            .desktop_entry_store
            .get_desktop_entry(desktop_entry_id)
            .expect("Expect to find a desktop entry for the id")
            .clone();

        let mut components = components.borrow_mut();

        if let Err(e) = components
            .mime_associations_store
            .set_default_handler_for_mime_type(mime_type, &desktop_entry)
        {
            // TODO: Error dialog?
            println!(
                "Unable to assign {} to open {}; error: {:?}",
                desktop_entry_id, mime_type, e
            );
        } else {
            println!(
                "Assigned {} to be default handler for {}",
                desktop_entry_id, mime_type
            );
            if let Err(e) = components.mime_associations_store.save() {
                // TODO: Error dialog?
                println!("Unable to save mime associations database, error: {:?}", e);
            }
        }
    }
}
