use std::path::PathBuf;

use clap::{Args, Parser, Subcommand};

use super::command_output::*;
use mimeassoc::desktop_entry::*;
use mimeassoc::mime_type::*;

/// Display system mime type and default applications info
#[derive(Parser)]
#[command(author, version, about, long_about = None, arg_required_else_help = true)]
pub struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

impl Cli {
    pub fn process(&self, mime_db: &mut MimeAssociations, desktop_entry_db: &DesktopEntries) {
        if let Some(command) = &self.command {
            let command_output = command.process(mime_db, desktop_entry_db);
            let output_consumer = Box::new(DefaultCommandOutputConsumer {});
            if let Err(e) = output_consumer.process(&command_output) {
                panic!("Error processing command output: {}", e);
            }
        }
    }
}

#[derive(Subcommand)]
enum Commands {
    /// Display all registered mime types
    MimeTypes,
    /// Display all applications which support the specified mime type, and which is currently assigned as default handler
    /// Passing a wildcard mimetype such as "image/*", is equivalent to passing "image/bmp image/png image/tiff ... image/N"
    MimeType(MimeTypeCommandArgs),
    /// Display all applications and their supported mime types, with an asterisk indicating which are registered to that application.
    Applications,
    /// Display a specific application and the mimetypes it supports, with an asterisk indicating which are registered to that application
    Application(ApplicationCommandArgs),
    /// Assign an application as default handler for one or more mime types. If no mime types are specified, makes the specified application default handler for ALL it's supported mime types
    Set(SetCommandArgs),
    /// Reset assignments for specified mime types to system default
    Reset(ResetCommandArgs),
    /// Display the configuration state of `mimeassoc`. Shows the mime association sources, and where desktop entry files were loaded from, in chain order.
    Configuration,
}

impl Commands {
    pub fn process(
        &self,
        mime_db: &mut MimeAssociations,
        desktop_entry_db: &DesktopEntries,
    ) -> CommandOutput {
        match self {
            Commands::MimeTypes => Self::get_mime_types(mime_db),
            Commands::MimeType(args) => {
                Self::get_mime_type(mime_db, desktop_entry_db, args.id.as_deref())
            }
            Commands::Applications => Self::get_applications(mime_db, desktop_entry_db),
            Commands::Application(args) => {
                Self::get_application(mime_db, desktop_entry_db, args.id.as_deref())
            }
            Commands::Set(args) => Self::set_default_handler(
                mime_db,
                desktop_entry_db,
                args.desktop_entry.as_str(),
                &args
                    .mime_types
                    .iter()
                    .map(AsRef::as_ref)
                    .collect::<Vec<_>>(),
                args.dry_run,
            ),
            Commands::Reset(args) => Self::reset_mime_types(
                mime_db,
                &args
                    .mime_types
                    .iter()
                    .map(AsRef::as_ref)
                    .collect::<Vec<_>>(),
                args.dry_run,
            ),
            Commands::Configuration => Self::get_configuration(mime_db, desktop_entry_db),
        }
    }

    fn get_mime_types(mime_db: &MimeAssociations) -> CommandOutput {
        let mut mime_types = mime_db.mime_types();
        mime_types.sort();

        CommandOutput::MimeTypes(
            mime_types
                .into_iter()
                .map(|m| MimeTypesCommandOutput {
                    mime_type: m.clone(),
                })
                .collect::<Vec<_>>(),
        )
    }

    fn get_mime_type(
        mime_db: &MimeAssociations,
        desktop_entry_db: &DesktopEntries,
        id: Option<&str>,
    ) -> CommandOutput {
        let Some(id) = id else {
            panic!("No mime type provded.");
        };

        let Ok(mime_type) = MimeType::parse(id) else {
            panic!("\"{}\" is not a valid mime type identifier", id);
        };

        let mut output = vec![];
        for mt_match in mime_db.find_matching_mimetypes(&mime_type) {
            output.push(Self::get_single_mime_type(
                mime_db,
                desktop_entry_db,
                mt_match,
            ));
        }

        CommandOutput::MimeType(output)
    }

    fn get_single_mime_type(
        mime_db: &MimeAssociations,
        desktop_entry_db: &DesktopEntries,
        mime_type: &MimeType,
    ) -> MimeTypeCommandOutput {
        let desktop_entries = desktop_entry_db.get_desktop_entries_for_mimetype(mime_type);
        let default_handler = mime_db.assigned_application_for(mime_type);

        let mut output = MimeTypeCommandOutput {
            mime_type: mime_type.clone(),
            handler_info: vec![],
        };

        for entry in desktop_entries {
            output.handler_info.push(MimeTypeCommandHandlerInfo {
                desktop_entry: entry.id().clone(),
                is_default_handler: Some(entry.id()) == default_handler,
            });
        }

        output
    }

