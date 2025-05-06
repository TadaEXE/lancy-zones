use std::path::Path;

use clap::{Args, Parser, Subcommand, ValueEnum};
use lancy_zones::config;

mod example;

#[derive(Debug, Parser)]
#[command(name = "lancy-zones-cfg")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Debug, Subcommand)]
enum Commands {
    #[command()]
    List {},
    #[command()]
    Init {},
    #[command(arg_required_else_help = true)]
    Add {
        x: i16,
        y: i16,
        width: u16,
        height: u16,
    },
    #[command(arg_required_else_help = true)]
    Remove {},
}

fn main() {
    let args = Cli::parse();

    let path = Path::new("~/.config/lancy-zones/config.json");

    match args.command {
        Commands::List {} => {
            let config = config::load_cfg_file(&path);
            println!("{}", config);
        }
        Commands::Add {x, y, width, height} => {
            println!("{} {} {}x{}", x, y, width, height);
        },
        Commands::Remove {} => todo!(),
        Commands::Init {} => todo!(),
    }
}
