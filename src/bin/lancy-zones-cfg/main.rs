use clap::{Parser, Subcommand};

mod cmd_impl;
mod example;

use crate::cmd_impl::*;

#[derive(Debug, Parser)]
#[command(name = "lancy-zones-cfg")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Debug, Subcommand)]
enum Commands {
    #[command()]
    Info {},
    #[command()]
    Reinit {},
    #[command(arg_required_else_help = true)]
    CreateConfig { config_name: String },
    #[command(arg_required_else_help = true)]
    RemoveConfig { config_name: String },
    #[command(arg_required_else_help = true)]
    Assign {
        monitor_name: String,
        config_name: String,
    },
    #[command(arg_required_else_help = true)]
    Unassign { monitor_name: String },
    #[command(arg_required_else_help = true)]
    AddZone {
        config_name: String,
        zone_name: String,
        x: i16,
        y: i16,
        width: i16,
        height: i16,
    },
    // #[command(arg_required_else_help = true)]
    // AddZoneRel {
    //     config_name: String,
    //     zone_name: String,
    //     x: String,
    //     y: String,
    //     width: String,
    //     height: String,
    // },
    #[command(arg_required_else_help = true)]
    RemoveZone {
        config_name: String,
        zone_name: String,
    },
    #[command()]
    RefrashGlobalPos {},
    #[command()]
    RefrshSizes {},
}

fn main() {
    let args = Cli::parse();

    match args.command {
        Commands::Info {} => list_cmd(),
        Commands::Reinit {} => reinit_cmd(),
        Commands::CreateConfig { config_name } => create_config_cmd(&config_name),
        Commands::RemoveConfig { config_name } => remove_config_cmd(&config_name),
        Commands::AddZone {
            config_name,
            zone_name,
            x,
            y,
            width,
            height,
        } => add_zone_cmd(&config_name, &zone_name, x, y, width, height),
        Commands::RemoveZone { .. } => todo!(),
        Commands::Assign {
            monitor_name,
            config_name,
        } => assign_cmd(&monitor_name, &config_name),
        Commands::Unassign { monitor_name } => unassing_cmd(&monitor_name),
        Commands::RefrashGlobalPos {} => refresh_global_pos_cmd(),
        Commands::RefrshSizes {} => refresh_sizes_cmd(),
    }
}
