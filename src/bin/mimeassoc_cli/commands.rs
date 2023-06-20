use std::path::PathBuf;

use clap::{Args, Subcommand};
use mimeassoc::mime_info::MimeTypeInfoStore;

use super::command_output::*;
use mimeassoc::desktop_entry::*;
use mimeassoc::mime_type::*;

#[derive(Args)]
pub struct MimeTypeCommandArgs {
    id: Option<String>,
}

#[derive(Args)]
pub struct ApplicationCommandArgs {
    id: Option<String>,
}

#[derive(Args)]
pub struct SetCommandArgs {
    /// If set, make no changes, just display what would be assigned
    #[arg(short, long)]
    dry_run: bool,
    desktop_entry: String,
    mime_types: Vec<String>,
}

#[derive(Args)]
pub struct ResetCommandArgs {
    /// If set, make no changes, just display what would be reset
    #[arg(short, long)]
    dry_run: bool,
    mime_types: Vec<String>,
}

////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////

#[derive(Subcommand)]
pub enum Commands {
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
        mime_associations_store: &mut MimeAssociationStore,
        desktop_entry_store: &DesktopEntryStore,
        mime_info_store: &MimeTypeInfoStore,
    ) -> CommandOutput {
        match self {
            Commands::MimeTypes => Self::get_mime_types(mime_associations_store, mime_info_store),
            Commands::MimeType(args) => Self::get_mime_type(
                mime_associations_store,
                desktop_entry_store,
                mime_info_store,
                args.id.as_deref(),
            ),
            Commands::Applications => {
                Self::get_applications(mime_associations_store, desktop_entry_store)
            }
            Commands::Application(args) => Self::get_application(
                mime_associations_store,
                desktop_entry_store,
                args.id.as_deref(),
            ),
            Commands::Set(args) => Self::set_default_handler(
                mime_associations_store,
                desktop_entry_store,
                args.desktop_entry.as_str(),
                &args
                    .mime_types
                    .iter()
                    .map(AsRef::as_ref)
                    .collect::<Vec<_>>(),
                args.dry_run,
            ),
            Commands::Reset(args) => Self::reset_mime_types(
                mime_associations_store,
                &args
                    .mime_types
                    .iter()
                    .map(AsRef::as_ref)
                    .collect::<Vec<_>>(),
                args.dry_run,
            ),
            Commands::Configuration => {
                Self::get_configuration(mime_associations_store, desktop_entry_store)
            }
        }
    }

    fn get_mime_types(
        mime_associations_store: &MimeAssociationStore,
        mime_info_store: &MimeTypeInfoStore,
    ) -> CommandOutput {
        let mut mime_types = mime_associations_store.mime_types();
        mime_types.sort();

        CommandOutput::MimeTypes(
            mime_types
                .into_iter()
                .map(|m| MimeTypesCommandOutput {
                    mime_type: m.clone(),
                    mime_info: mime_info_store.get_info_for_mime_type(m).cloned(),
                })
                .collect::<Vec<_>>(),
        )
    }

    fn get_mime_type(
        mime_associations_store: &MimeAssociationStore,
        desktop_entry_store: &DesktopEntryStore,
        mime_info_store: &MimeTypeInfoStore,
        id: Option<&str>,
    ) -> CommandOutput {
        let Some(id) = id else {
            panic!("No mime type provded.");
        };

        let Ok(mime_type) = MimeType::parse(id) else {
            panic!("\"{}\" is not a valid mime type identifier", id);
        };

        let mut output = vec![];
        for mt_match in mime_associations_store.find_matching_mimetypes(&mime_type) {
            output.push(Self::get_single_mime_type(
                mime_associations_store,
                desktop_entry_store,
                mime_info_store,
                mt_match,
            ));
        }

        output.sort_by(|a, b| a.mime_type.cmp(&b.mime_type));

        CommandOutput::MimeType(output)
    }

    fn get_single_mime_type(
        mime_associations_store: &MimeAssociationStore,
        desktop_entry_store: &DesktopEntryStore,
        mime_info_store: &MimeTypeInfoStore,
        mime_type: &MimeType,
    ) -> MimeTypeCommandOutput {
        let desktop_entries = desktop_entry_store.get_desktop_entries_for_mimetype(mime_type);
        let default_handler = mime_associations_store.assigned_application_for(mime_type);

        let mut output = MimeTypeCommandOutput {
            mime_type: mime_type.clone(),
            mime_info: mime_info_store.get_info_for_mime_type(mime_type).cloned(),
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
        mime_associations_store: &MimeAssociationStore,
        desktop_entry_store: &DesktopEntryStore,
    ) -> CommandOutput {
        CommandOutput::Applications(
            desktop_entry_store
                .desktop_entries()
                .iter()
                .map(|desktop_entry| {
                    let mut mime_types = desktop_entry.mime_types().clone();
                    mime_types.sort();

                    let mime_info = mime_types
                        .iter()
                        .map(|mime_type| {
                            let is_handler = mime_associations_store
                                .assigned_application_for(mime_type)
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
        mime_associations_store: &MimeAssociationStore,
        desktop_entry_store: &DesktopEntryStore,
        id: Option<&str>,
    ) -> CommandOutput {
        let Some(id) = id else {
            panic!("No desktop entry id provded.");
        };

        let Some(desktop_entry) = lookup_desktop_entry(desktop_entry_store, id) else {
            panic!("\"{}\" does not appear to be an installed application", id);
        };

        let mut mime_types = desktop_entry.mime_types().clone();
        mime_types.sort();

        let mime_info = mime_types
            .iter()
            .map(|mime_type| {
                let is_handler = mime_associations_store.assigned_application_for(mime_type)
                    == Some(desktop_entry.id());
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
        mime_associations_store: &mut MimeAssociationStore,
        desktop_entry_store: &DesktopEntryStore,
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
        let Some(desktop_entry) = lookup_desktop_entry(desktop_entry_store, desktop_entry_id) else {
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
            match mime_associations_store
                .set_default_handler_for_mime_type(mime_type, desktop_entry)
            {
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
            if let Err(e) = mime_associations_store.save() {
                panic!("Unable to save changes: {:?}", e);
            }
        }

        CommandOutput::Set(output)
    }

    fn reset_mime_types(
        mime_associations_store: &mut MimeAssociationStore,
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

        let mut output = ResetDefaultHandlerCommandOutput {
            reset_mime_types: vec![],
        };

        for mime_type in resolved_mime_types.into_iter() {
            match mime_associations_store.remove_assigned_applications_for(&mime_type) {
                Ok(_) => output.reset_mime_types.push(mime_type),
                Err(e) => panic!(
                    "Failed to reset default applicaiton assignment for \"{}\", error: {:?}",
                    &mime_type, e
                ),
            }
        }

        // persist the changes...
        if !dry_run {
            if let Err(e) = mime_associations_store.save() {
                panic!("Unable to save changes: {:?}", e);
            }
        }

        CommandOutput::Reset(output)
    }

    fn get_configuration(
        mime_associations_store: &MimeAssociationStore,
        desktop_entry_store: &DesktopEntryStore,
    ) -> CommandOutput {
        let mime_association_scope_paths = mime_associations_store
            .sources()
            .iter()
            .map(PathBuf::from)
            .collect::<Vec<_>>();
        let desktop_entry_scope_paths = desktop_entry_store
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
