use std::error::Error;
use std::fs;
use std::path::PathBuf;

use clap::Parser;

use envio::{
    cli_args::{
        self, DbMigrateSubcommands, Language, LocalCommandTypes, LocalDockerSubcommands,
        ProjectPathsArgs,
    },
    commands,
    config_parsing::{self, entity_parsing, event_parsing},
    hbs_templating::codegen_templates::{
        entities_to_map, generate_templates, EventRecordTypeTemplate,
    },
    hbs_templating::{hbs_dir_generator::HandleBarsDirGenerator, init_templates::InitTemplates},
    linked_hashmap::{LinkedHashMap, RescriptRecordHierarchyLinkedHashMap, RescriptRecordKey},
    project_paths::{self, ParsedPaths},
};

use cli_args::{CommandLineArgs, CommandType, Template, ToProjectPathsArgs};
use include_dir::{include_dir, Dir};

static CODEGEN_STATIC_DIR: Dir<'_> = include_dir!("templates/static/codegen");
static BLANK_TEMPLATE_STATIC_SHARED_DIR: Dir<'_> =
    include_dir!("templates/static/blank_template/shared");
static BLANK_TEMPLATE_STATIC_RESCRIPT_DIR: Dir<'_> =
    include_dir!("templates/static/blank_template/rescript");
static BLANK_TEMPLATE_STATIC_TYPESCRIPT_DIR: Dir<'_> =
    include_dir!("templates/static/blank_template/typescript");
static BLANK_TEMPLATE_STATIC_JAVASCRIPT_DIR: Dir<'_> =
    include_dir!("templates/static/blank_template/javascript");
static GREETER_TEMPLATE_STATIC_SHARED_DIR: Dir<'_> =
    include_dir!("templates/static/greeter_template/shared");
static GREETER_TEMPLATE_STATIC_RESCRIPT_DIR: Dir<'_> =
    include_dir!("templates/static/greeter_template/rescript");
static GREETER_TEMPLATE_STATIC_TYPESCRIPT_DIR: Dir<'_> =
    include_dir!("templates/static/greeter_template/typescript");
static GREETER_TEMPLATE_STATIC_JAVASCRIPT_DIR: Dir<'_> =
    include_dir!("templates/static/greeter_template/javascript");
static ERC20_TEMPLATE_STATIC_SHARED_DIR: Dir<'_> =
    include_dir!("templates/static/erc20_template/shared");
static ERC20_TEMPLATE_STATIC_RESCRIPT_DIR: Dir<'_> =
    include_dir!("templates/static/erc20_template/rescript");
static ERC20_TEMPLATE_STATIC_TYPESCRIPT_DIR: Dir<'_> =
    include_dir!("templates/static/erc20_template/typescript");
static ERC20_TEMPLATE_STATIC_JAVASCRIPT_DIR: Dir<'_> =
    include_dir!("templates/static/erc20_template/javascript");
static INIT_TEMPLATES_SHARED_DIR: Dir<'_> = include_dir!("templates/dynamic/init_templates/shared");

