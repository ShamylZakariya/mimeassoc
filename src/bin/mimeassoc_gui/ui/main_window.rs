use std::cell::OnceCell;

use adw::subclass::prelude::*;
use adw::{prelude::*, *};
use glib::subclass::*;
use gtk::{glib::*, *};
use mimeassoc::*;

///////////////////////////////////////////////////////////////////////

/// Represents a command which can be sent to the main window. This is primarily
/// meant for easing manual testing, but could be used to handle gui cmdline arguments,
/// for example taking the app directly to a specified mime type.
#[derive(Debug)]
pub enum MainWindowCommand {
    ShowMimeType(MimeType),
    ShowApplication(DesktopEntryId),
}

/// Represents the top-level "page" the app is displaying, Applications or Mime Types.
#[derive(Debug)]
pub enum MainWindowPage {
    Applications,
    MimeTypes,
}

impl MainWindowPage {
    /// Creates a MainWindowPage from the page name as set in the .ui XML file.
    fn from_ui_name(name: &str) -> Option<Self> {
        match name {
            "mime_types_page" => Some(MainWindowPage::MimeTypes),
            "applications_page" => Some(MainWindowPage::Applications),
            _ => None,
        }
    }
}

///////////////////////////////////////////////////////////////////////

mod imp {
    use super::*;

    #[derive(CompositeTemplate, Default)]
    #[template(resource = "/org/zakariya/MimeAssoc/main_window.ui")]
    pub struct MainWindow {
        pub undo_action: OnceCell<gtk::gio::SimpleAction>,
        // Controllers
        // pub app_controller: OnceCell<crate::controllers::AppController>,
        // // UI bindings
        // #[template_child]
        // pub commit_button: TemplateChild<Button>,

        // #[template_child]
        // pub stack: TemplateChild<ViewStack>,

        // #[template_child]
        // pub mime_types_page: TemplateChild<ViewStackPage>,

        // // mime types page UI bindings
        // #[template_child]
        // pub mime_types_scrolled_window: TemplateChild<ScrolledWindow>,

        // #[template_child]
        // pub mime_types_list_box: TemplateChild<ListBox>,

        // #[template_child]
        // pub mime_type_pane_detail_scrolled_window: TemplateChild<ScrolledWindow>,

        // #[template_child]
        // pub mime_type_pane_detail_applications_list_box: TemplateChild<ListBox>,

        // #[template_child]
        // pub mime_type_pane_detail_info_label: TemplateChild<Label>,

        // // applications page UI bindings
        // #[template_child]
        // pub applications_page: TemplateChild<ViewStackPage>,

        // #[template_child]
        // pub applications_scrolled_window: TemplateChild<ScrolledWindow>,

        // #[template_child]
        // pub applications_list_box: TemplateChild<ListBox>,

        // #[template_child]
        // pub application_to_mime_type_assignment_scrolled_window: TemplateChild<ScrolledWindow>,

        // #[template_child]
        // pub application_to_mime_type_assignment_list_box: TemplateChild<ListBox>,

        // #[template_child]
        // pub application_detail_select_all: TemplateChild<Button>,

        // #[template_child]
        // pub application_detail_select_none: TemplateChild<Button>,
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

            let obj = self.obj();

            // Setup
            obj.setup_actions();
            obj.setup_ui();
            obj.setup_view_controllers();
        }
    }

    // Trait shared by all widgets
    impl WidgetImpl for MainWindow {}

    // Trait shared by all windows
    impl WindowImpl for MainWindow {}

    // Trait shared by all application windows
    impl ApplicationWindowImpl for MainWindow {}
}

glib::wrapper! {
    pub struct MainWindow(ObjectSubclass<imp::MainWindow>)
        @extends gtk::ApplicationWindow, gtk::Window, gtk::Widget,
        @implements gio::ActionGroup, gio::ActionMap, gtk::Accessible, gtk::Buildable,
                    gtk::ConstraintTarget, gtk::Native, gtk::Root, gtk::ShortcutManager;
}

impl MainWindow {
    pub fn new(app: &adw::Application) -> Self {
        Object::builder().property("application", app).build()
    }

    // pub fn app_controller(&self) -> &crate::controllers::AppController {
    //     self.imp().app_controller.get().unwrap()
    // }

