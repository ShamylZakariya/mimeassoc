use std::cell::{OnceCell, RefCell};
use std::rc::Rc;

use adw::prelude::*;
use adw::{subclass::prelude::*, *};
use glib::subclass::*;
use gtk::*;

use crate::components::Components;

#[derive(CompositeTemplate, Default)]
#[template(resource = "/org/zakariya/MimeAssoc/main_window.ui")]
pub struct MainWindow {
    pub components: OnceCell<Rc<RefCell<Components>>>,

    #[template_child]
    pub stack: TemplateChild<ViewStack>,

    #[template_child]
    pub page_mime_types: TemplateChild<ViewStackPage>,

    #[template_child]
    pub mime_type_entries_list: TemplateChild<ListBox>,

    #[template_child]
    pub page_applications: TemplateChild<ViewStackPage>,

    pub mime_type_entries: RefCell<Option<gio::ListStore>>,
}

// The central trait for subclassing a GObject
#[glib::object_subclass]
impl ObjectSubclass for MainWindow {
    // `NAME` needs to match `class` attribute of template
    const NAME: &'static str = "MainWindow";
    type Type = super::MainWindow;
    type ParentType = gtk::ApplicationWindow;

    fn class_init(klass: &mut Self::Class) {
        klass.bind_template();
    }

    fn instance_init(obj: &InitializingObject<Self>) {
        obj.init_template();
    }
}

// Trait shared by all GObjects
impl ObjectImpl for MainWindow {
    fn constructed(&self) {
        // Call "constructed" on parent
        self.parent_constructed();

        // Setup
        let obj = self.obj();
        obj.setup_components();
        obj.setup_models();
        obj.ready();

        // finally
        obj.setup_actions();
    }
}

// Trait shared by all widgets
impl WidgetImpl for MainWindow {}

// Trait shared by all windows
impl WindowImpl for MainWindow {}

// Trait shared by all application windows
impl ApplicationWindowImpl for MainWindow {}
