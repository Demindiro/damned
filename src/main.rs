#![forbid(unused_must_use)]

mod script;

use crossterm::{
    ExecutableCommand, event, execute,
    style::{Color, Print, ResetColor, SetBackgroundColor, SetForegroundColor},
    terminal::{self, EnterAlternateScreen, LeaveAlternateScreen},
};
use std::io::{self, Write};

struct Args {
    file: String,
}

fn enable_tui() {
    let _ = execute!(io::stdout(), EnterAlternateScreen);
    let _ = terminal::enable_raw_mode();
}

fn disable_tui() {
    let _ = terminal::disable_raw_mode();
    let _ = execute!(io::stdout(), LeaveAlternateScreen);
}

fn main() {
    let mut args = std::env::args();
    let _ = args.next();
    let script = args.next().unwrap();
    let script = std::fs::read(&script).unwrap();
    let mut vm = script::create_root_vm(&mut args);
    vm.stream_append(&script);

    enable_tui();
    let hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        disable_tui();
        (hook)(info);
    }));
    vm.run().unwrap();
    disable_tui();
}
