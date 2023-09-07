use adw::subclass::prelude::*;
use adw::*;
use glib::Object;
use gtk::{glib, *};

use crate::model::*;

mod imp {
    use super::*;
    use gtk::traits::*;

    // Object holding the state
    #[derive(Default, CompositeTemplate)]
    #[template(resource = "/org/zakariya/MimeAssoc/application_entry_list_row.ui")]
    pub struct ApplicationEntryListRow {
        // UI Bindings
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

            self.name_label.set_halign(Align::Start);
            self.info_label.set_halign(Align::Start);
            self.info_label.set_opacity(0.7);
        }
    }

    // Trait shared by all widgets
    impl WidgetImpl for ApplicationEntryListRow {}

    // Trait shared by all boxes
    impl BoxImpl for ApplicationEntryListRow {}
}

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
        let desktop_entry = &application_entry
            .desktop_entry()
            .expect("Expect to get desktop entry id from ApplicationEntry");
        let content_label = self.imp().name_label.get();
        content_label.set_text(desktop_entry.name().unwrap_or("<Unnamed Application>"));

        let info_label = self.imp().info_label.get();
        info_label.set_text(&desktop_entry.id().to_string());
    }

    pub fn unbind(&self) {}
}
