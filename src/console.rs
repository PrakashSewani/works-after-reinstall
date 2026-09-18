use std::sync::mpsc::Sender;

use crate::cli::Component;

/// Where progress output goes: straight to the terminal for the CLI, or down a
/// channel when the TUI is driving.
#[derive(Clone)]
pub enum Console {
    Stdout,
    Channel(Sender<String>),
}

impl Console {
    pub fn stdout() -> Self {
        Console::Stdout
    }

    pub fn channel(sender: Sender<String>) -> Self {
        Console::Channel(sender)
    }

    pub fn line(&self, text: impl AsRef<str>) {
        match self {
            Console::Stdout => println!("{}", text.as_ref()),
            Console::Channel(sender) => {
                let _ = sender.send(text.as_ref().to_string());
            }
        }
    }

    pub fn blank(&self) {
        self.line("");
    }

    pub fn section(&self, component: Component) {
        self.line(format!("[{}]", component.label()));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::mpsc::channel;

    #[test]
    fn sends_lines_through_the_channel() {
        let (sender, receiver) = channel();
        let console = Console::channel(sender);

        console.line("first");
        console.section(Component::Apps);
        console.blank();

        assert_eq!(
            receiver.try_iter().collect::<Vec<String>>(),
            vec!["first".to_string(), "[apps]".to_string(), String::new()]
        );
    }
}
