mod imp;

use adw::prelude::*;
use adw::subclass::prelude::*;
use glib::Object;
use gtk::glib;

use crate::model::*;

glib::wrapper! {
    pub struct ApplicationEntryMimetypeAssignmentRow(ObjectSubclass<imp::ApplicationEntryMimetypeAssignmentRow>)
    @extends gtk::Box, gtk::Widget,
    @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget, gtk::Orientable;
}

impl Default for ApplicationEntryMimetypeAssignmentRow {
    fn default() -> Self {
        Self::new()
    }
}

impl ApplicationEntryMimetypeAssignmentRow {
    pub fn new() -> Self {
        Object::builder().build()
    }

    pub fn bind(&self, handled_mime_type_entry: &MimeTypeAssignmentEntry) {
        let mime_type_name_label = self.imp().mime_type_name_label.get();
        let checkbox = self.imp().assigned_checkbox.get();
        let mut bindings = self.imp().bindings.borrow_mut();

        let name_label_binding = handled_mime_type_entry
            .bind_property("id", &mime_type_name_label, "label")
            .sync_create()
            .build();
        bindings.push(name_label_binding);

        let is_assigned_binding = handled_mime_type_entry
            .bind_property("assigned", &checkbox, "active")
            .bidirectional()
            .sync_create()
            .build();
        bindings.push(is_assigned_binding);
    }

    pub fn unbind(&self) {
        for binding in self.imp().bindings.borrow_mut().drain(..) {
            binding.unbind();
        }
    }
}
