mod imp;

use adw::prelude::*;
use adw::subclass::prelude::*;
use adw::*;
use glib::Object;
use gtk::{
    glib::{self, clone},
    *,
};

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
        self.bind_labels(application_entry);
        self.bind_mime_type_assignment_list(application_entry);
    }

    pub fn unbind(&self) {}

    fn bind_labels(&self, application_entry: &ApplicationEntry) {
        let desktop_entry = &application_entry.desktop_entry();
        let content_label = self.imp().name_label.get();
        content_label.set_text(desktop_entry.name().unwrap_or("<Unnamed Application>"));

        let info_label = self.imp().info_label.get();
        info_label.set_text(&desktop_entry.id().to_string());
    }

    fn bind_mime_type_assignment_list(&self, application_entry: &ApplicationEntry) {
        let model = NoSelection::new(Some(application_entry.mime_type_assignments()));
        self.imp().mime_type_assignment_list.bind_model(Some(&model),
            clone!(@weak self as window => @default-panic, move |obj| {
                let model = obj.downcast_ref().expect("The object should be of type `MimeTypeAssignmentEntry`.");
                let row = window.create_mime_type_assignment_row(model);
                row.upcast()
            }));
    }

    fn create_mime_type_assignment_row(
        &self,
        mime_type_assignment_entry: &MimeTypeAssignmentEntry,
    ) -> ActionRow {
        let check_button = CheckButton::builder()
            .valign(Align::Center)
            .can_focus(false)
            .build();

        // Create row
        let row = ActionRow::builder()
            .activatable_widget(&check_button)
            .build();
        row.add_prefix(&check_button);

        // Bind properties
        mime_type_assignment_entry
            .bind_property("assigned", &check_button, "active")
            .bidirectional()
            .sync_create()
            .build();
        mime_type_assignment_entry
            .bind_property("id", &row, "title")
            .sync_create()
            .build();

        row
    }
}
