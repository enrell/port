use std::io;
use std::process::ExitCode;
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
use ports::get_open_ports;

#[derive(Parser)]
#[command(name = "port")]
#[command(about = "TUI port manager for Linux")]
struct Args {
 #[arg(short, long)]
 tui: bool,
 port: Option<u16>,
}

fn main() -> io::Result<ExitCode> {
 let args = Args::parse();

 if let Some(port) = args.port {
 if let Err(e) = kill_by_port(port) {
 eprintln!("Error: {}", e);
 return Ok(ExitCode::FAILURE);
 }
 return Ok(ExitCode::SUCCESS);
 }

 terminal::enable_raw_mode()?;
 io::stdout().execute(EnterAlternateScreen)?;
 let backend = CrosstermBackend::new(io::stdout());
 let mut terminal = Terminal::new(backend)?;
 let mut app = App::new();

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
 Ok(ExitCode::SUCCESS)
}

fn kill_by_port(port: u16) -> io::Result<()> {
 let all_ports = get_open_ports()?;
 if let Some(port_info) = all_ports.iter().find(|p| p.port == port) {
 return process::kill_process(port_info.pid);
 }
 Err(io::Error::new(io::ErrorKind::NotFound, format!("No process found on port {}", port)))
}
