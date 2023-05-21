use clap::{Args, Parser, Subcommand};

use mimeassoc::desktop_entry::*;
use mimeassoc::mime_type::*;
use mimeassoc::*;

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
    /// Display all registered mime types
    MimeTypes,
    /// Display all applications which support the specified mime type, and which is currently assigned as default handler
    MimeType(MimeTypeCommandArgs),
    /// Display all applications and their supported mime types, with an asterisk indicating which are registered to that application
    Applications,
    /// Display a specific application and the mimetypes it supports, with an asterisk indicating which are registered to that application
    Application(ApplicationCommandArgs),
    /// Assign an application as default handler for one or more mime types. If no mime types are specified, makes the specified application default handler for ALL it's supported mime types
    Set(SetCommandArgs),
    /// Display the configuration state of `mimeassoc`. Shows the mime association sources, and where desktop entry files were loaded from, in chain order.
    Configuration,
}

impl Commands {
    fn process(&self, mime_db: &mut MimeAssociations, desktop_entry_db: &DesktopEntries) {
        match self {
            Commands::MimeTypes => Self::display_mime_types(mime_db),
            Commands::MimeType(args) => {
                Self::display_mime_type(mime_db, desktop_entry_db, args.id.as_deref())
            }
            Commands::Applications => Self::display_applications(mime_db, desktop_entry_db),
            Commands::Application(args) => {
                Self::display_application(mime_db, desktop_entry_db, args.id.as_deref())
            }
            Commands::Configuration => Self::display_configuration(mime_db, desktop_entry_db),
            Commands::Set(args) => Self::set_default_handler(
                mime_db,
                desktop_entry_db,
                &args.desktop_entry.as_str(),
                &args
                    .mime_types
                    .iter()
                    .map(AsRef::as_ref)
                    .collect::<Vec<_>>(),
            ),
        }
    }

    fn display_mime_types(mime_db: &MimeAssociations) {
        let mut mime_types = mime_db.mime_types();
        mime_types.sort();

        for mime_type in mime_types.iter() {
            println!("{}", mime_type);
        }
    }

    fn display_mime_type(
        mime_db: &MimeAssociations,
        desktop_entry_db: &DesktopEntries,
        id: Option<&str>,
    ) {
        let Some(id) = id else {
            panic!("No mime type provded.");
        };

        let Ok(mime_type) = MimeType::parse(id) else {
            panic!("\"{}\" is not a valid mime type identifier", id);
        };

        let desktop_entries = desktop_entry_db.get_desktop_entries_for_mimetype(&mime_type);
        let default_handler = mime_db.assigned_application_for(&mime_type);
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
    }

