mod mime_assoc;
use mime_assoc::*;

fn main() {
    let desktop_entry_dirs = desktop_entry_dirs();
    let mimeapps_lists = mimeapps_lists_paths();

    if let Ok(desktop_entry_dirs) = desktop_entry_dirs {
        println!("DESKTOP ENTRY DIRS: ");
        for dir in desktop_entry_dirs.iter() {
            println!("\t{:?}", dir);
        }
        println!("\n\n");
    }

    if let Ok(mimeapps_lists) = mimeapps_lists {
        println!("MIMEAPPS LISTS FILES: ");
        for file in mimeapps_lists.iter() {
            println!("\t{:?}", file);
        }
    }
}
