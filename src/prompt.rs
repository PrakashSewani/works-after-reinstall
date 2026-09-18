use std::io::{IsTerminal, Write};

pub fn is_interactive() -> bool {
    std::io::stdin().is_terminal() && std::io::stdout().is_terminal()
}

pub fn confirm(question: &str, default_yes: bool) -> anyhow::Result<bool> {
    let hint = if default_yes { "[Y/n]" } else { "[y/N]" };
    print!("{question} {hint} ");
    std::io::stdout().flush()?;

    match read_line() {
        Some(answer) => Ok(interpret(&answer, default_yes)),
        None => Ok(default_yes),
    }
}

pub fn choose(question: &str, items: &[String]) -> Option<usize> {
    println!("{question}");
    for (index, item) in items.iter().enumerate() {
        println!("  {}. {item}", index + 1);
    }
    print!("Choose 1-{} (or press Enter to cancel): ", items.len());
    std::io::stdout().flush().ok()?;

    parse_choice(&read_line()?, items.len())
}

fn interpret(answer: &str, default_yes: bool) -> bool {
    match answer.trim().to_ascii_lowercase().as_str() {
        "" => default_yes,
        "y" | "yes" => true,
        _ => false,
    }
}

fn parse_choice(answer: &str, count: usize) -> Option<usize> {
    let choice: usize = answer.trim().parse().ok()?;
    if (1..=count).contains(&choice) {
        Some(choice - 1)
    } else {
        None
    }
}

fn read_line() -> Option<String> {
    let mut line = String::new();
    match std::io::stdin().read_line(&mut line) {
        Ok(0) | Err(_) => None,
        Ok(_) => Some(line),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn confirm_accepts_the_usual_yes_forms() {
        assert!(interpret("y", false));
        assert!(interpret("YES", false));
        assert!(interpret(" yes \r\n", false));
    }

    #[test]
    fn confirm_falls_back_to_the_default_on_anything_else() {
        assert!(!interpret("", false));
        assert!(interpret("\n", true));
        assert!(!interpret("n", true));
        assert!(!interpret("maybe", true));
    }

    #[test]
    fn choose_maps_a_number_to_an_index() {
        assert_eq!(parse_choice("1", 3), Some(0));
        assert_eq!(parse_choice(" 2 ", 3), Some(1));
        assert_eq!(parse_choice("3", 3), Some(2));
    }

    #[test]
    fn choose_rejects_anything_out_of_range() {
        assert_eq!(parse_choice("0", 3), None);
        assert_eq!(parse_choice("4", 3), None);
        assert_eq!(parse_choice("", 3), None);
        assert_eq!(parse_choice("banana", 3), None);
    }
}
