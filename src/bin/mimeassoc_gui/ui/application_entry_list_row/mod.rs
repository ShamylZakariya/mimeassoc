mod imp;

use adw::prelude::*;
use adw::subclass::prelude::*;
use glib::Object;
use gtk::{glib, *};

use crate::model::*;

use super::ApplicationEntryMimetypeAssignmentRow;

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
        self.imp().mime_type_assignment_list.set_model(Some(&model));
    }

    // configures the mimetype assignment list view's factory
    fn setup_mimetype_assignment_list_view(&self) {
        let factory = SignalListItemFactory::new();
        factory.connect_setup(move |_, list_item| {
            let row = ApplicationEntryMimetypeAssignmentRow::new();
            let list_item = list_item
                .downcast_ref::<ListItem>()
                .expect("Needs to be ListItem");
            list_item.set_child(Some(&row));
        });

        factory.connect_bind(move |_, list_item| {
            let model = list_item
                .downcast_ref::<ListItem>()
                .expect("Needs to be ListItem")
                .item()
                .and_downcast::<MimeTypeAssignmentEntry>()
                .expect("The item has to be an `MimeTypeAssignmentEntry`.");

            let row = list_item
                .downcast_ref::<ListItem>()
                .expect("Needs to be ListItem")
                .child()
                .and_downcast::<ApplicationEntryMimetypeAssignmentRow>()
                .expect("The child has to be a `ApplicationEntryMimetypeAssignmentRow`.");

            row.bind(&model);
        });

        factory.connect_unbind(move |_, list_item| {
            let row = list_item
                .downcast_ref::<ListItem>()
                .expect("Needs to be ListItem")
                .child()
                .and_downcast::<ApplicationEntryMimetypeAssignmentRow>()
                .expect("The child has to be a `ApplicationEntryMimetypeAssignmentRow`.");
            row.unbind();
        });

        self.imp().mime_type_assignment_list.set_factory(Some(&factory));
    }
}