fn main() -> Result<(), Box<dyn Error>> {
    let command_line_args = CommandLineArgs::parse();

    match command_line_args.command {
        CommandType::Init(init_args) => {
            //get_init_args_interactive opens an interactive cli for required args to be selected
            //if they haven't already been
            let args = init_args.get_init_args_interactive()?;
            let project_root_path = PathBuf::from(&args.directory);
            // check that project_root_path exists
            let project_dir = project_paths::path_utils::NewDir::new(project_root_path.clone())?;

            let hbs_template = InitTemplates::new(project_dir.root_dir_name, &args.language);
            let hbs_generator = HandleBarsDirGenerator::new(
                &INIT_TEMPLATES_SHARED_DIR,
                &hbs_template,
                &project_root_path,
            );

            match args.template {
                Template::Blank => {
                    //Copy in the relevant language specific blank template files
                    match &args.language {
                        Language::Rescript => {
                            BLANK_TEMPLATE_STATIC_RESCRIPT_DIR.extract(&project_root_path)?;
                        }
                        Language::Typescript => {
                            BLANK_TEMPLATE_STATIC_TYPESCRIPT_DIR.extract(&project_root_path)?;
                        }
                        Language::Javascript => {
                            BLANK_TEMPLATE_STATIC_JAVASCRIPT_DIR.extract(&project_root_path)?;
                        }
                    }
                    //Copy in the rest of the shared blank template files
                    BLANK_TEMPLATE_STATIC_SHARED_DIR.extract(&project_root_path)?;
                    hbs_generator.generate_hbs_templates()?;
                }
                Template::Greeter => {
                    //Copy in the relevant language specific greeter files
                    match &args.language {
                        Language::Rescript => {
                            GREETER_TEMPLATE_STATIC_RESCRIPT_DIR.extract(&project_root_path)?;
                        }
                        Language::Typescript => {
                            GREETER_TEMPLATE_STATIC_TYPESCRIPT_DIR.extract(&project_root_path)?;
                        }
                        Language::Javascript => {
                            GREETER_TEMPLATE_STATIC_JAVASCRIPT_DIR.extract(&project_root_path)?;
                        }
                    }
                    //Copy in the rest of the shared greeter files
                    GREETER_TEMPLATE_STATIC_SHARED_DIR.extract(&project_root_path)?;
                }
                Template::Erc20 => {
                    //Copy in the relevant js flavor specific greeter files
                    match &args.language {
                        Language::Rescript => {
                            ERC20_TEMPLATE_STATIC_RESCRIPT_DIR.extract(&project_root_path)?;
                        }
                        Language::Typescript => {
                            ERC20_TEMPLATE_STATIC_TYPESCRIPT_DIR.extract(&project_root_path)?;
                        }
                        Language::Javascript => {
                            ERC20_TEMPLATE_STATIC_JAVASCRIPT_DIR.extract(&project_root_path)?;
                        }
                    }
                    //Copy in the rest of the shared greeter files
                    ERC20_TEMPLATE_STATIC_SHARED_DIR.extract(&project_root_path)?;
                }
            }

            println!("Project template ready");
            println!("Running codegen");

            let parsed_paths = ParsedPaths::new(init_args.to_project_paths_args())?;
            let project_paths = &parsed_paths.project_paths;
            commands::codegen::run_codegen_command_sequence(&project_paths)
        }

        CommandType::Codegen(args) => {
            let parsed_paths = ParsedPaths::new(args.to_project_paths_args())?;
            let project_paths = &parsed_paths.project_paths;

            fs::create_dir_all(&project_paths.generated)?;

            let entity_types = entity_parsing::get_entity_record_types_from_schema(&parsed_paths)?;

            let contract_types = event_parsing::get_contract_types_from_config(
                &parsed_paths,
                &entities_to_map(entity_types.clone()),
            )?;

            let chain_config_templates =
                config_parsing::convert_config_to_chain_configs(&parsed_paths)?;

            let sync_config = config_parsing::convert_config_to_sync_config(&parsed_paths)?;

            //NOTE: This structure is no longer used int event parsing since it has been refactored
            //to use an inline tuple type for parsed structs. However this is being left until it
            //is decided to completely remove the need for subrecords in which case the entire
            //linked_hashmap module can be removed.
            let rescript_subrecord_dependencies: LinkedHashMap<
                RescriptRecordKey,
                EventRecordTypeTemplate,
            > = RescriptRecordHierarchyLinkedHashMap::new();

            let sub_record_dependencies: Vec<EventRecordTypeTemplate> =
                rescript_subrecord_dependencies
                    .iter()
                    .collect::<Vec<EventRecordTypeTemplate>>();

            CODEGEN_STATIC_DIR.extract(&project_paths.generated)?;

            generate_templates(
                sub_record_dependencies,
                contract_types,
                chain_config_templates,
                entity_types,
                &project_paths,
                sync_config,
            )?;

            commands::codegen::run_codegen_command_sequence(project_paths)?;

            Ok(())
        }
        CommandType::Local(local_commands) => {
            let parsed_paths = ParsedPaths::new(ProjectPathsArgs::default())?;
            let project_paths = &parsed_paths.project_paths;
            match local_commands {
                LocalCommandTypes::Docker(subcommand) => match subcommand {
                    LocalDockerSubcommands::Up => {
                        commands::docker::docker_compose_up_d(project_paths)?;
                    }
                    LocalDockerSubcommands::Down => {
                        commands::docker::docker_compose_down_v(project_paths)?;
                    }
                },
                LocalCommandTypes::DbMigrate(subcommand) => match subcommand {
                    DbMigrateSubcommands::Up => {
                        commands::db_migrate::run_up_migrations(project_paths)?;
                    }

                    DbMigrateSubcommands::Down => {
                        commands::db_migrate::run_drop_schema(project_paths)?;
                    }

                    DbMigrateSubcommands::Setup => {
                        commands::db_migrate::run_db_setup(project_paths)?;
                    }
                },
            }
            Ok(())
        }
        CommandType::PrintAllHelp {} => {
            clap_markdown::print_help_markdown::<CommandLineArgs>();
            Ok(())
        }
    }
}
