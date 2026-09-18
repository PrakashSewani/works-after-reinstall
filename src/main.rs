mod catalog;
mod cli;
mod commands;
mod console;
mod discovery;
mod envvars;
mod files;
mod manifest;
mod paths;
mod prompt;
mod settings;
mod starter;
mod tui;
mod util;
mod win;
mod winget;

use anyhow::Result;
use clap::{CommandFactory, Parser};

use crate::cli::{Cli, Command, ExportArgs, SetupArgs, StarterArgs};
use crate::console::Console;
use crate::discovery::BundleChoice;

fn main() {
    let cli = Cli::parse();
    let outcome = match cli.command {
        None => run_default(),
        Some(Command::Ui) => tui::run(),
        Some(Command::Export(args)) => run_export(args),
        Some(Command::Setup(args)) => run_setup(args),
        Some(Command::Starter(args)) => run_starter(args),
    };

    if let Err(error) = outcome {
        eprintln!("\nerror: {error:#}");
        std::process::exit(1);
    }
}

/// No command given: open the interactive interface when there is a terminal to
/// draw in, otherwise simply describe the commands.
fn run_default() -> Result<()> {
    if tui::can_run() {
        return tui::run();
    }
    let mut command = Cli::command();
    command.print_help()?;
    println!();
    Ok(())
}

fn run_export(args: ExportArgs) -> Result<()> {
    commands::export(&args.output, &args.include, &args.skip, &Console::stdout())
}

fn run_setup(args: SetupArgs) -> Result<()> {
    let bundle = match discovery::resolve_requested_bundle(args.bundle)? {
        BundleChoice::Ready(bundle) => bundle,
        BundleChoice::NothingToApply => return Ok(()),
    };

    commands::setup(&bundle, args.dry_run, &args.only, &Console::stdout())
}

/// A fresh machine has nothing to export, so this writes a bundle that is
/// already usable: a starter set of apps, preferences and example config files.
fn run_starter(args: StarterArgs) -> Result<()> {
    if args.list {
        print!("{}", starter::listing());
        return Ok(());
    }

    let manifest_path = starter::scaffold(&args.dir, args.force)?;

    println!("Starter bundle written to {}\n", args.dir.display());
    println!(
        "  {}  what to install, with the options commented out",
        manifest_path.display()
    );
    println!(
        "  {}   example configs - fill them in, then uncomment the matching lines",
        args.dir.join("files").display()
    );
    println!();
    println!("Run `works-after-reinstall starter --list` to see the package ids you can drop in,");
    println!("then apply it:");
    println!("  works-after-reinstall setup \"{}\"", args.dir.display());
    Ok(())
}
