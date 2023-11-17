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

pub struct MimeTypesPaneController {
    window: glib::object::WeakRef<MainWindow>,
    on_commit: Box<dyn Fn() -> ()>,
}

impl MimeTypesPaneController {
    pub fn new(
        window: glib::object::WeakRef<MainWindow>,
        on_commit: impl Fn() -> () + 'static,
    ) -> Self {
        log::debug!("MimeTypesViewModel::new");
        Self {
            window,
            on_commit: Box::new(on_commit),
        }
    }

    fn foo(&self) {
        if let Some(window) = self.window.upgrade() {
            // huh, this might work
        }
    }
}