    fn setup_view_controllers(&self) {
        // let weak_self = glib::object::WeakRef::new();
        // weak_self.set(Some(self));

        // let _ = self.imp().app_controller.set(crate::controllers::AppController::new(weak_self));
    }

    fn setup_ui(&self) {
        // wire up buttons - note, AppController instance isn't created yet when setup_ui
        // is run, so we defer access to the time of invocation.

        // self.imp()
        //     .commit_button
        //     .connect_clicked(clone!(@weak self as window => move |_|{
        //         window.app_controller().commit_changes();
        //     }));

        // self.imp().application_detail_select_all.connect_clicked(
        //     clone!(@weak self as window => move |_|{
        //         window.app_controller().applications_pane_controller().on_select_all();
        //     }),
        // );

        // self.imp().application_detail_select_none.connect_clicked(
        //     clone!(@weak self as window => move |_|{
        //         window.app_controller().applications_pane_controller().on_select_none();
        //     }),
        // );

        // // listen for when user switches between MimeTypes and Applications panes
        // self.imp().stack.connect_visible_child_notify(
        //     clone!(@weak self as window => move |stack| {
        //         if let Some(name) = stack.visible_child_name() {
        //             if let Some(page) = MainWindowPage::from_ui_name(&name) {
        //                 window.app_controller().on_current_page_changed(page);
        //             }
        //         }
        //     }),
        // );
    }

    fn setup_actions(&self) {
        // wire up actions - note, AppController instance isn't created yet when setup_ui
        // is run, so we defer access to the time of invocation.

        // let action_show_mime_types = gtk::gio::SimpleAction::new("show-mime-types", None);
        // action_show_mime_types.connect_activate(clone!(@weak self as window => move |_, _|{
        //     window.app_controller().show_page(MainWindowPage::MimeTypes);
        // }));
        // self.add_action(&action_show_mime_types);

        // let action_show_applications = gtk::gio::SimpleAction::new("show-applications", None);
        // action_show_applications.connect_activate(clone!(@weak self as window => move |_, _|{
        //     window.app_controller().show_page(MainWindowPage::Applications);
        // }));
        // self.add_action(&action_show_applications);

        // let action_reset_user_default_application_assignments =
        //     gtk::gio::SimpleAction::new("reset-user-default-applications", None);
        // action_reset_user_default_application_assignments.connect_activate(
        //     clone!(@weak self as window => move |_, _| {
        //         window.app_controller().query_reset_user_default_application_assignments();
        //     }),
        // );
        // self.add_action(&action_reset_user_default_application_assignments);

        // let action_clear_orphaned_application_assignments =
        //     gtk::gio::SimpleAction::new("prune-orphaned-application-assignments", None);
        // action_clear_orphaned_application_assignments.connect_activate(
        //     clone!(@weak self as window => move |_, _| {
        //         window.app_controller().query_prune_orphaned_application_assignments();
        //     }),
        // );
        // self.add_action(&action_clear_orphaned_application_assignments);

        // let about_action = gtk::gio::SimpleAction::new("show-about", None);
        // about_action.connect_activate(
        //     clone!(@weak self as window => move |_, _| { window.app_controller().show_about(); }),
        // );
        // self.add_action(&about_action);

        // let discard_uncommited_changes_action =
        //     gtk::gio::SimpleAction::new("discard-uncommitted-changes", None);
        // discard_uncommited_changes_action.connect_activate(
        //     clone!(@weak self as window => move |_, _| {
        //         window.app_controller().discard_uncommitted_changes();
        //     }),
        // );
        // self.add_action(&discard_uncommited_changes_action);

        // let undo_action = gtk::gio::SimpleAction::new("undo", None);
        // undo_action.connect_activate(clone!(@weak self as window => move |_, _| {
        //     window.app_controller().undo();
        // }));
        // self.add_action(&undo_action);
        // self.imp()
        //     .undo_action
        //     .set(undo_action)
        //     .expect("MainWindow::setup_actions must only be executed once");

        // let log_history_action = gtk::gio::SimpleAction::new("log-history-stack", None);
        // log_history_action.connect_activate(clone!(@weak self as window => move |_, _| {
        //     let stores = window.app_controller().stores();
        //     let stores = stores.borrow();
        //     stores.debug_log_history_stack();
        // }));
        // self.add_action(&log_history_action);
    }
}
