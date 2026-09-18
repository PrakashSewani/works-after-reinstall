use std::path::PathBuf;

use clap::{Args, Parser, Subcommand, ValueEnum};

#[derive(Debug, Parser)]
#[command(
    name = "works-after-reinstall",
    version,
    about = "Your Windows setup, rebuilt after every reinstall.",
    after_help = "Run without a command to open the interactive interface."
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Command>,
}

#[derive(Debug, Subcommand)]
pub enum Command {
    /// Capture this machine into a portable config bundle
    Export(ExportArgs),
    /// Rebuild this machine from a config bundle
    Setup(SetupArgs),
    /// Create a starter bundle you can edit and apply
    Starter(StarterArgs),
    /// Open the interactive interface
    Ui,
}

#[derive(Debug, Args)]
pub struct StarterArgs {
    /// Folder to write the starter bundle into
    #[arg(value_name = "DIR", default_value = "works-after-reinstall-bundle")]
    pub dir: PathBuf,

    /// Overwrite an existing bundle in that folder
    #[arg(long)]
    pub force: bool,

    /// Print the apps you can pick from instead of writing a bundle
    #[arg(long)]
    pub list: bool,
}

#[derive(Debug, Args)]
pub struct ExportArgs {
    /// Directory to write the bundle into
    #[arg(
        short,
        long,
        value_name = "DIR",
        default_value = "works-after-reinstall-bundle"
    )]
    pub output: PathBuf,

    /// Extra file to include in the bundle (repeatable)
    #[arg(short, long = "include", value_name = "PATH")]
    pub include: Vec<String>,

    /// Leave a component out of the bundle (repeatable)
    #[arg(long, value_name = "COMPONENT")]
    pub skip: Vec<Component>,
}

#[derive(Debug, Args)]
pub struct SetupArgs {
    /// Bundle directory or manifest file to apply; without it, nearby bundles are searched for
    #[arg(value_name = "BUNDLE")]
    pub bundle: Option<PathBuf>,

    /// Show what would change without changing anything
    #[arg(long)]
    pub dry_run: bool,

    /// Apply only these components (repeatable)
    #[arg(long, value_name = "COMPONENT")]
    pub only: Vec<Component>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, ValueEnum)]
pub enum Component {
    Apps,
    Environment,
    Windows,
    Files,
}

impl Component {
    pub const ALL: [Component; 4] = [
        Component::Apps,
        Component::Environment,
        Component::Windows,
        Component::Files,
    ];

    /// Lower-case name used on the command line and in bundle output.
    pub fn label(self) -> &'static str {
        match self {
            Component::Apps => "apps",
            Component::Environment => "environment",
            Component::Windows => "windows",
            Component::Files => "files",
        }
    }

    /// Human-friendly name used by the interactive interface.
    pub fn title(self) -> &'static str {
        match self {
            Component::Apps => "Applications",
            Component::Environment => "Environment variables",
            Component::Windows => "Windows settings",
            Component::Files => "Configuration files",
        }
    }
}

pub fn selected(only: &[Component], skip: &[Component], component: Component) -> bool {
    if !only.is_empty() {
        return only.contains(&component);
    }
    !skip.contains(&component)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn skip_wins_when_no_only_given() {
        assert!(selected(&[], &[], Component::Apps));
        assert!(!selected(&[], &[Component::Apps], Component::Apps));
        assert!(selected(&[], &[Component::Apps], Component::Files));
    }

    #[test]
    fn only_takes_precedence_over_skip() {
        assert!(selected(
            &[Component::Files],
            &[Component::Files],
            Component::Files
        ));
        assert!(!selected(&[Component::Files], &[], Component::Apps));
    }

    #[test]
    fn every_component_has_both_names() {
        for component in Component::ALL {
            assert!(!component.label().is_empty());
            assert!(!component.title().is_empty());
            assert_ne!(component.label(), component.title());
        }
    }

    #[test]
    fn no_command_is_allowed() {
        let cli = Cli::try_parse_from(["works-after-reinstall"]).unwrap();
        assert!(cli.command.is_none());
    }

    #[test]
    fn setup_bundle_is_optional() {
        let cli = Cli::try_parse_from(["works-after-reinstall", "setup"]).unwrap();
        match cli.command {
            Some(Command::Setup(args)) => assert!(args.bundle.is_none()),
            other => panic!("unexpected command: {other:?}"),
        }
    }
}
