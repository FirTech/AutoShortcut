use console::style;

pub enum ConsoleType {
    Info,
    Success,
    Warning,
    Error,
}

pub fn writeConsole(consoleType: ConsoleType, message: &str) {
    let title = match &consoleType {
        ConsoleType::Info => style("Info   ").cyan(),
        ConsoleType::Success => style("Success").green(),
        ConsoleType::Warning => style("Warning").yellow(),
        ConsoleType::Error => style("Err    ").red(),
    };
    println!("  {}      {}", &title, message);
}
