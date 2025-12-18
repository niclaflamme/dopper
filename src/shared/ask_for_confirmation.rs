use std::io::{self, Write};

pub fn ask_for_confirmation(message: &str) -> bool {
    print!("{} (y/N): ", message);
    io::stdout().flush().expect("Failed to flush stdout");

    let mut input = String::new();
    io::stdin()
        .read_line(&mut input)
        .expect("Failed to read input");

    input.trim().eq_ignore_ascii_case("y") || input.trim().eq_ignore_ascii_case("yes")
}