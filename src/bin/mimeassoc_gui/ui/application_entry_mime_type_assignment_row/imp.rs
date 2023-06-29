use std::cell::RefCell;

use gtk::subclass::prelude::*;
use gtk::traits::{BoxExt, WidgetExt};
use gtk::{glib, *};

// Object holding the state
#[derive(Default, CompositeTemplate)]
#[template(resource = "/org/zakariya/MimeAssoc/application_entry_mimetype_assignment_row.ui")]
pub struct ApplicationEntryMimetypeAssignmentRow {
    // UI Bindings
    #[template_child]
    pub assigned_checkbox: TemplateChild<CheckButton>,

    #[template_child]
    pub mime_type_name_label: TemplateChild<Label>,

    pub bindings: RefCell<Vec<glib::Binding>>,
}

#[glib::object_subclass]
impl ObjectSubclass for ApplicationEntryMimetypeAssignmentRow {
    // `NAME` needs to match `class` attribute of template
    const NAME: &'static str = "ApplicationEntryMimeTypeAssignmentRow";
    type Type = super::ApplicationEntryMimetypeAssignmentRow;
    type ParentType = gtk::Box;

    fn class_init(klass: &mut Self::Class) {
        klass.bind_template();
    }

    fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
        obj.init_template();
    }
}

// Trait shared by all GObjects
impl ObjectImpl for ApplicationEntryMimetypeAssignmentRow {
    fn constructed(&self) {
        self.parent_constructed();
        self.obj().set_spacing(12);
    }
}

// Trait shared by all widgets
impl WidgetImpl for ApplicationEntryMimetypeAssignmentRow {}

// Trait shared by all boxes
impl BoxImpl for ApplicationEntryMimetypeAssignmentRow {}
