use clap::{Parser, Subcommand};

mod mime_assoc;
use mime_assoc::*;

use crate::mime_assoc::desktop_entry::*;
use crate::mime_assoc::mime_type::*;

////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////

fn display_mime_types(mime_db: &MimeAssociations) {
    let mut mime_types = mime_db.mime_types();
    mime_types.sort();

    for mime_type in mime_types.iter() {
        println!("{}", mime_type);
    }
}

fn display_mime_type(mime_db: &MimeAssociations, desktop_db: &DesktopEntries, id: &str) {
    if let Ok(mime_type) = MimeType::parse(id) {
        let desktop_entries = desktop_db.get_desktop_entries_for_mimetype(&mime_type);
        let default_handler = mime_db.default_application_for(&mime_type);
        if !desktop_entries.is_empty() {
            for desktop_entry in desktop_entries {
                if Some(desktop_entry.id()) == default_handler {
                    println!("*{}", desktop_entry.id());
                } else {
                    println!(" {}", desktop_entry.id());
                }
            }
        } else {
            println!("No installed applications support \"{}\"", mime_type);
        }
    } else {
        panic!("\"{}\" is not a valid mime type identifier", id);
    }
}

fn display_applications(mime_db: &MimeAssociations, desktop_db: &DesktopEntries) {
    for desktop_entry in desktop_db.desktop_entries() {
        println!("{}:", desktop_entry.id());
        let mut mime_types = desktop_entry.mime_types().clone();
        mime_types.sort();
        for mime_type in mime_types.iter() {
            let is_handler = mime_db.default_application_for(mime_type) == Some(desktop_entry.id());
            if is_handler {
                println!("\t*{}", mime_type);
            } else {
                println!("\t {}", mime_type);
            }
        }
        if !mime_types.is_empty() {
            println!();
        }
    }
}

fn display_application(mime_db: &MimeAssociations, desktop_db: &DesktopEntries, id: &str) {
    if let Ok(desktop_entry_id) = DesktopEntryId::parse(id) {
        if let Some(desktop_entry) = desktop_db.get_desktop_entry(&desktop_entry_id) {
            println!("{}:", desktop_entry.id());
            let mut mime_types = desktop_entry.mime_types().clone();
            mime_types.sort();
            for mime_type in mime_types.iter() {
                let is_handler =
                    mime_db.default_application_for(mime_type) == Some(desktop_entry.id());
                if is_handler {
                    println!("\t*{}", mime_type);
                } else {
                    println!("\t {}", mime_type);
                }
            }
        } else {
            panic!("\"{}\" does not appear to be an installed application", id);
        }
    } else {
        panic!("\"{}\" is not a valid desktop entry id", id);
    }
}

////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////

/// Display system mime type and default applications info
#[derive(Parser)]
#[command(author, version, about, long_about = None, arg_required_else_help = true)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// Print registered mime types
    MimeTypes,
    /// Prints all applications which support the specified mime type, and which is currently assigned as default handler
    MimeType {
        #[arg(short, long)]
        id: String,
    },
    /// Display all applications and their supported mime types, with an asterisk indicating which are registered to that application
    Applications,
    /// Display a specific application and the mimetypes it supports, with an asterisk indicating which are registered to that application
    Application {
        #[arg(short, long)]
        id: String,
    },
}

fn main() {
    let cli = Cli::parse();

    let desktop_entry_dirs = match desktop_entry_dirs() {
        Ok(desktop_entry_dirs) => desktop_entry_dirs,
        Err(e) => panic!("Unable to load desktop_entry_dirs: {:?}", e),
    };

    let mimeapps_lists = match mimeapps_lists_paths() {
        Ok(mimeapps_lists) => mimeapps_lists,
        Err(e) => panic!("Unable to load mimeapps_lists_paths: {:?}", e),
    };

    let mime_db = match MimeAssociations::load(&mimeapps_lists) {
        Ok(mime_assoc) => mime_assoc,
        Err(e) => panic!("Unable to load MimeAssociations: {:?}", e),
    };

    let desktop_db = match DesktopEntries::load(&desktop_entry_dirs) {
        Ok(desktop_entries) => desktop_entries,
        Err(e) => panic!("Unable to load DesktopEntries: {:?}", e),
    };

    match &cli.command {
        Some(Commands::MimeTypes) => display_mime_types(&mime_db),
        Some(Commands::MimeType { id }) => display_mime_type(&mime_db, &desktop_db, id),
        Some(Commands::Applications) => display_applications(&mime_db, &desktop_db),
        Some(Commands::Application { id }) => display_application(&mime_db, &desktop_db, id),
        None => {}
    }
}
