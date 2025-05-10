use std::{
    fs,
    path::{self, Path, PathBuf},
};

use clap::{Args, Parser, Subcommand, ValueEnum};
use lancy_zones::config;

use x11rb::connection::Connection;

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
    Show {},
    #[command()]
    Reinit {},
    #[command(arg_required_else_help = true)]
    Add {
        monitor_name: String,
        x: i16,
        y: i16,
        width: u16,
        height: u16,
    },
    #[command(arg_required_else_help = true)]
    Remove {},
}

fn list_cmd(path: &Path) {
    let config = config::load_cfg_file(&path);
    println!("{}", config);
}

fn reinit_cmd(path: &Path) {
    if path.exists() {
        fs::remove_file(&path).expect(&std::format!(
            "Could not delete exisiting config file at {}",
            path.to_str().unwrap()
        ));
    }
    let (conn, screen_num) = x11rb::connect(None).unwrap();
    let screen = &conn.setup().roots[screen_num];
    config::init_cfg_file(&path, &conn, screen.root);
}

fn main() {
    let args = Cli::parse();

    let path = Path::new("~/.config/lancy-zones/config.json");

    match args.command {
        Commands::List {} => list_cmd(path),
        Commands::Show {} => todo!(),
        Commands::Add {
            x,
            y,
            width,
            height,
        } => {
            println!("{} {} {}x{}", x, y, width, height);
        }
        Commands::Remove {} => todo!(),
        Commands::Reinit {} => reinit_cmd(path),
    }
}