    fn display_applications(mime_db: &MimeAssociations, desktop_entry_db: &DesktopEntries) {
        for desktop_entry in desktop_entry_db.desktop_entries() {
            println!(
                "{} ({}):",
                desktop_entry.id(),
                desktop_entry.name().unwrap_or("<Unnamed>")
            );

            let mut mime_types = desktop_entry.mime_types().clone();
            mime_types.sort();

            for mime_type in mime_types.iter() {
                let is_handler =
                    mime_db.assigned_application_for(mime_type) == Some(desktop_entry.id());
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

    fn display_application(
        mime_db: &MimeAssociations,
        desktop_entry_db: &DesktopEntries,
        id: Option<&str>,
    ) {
        let Some(id) = id else {
            panic!("No desktop entry id provded.");
        };

        let Some(desktop_entry) = lookup_desktop_entry(desktop_entry_db, id) else {
            panic!("\"{}\" does not appear to be an installed application", id);
        };

        println!(
            "{} ({}):",
            desktop_entry.id(),
            desktop_entry.name().unwrap_or("<Unnamed>")
        );

        let mut mime_types = desktop_entry.mime_types().clone();
        mime_types.sort();

        for mime_type in mime_types.iter() {
            let is_handler =
                mime_db.assigned_application_for(mime_type) == Some(desktop_entry.id());
            if is_handler {
                println!("\t*{}", mime_type);
            } else {
                println!("\t {}", mime_type);
            }
        }
    }

    fn display_configuration(mime_db: &MimeAssociations, desktop_entry_db: &DesktopEntries) {
        println!("Mime association sources:");
        for path in mime_db.sources() {
            println!("\t{:?}", path);
        }
        println!("\nDesktop Entry directories:");
        for directory in desktop_entry_db.sources() {
            println!("\t{:?}", directory);
        }
    }

    fn set_default_handler(
        mime_db: &mut MimeAssociations,
        desktop_entry_db: &DesktopEntries,
        desktop_entry_id: &str,
        mime_types: &[&str],
    ) {
        // map input mime type strings to mime types
        let mut resolved_mime_types: Vec<MimeType> = vec![];
        for mime_type in mime_types {
            if let Ok(mime_type) = MimeType::parse(mime_type) {
                resolved_mime_types.push(mime_type);
            } else {
                panic!("\"{}\" is not a valid mime type identifier", mime_type);
            }
        }

        // find the desktop entry
        let Some(desktop_entry) = lookup_desktop_entry(desktop_entry_db, desktop_entry_id) else {
            panic!("\"{}\" does not appear to be an installed application", desktop_entry_id);
        };

        // if no mime types are supported, we will use all the mime types this app claims to handle
        if resolved_mime_types.is_empty() {
            resolved_mime_types = desktop_entry.mime_types().clone();
        }

        // verify the desktop entry can handle the provided mime types
        for mime_type in resolved_mime_types.iter() {
            if !desktop_entry.can_open_mime_type(mime_type) {
                panic!(
                    "\"{}\" is not able to open mime type \"{}\"",
                    desktop_entry.id(),
                    mime_type
                );
            }
        }

        println!(
            "Assigning {} as default handler for [{}]",
            desktop_entry.id(),
            resolved_mime_types
                .iter()
                .map(|m| m.to_string())
                .collect::<Vec<_>>()
                .join(", ")
        );

        for mime_type in resolved_mime_types.iter() {
            match mime_db.set_default_handler_for_mime_type(mime_type, desktop_entry) {
                Ok(_) => println!("\tassigned {}", mime_type),
                Err(e) => panic!(
                    "Failed to assign {} to {}, error: {:?}",
                    mime_type,
                    desktop_entry.id(),
                    e
                ),
            }
        }

        // persist the changes...
        if let Err(e) = mime_db.save() {
            panic!("Unable to save changes: {:?}", e);
        }
    }
}

#[derive(Args)]
struct MimeTypeCommandArgs {
    id: Option<String>,
}

#[derive(Args)]
struct ApplicationCommandArgs {
    id: Option<String>,
}

#[derive(Args)]
struct SetCommandArgs {
    desktop_entry: String,
    mime_types: Vec<String>,
}

fn main() {
    let desktop_entry_dirs = match desktop_entry_dirs() {
        Ok(desktop_entry_dirs) => desktop_entry_dirs,
        Err(e) => panic!("Unable to load desktop_entry_dirs: {:?}", e),
    };

    let mimeapps_lists = match mimeapps_lists_paths() {
        Ok(mimeapps_lists) => mimeapps_lists,
        Err(e) => panic!("Unable to load mimeapps_lists_paths: {:?}", e),
    };

    let mut mime_db = match MimeAssociations::load(&mimeapps_lists) {
        Ok(mimeassoc) => mimeassoc,
        Err(e) => panic!("Unable to load MimeAssociations: {:?}", e),
    };

    let desktop_entry_db = match DesktopEntries::load(&desktop_entry_dirs) {
        Ok(desktop_entries) => desktop_entries,
        Err(e) => panic!("Unable to load DesktopEntries: {:?}", e),
    };

    let cli = Cli::parse();
    if let Some(command) = &cli.command {
        command.process(&mut mime_db, &desktop_entry_db);
    }
}