    fn get_applications(
        mime_db: &MimeAssociations,
        desktop_entry_db: &DesktopEntries,
    ) -> CommandOutput {
        CommandOutput::Applications(
            desktop_entry_db
                .desktop_entries()
                .iter()
                .map(|desktop_entry| {
                    let mut mime_types = desktop_entry.mime_types().clone();
                    mime_types.sort();

                    let mime_info = mime_types
                        .iter()
                        .map(|mime_type| {
                            let is_handler = mime_db.assigned_application_for(mime_type)
                                == Some(desktop_entry.id());
                            MimeInfo {
                                mime_type: mime_type.clone(),
                                is_default_handler: is_handler,
                            }
                        })
                        .collect::<Vec<_>>();

                    ApplicationCommandOutput {
                        desktop_entry: desktop_entry.id().clone(),
                        mime_info,
                    }
                })
                .collect::<Vec<_>>(),
        )
    }

    fn get_application(
        mime_db: &MimeAssociations,
        desktop_entry_db: &DesktopEntries,
        id: Option<&str>,
    ) -> CommandOutput {
        let Some(id) = id else {
            panic!("No desktop entry id provded.");
        };

        let Some(desktop_entry) = lookup_desktop_entry(desktop_entry_db, id) else {
            panic!("\"{}\" does not appear to be an installed application", id);
        };

        let mut mime_types = desktop_entry.mime_types().clone();
        mime_types.sort();

        let mime_info = mime_types
            .iter()
            .map(|mime_type| {
                let is_handler =
                    mime_db.assigned_application_for(mime_type) == Some(desktop_entry.id());
                MimeInfo {
                    mime_type: mime_type.clone(),
                    is_default_handler: is_handler,
                }
            })
            .collect::<Vec<_>>();

        CommandOutput::Application(ApplicationCommandOutput {
            desktop_entry: desktop_entry.id().clone(),
            mime_info,
        })
    }

    fn set_default_handler(
        mime_db: &mut MimeAssociations,
        desktop_entry_db: &DesktopEntries,
        desktop_entry_id: &str,
        mime_types: &[&str],
        dry_run: bool,
    ) -> CommandOutput {
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

        let mut output = SetDefaultHandlerCommandOutput {
            desktop_entry: desktop_entry.id().clone(),
            mime_types: vec![],
        };

        for mime_type in resolved_mime_types.iter() {
            match mime_db.set_default_handler_for_mime_type(mime_type, desktop_entry) {
                Ok(_) => output.mime_types.push(mime_type.clone()),
                Err(e) => panic!(
                    "Failed to assign {} to {}, error: {:?}",
                    mime_type,
                    desktop_entry.id(),
                    e
                ),
            }
        }

        // persist the changes...
        if !dry_run {
            if let Err(e) = mime_db.save() {
                panic!("Unable to save changes: {:?}", e);
            }
        }

        CommandOutput::Set(output)
    }

    fn reset_mime_types(
        mime_db: &mut MimeAssociations,
        mime_types: &[&str],
        dry_run: bool,
    ) -> CommandOutput {
        // map input mime type strings to mime types
        let mut resolved_mime_types: Vec<MimeType> = vec![];
        for mime_type in mime_types {
            if let Ok(mime_type) = MimeType::parse(mime_type) {
                resolved_mime_types.push(mime_type);
            } else {
                panic!("\"{}\" is not a valid mime type identifier", mime_type);
            }
        }

        let mut output = ResetDefaultHandlerCommandOutput { mime_types: vec![] };

        for mime_type in resolved_mime_types.into_iter() {
            match mime_db.remove_assigned_applications_for(&mime_type) {
                Ok(_) => output.mime_types.push(mime_type),
                Err(e) => panic!(
                    "Failed to reset default applicaiton assignment for \"{}\", error: {:?}",
                    &mime_type, e
                ),
            }
        }

        // persist the changes...
        if !dry_run {
            if let Err(e) = mime_db.save() {
                panic!("Unable to save changes: {:?}", e);
            }
        }

        CommandOutput::Reset(output)
    }

    fn get_configuration(
        mime_db: &MimeAssociations,
        desktop_entry_db: &DesktopEntries,
    ) -> CommandOutput {
        let mime_association_scope_paths = mime_db
            .sources()
            .iter()
            .map(PathBuf::from)
            .collect::<Vec<_>>();
        let desktop_entry_scope_paths = desktop_entry_db
            .sources()
            .iter()
            .map(PathBuf::from)
            .collect::<Vec<_>>();

        CommandOutput::Configuration(ConfigurationCommandOutput {
            mime_association_scope_paths,
            desktop_entry_scope_paths,
        })
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
    /// If set, make no changes, just display what would be assigned
    #[arg(short, long)]
    dry_run: bool,
    desktop_entry: String,
    mime_types: Vec<String>,
}

#[derive(Args)]
struct ResetCommandArgs {
    /// If set, make no changes, just display what would be reset
    #[arg(short, long)]
    dry_run: bool,
    mime_types: Vec<String>,
}
