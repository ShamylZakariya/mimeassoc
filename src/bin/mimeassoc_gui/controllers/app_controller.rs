use std::boxed::Box;
use std::cell::RefCell;
use std::rc::Rc;

use adw::subclass::prelude::*;
use adw::{prelude::*, *};
use glib::subclass::*;
use gtk::{glib::*, *};
use mimeassoc::*;

use crate::model::*;
use crate::ui::MainWindow;

use super::{ApplicationsPaneController, MimeTypesPaneController};

pub struct AppController {
    window: glib::object::WeakRef<MainWindow>,
    mime_types_pane_controller: MimeTypesPaneController,
    applications_pane_controller: ApplicationsPaneController,
}

impl AppController {
    pub fn new(window: glib::object::WeakRef<MainWindow>) -> Self {
        log::debug!("MimeTypesViewModel::new");
        Self {
            window: window.clone(),
            mime_types_pane_controller: MimeTypesPaneController::new(window.clone()),
            applications_pane_controller: ApplicationsPaneController::new(window),
        }
    }

    fn foo(&self) {
        if let Some(window) = self.window.upgrade() {
            // huh, this might work
        }
    }
}
