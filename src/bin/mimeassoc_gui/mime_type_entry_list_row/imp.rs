use std::cell::RefCell;

use glib::Binding;
use gtk::subclass::prelude::*;
use gtk::{glib, CheckButton, CompositeTemplate, Label};

// Object holding the state
#[derive(Default, CompositeTemplate)]
#[template(resource = "/org/zakariya/MimeAssoc/mime_type_entry_list_row.ui")]
pub struct MimeTypeEntryListRow {
    pub bindings: RefCell<Vec<Binding>>,

    // UI Bindings
    #[template_child]
    pub completed_button: TemplateChild<CheckButton>,

    #[template_child]
    pub content_label: TemplateChild<Label>,
}

#[glib::object_subclass]
impl ObjectSubclass for MimeTypeEntryListRow {
    // `NAME` needs to match `class` attribute of template
    const NAME: &'static str = "MimeTypeEntryListRow";
    type Type = super::MimeTypeEntryListRow;
    type ParentType = gtk::Box;

    fn class_init(klass: &mut Self::Class) {
        klass.bind_template();
    }

    fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
        obj.init_template();
    }
}

// Trait shared by all GObjects
impl ObjectImpl for MimeTypeEntryListRow {}

// Trait shared by all widgets
impl WidgetImpl for MimeTypeEntryListRow {}

// Trait shared by all boxes
impl BoxImpl for MimeTypeEntryListRow {}
