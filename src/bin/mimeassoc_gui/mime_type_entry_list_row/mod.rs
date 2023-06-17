mod imp;

use adw::prelude::*;
use adw::subclass::prelude::*;
use glib::Object;
use gtk::glib;

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
        // Get state
        // let completed_button = self.imp().completed_button.get();
        let content_label = self.imp().content_label.get();
        // let mut bindings = self.imp().bindings.borrow_mut();

        content_label.set_text(&mime_type_entry.get_mime_type().to_string());

        // // Bind `task_object.completed` to `task_row.completed_button.active`
        // let completed_button_binding = task_object
        //     .bind_property("completed", &completed_button, "active")
        //     .bidirectional()
        //     .sync_create()
        //     .build();
        // // Save binding
        // bindings.push(completed_button_binding);

        // Bind `task_object.content` to `task_row.content_label.label`
        // let content_label_binding = mime_type_entry
        //     .bind_property("content", &content_label, "label")
        //     .sync_create()
        //     .build();
        // // Save binding
        // bindings.push(content_label_binding);

        // // Bind `task_object.completed` to `task_row.content_label.attributes`
        // let content_label_binding = task_object
        //     .bind_property("completed", &content_label, "attributes")
        //     .sync_create()
        //     .transform_to(|_, active| {
        //         let attribute_list = AttrList::new();
        //         if active {
        //             // If "active" is true, content of the label will be strikethrough
        //             let attribute = AttrInt::new_strikethrough(true);
        //             attribute_list.insert(attribute);
        //         }
        //         Some(attribute_list.to_value())
        //     })
        //     .build();
        // // Save binding
        // bindings.push(content_label_binding);
    }

    pub fn unbind(&self) {
        // Unbind all stored bindings
        for binding in self.imp().bindings.borrow_mut().drain(..) {
            binding.unbind();
        }
    }
}
