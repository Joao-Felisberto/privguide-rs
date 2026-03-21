use std::{collections::HashMap, error::Error};

use clap::{Parser, Subcommand};
use privguide::database::{Database, MemDatabase};
use privguide_cli::cmd::analyse::analyse;
use privguide_cli::cmd::code_check::analyse as anal_src;
    
#[derive(Parser)]
#[command(name = "privguide")]
#[command(about = "Analyse compliance of system descriptions with regulations", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
#[allow(clippy::upper_case_acronyms)]
enum Commands {
    SCHEMA {
        schema: String,
    },
    ANALYSE {
        #[arg(short, long, default_value="./.privguide")]
        dir: String,
    },
    CODE_CHECK {
        #[arg(short, long, default_value="./.privguide")]
        dir: String,
        #[arg(short, long)]
        code_dir: String
    },
}

fn main() {
    let cli = Cli::parse();

    match cli.command {
        Commands::SCHEMA { schema } => {
            println!("Printing schema '{}'", schema);
        }
        Commands::ANALYSE { dir } => {
            println!("Running analysis with rules in '{}'", dir);
            analyse(&dir);
        }
        Commands::CODE_CHECK { dir, code_dir } => {
            println!("Analysing code in '{}' with rules in '{}'", code_dir, dir);
            anal_src(&dir, &code_dir);
        }
    }
}
