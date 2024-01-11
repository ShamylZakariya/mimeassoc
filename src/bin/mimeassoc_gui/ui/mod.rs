mod main_window;

use gtk::prelude::ListModelExt;
// re-export for clarity
pub use main_window::MainWindow;

use gtk::{glib::*, *};
use traits::AdjustmentExt;
use traits::WidgetExt;

/// Select the listbox row with an object from the provided model which matches the
/// test, and scroll to make that row visible.
pub fn select_listbox_row_and_scroll_to_visible<F, T, M>(list_box: ListBox, model: &M, test: F)
where
    F: Fn(T) -> bool,
    T: IsA<glib::Object>,
    M: ListModelExt,
{
    let count = model.n_items();
    for i in 0..count {
        let item = model.item(i)
                        .and_downcast::<T>()
                        .expect("ApplicationsModeController::collections_list_model() model should contain instances of ApplicationEntry only");
        if test(item) {
            list_box.select_row(list_box.row_at_index(i as i32).as_ref());

            if i > 0 {
                scroll_listbox_to_selected_row(list_box);
            }
            break;
        }
    }
}

/// Utility to scroll a ListBox to whichever row is currently selected.
pub fn scroll_listbox_to_selected_row(list_box: ListBox) {
    // Note: glib lays out lazily; if a list box has just been created or populated,
    // we won't get correct layout metrics for scrolling until a layout pass has completed.
    // So, we schedule the layout for next idle tick, and keep trying until we succeed.
    // That being said, try once immediately just in case everything's ready.

    if try_scroll_listbox_to_selected_row(list_box.clone()) {
        return;
    }

    idle_add_local(
        clone!(@weak list_box => @default-return glib::ControlFlow::Break, move || {
            if try_scroll_listbox_to_selected_row(list_box) {
                ControlFlow::Break
            } else {
                ControlFlow::Continue
            }
        }),
    );
}

fn try_scroll_listbox_to_selected_row(list_box: ListBox) -> bool {
    if let Some(row) = list_box.selected_row() {
        if let Some(coordinates) = row.translate_coordinates(&list_box, 0.0, 0.0) {
            // If the window hasn't laid out, we'll have (0,0) coordinates
            if coordinates == (0.0, 0.0) {
                return false;
            }

            if let Some(adjustment) = list_box.adjustment() {
                let (_, natural_size) = row.preferred_size();

                adjustment.set_value(
                    coordinates.1 - (adjustment.page_size() - natural_size.height() as f64) / 2.0,
                );

                return true;
            }
        }
    }
    false
}
