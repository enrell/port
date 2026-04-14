use std::io;
use crossterm::{
 terminal::{self, EnterAlternateScreen, LeaveAlternateScreen},
 ExecutableCommand,
};
use ratatui::backend::CrosstermBackend;
use ratatui::Terminal;
use clap::Parser;

mod app;
mod events;
mod filter;
mod ports;
mod process;
mod ui;

use app::App;

#[derive(Parser)]
#[command(name = "port")]
#[command(about = "TUI port manager for Linux")]
struct Args {
 #[arg(short, long, default_value_t = false)]
 all: bool,
}

fn main() -> io::Result<()> {
 let args = Args::parse();

 terminal::enable_raw_mode()?;
 io::stdout().execute(EnterAlternateScreen)?;
 let backend = CrosstermBackend::new(io::stdout());
 let mut terminal = Terminal::new(backend)?;
 let mut app = App::new(args.all);

 loop {
 terminal.draw(|frame| ui::render(frame, &app))?;

 if events::handle(&mut app)? {
 break;
 }

 if app.should_quit {
 break;
 }
 }

 terminal::disable_raw_mode()?;
 io::stdout().execute(LeaveAlternateScreen)?;
 Ok(())
}
