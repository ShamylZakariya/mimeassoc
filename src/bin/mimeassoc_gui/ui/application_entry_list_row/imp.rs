use gtk::subclass::prelude::*;
use gtk::traits::{BoxExt, WidgetExt};
use gtk::{glib, *};

// Object holding the state
#[derive(Default, CompositeTemplate)]
#[template(resource = "/org/zakariya/MimeAssoc/application_entry_list_row.ui")]
pub struct ApplicationEntryListRow {
    // UI Bindings
    #[template_child]
    pub label_box: TemplateChild<Box>,

    #[template_child]
    pub name_label: TemplateChild<Label>,

    #[template_child]
    pub info_label: TemplateChild<Label>,
}

#[glib::object_subclass]
impl ObjectSubclass for ApplicationEntryListRow {
    // `NAME` needs to match `class` attribute of template
    const NAME: &'static str = "ApplicationEntryListRow";
    type Type = super::ApplicationEntryListRow;
    type ParentType = gtk::Box;

    fn class_init(klass: &mut Self::Class) {
        klass.bind_template();
    }

    fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
        obj.init_template();
    }
}

// Trait shared by all GObjects
impl ObjectImpl for ApplicationEntryListRow {
    fn constructed(&self) {
        self.parent_constructed();

        self.obj().set_spacing(12);
        self.label_box.set_hexpand(true);
        self.name_label.set_halign(Align::Start);
        self.info_label.set_halign(Align::Start);
        self.info_label.set_opacity(0.7);
    }
}

// Trait shared by all widgets
impl WidgetImpl for ApplicationEntryListRow {}

// Trait shared by all boxes
impl BoxImpl for ApplicationEntryListRow {}
