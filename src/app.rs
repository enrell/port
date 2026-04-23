use crate::ports::{get_open_ports, PortInfo};
use crate::process::kill_docker_container;
use crate::process::{format_error, kill_process};
use std::time::{Duration, Instant};

const MESSAGE_TIMEOUT: Duration = Duration::from_secs(2);

#[derive(Debug, Clone)]
pub enum Mode {
    Normal,
    Search,
    ConfirmKill {
        pid: u32,
        name: String,
        container_id: Option<String>,
    },
}

pub struct App {
    pub ports: Vec<PortInfo>,
    pub filtered: Vec<usize>,
    pub selected: usize,
    pub search_query: String,
    pub mode: Mode,
    pub message: Option<String>,
    pub message_expires_at: Option<Instant>,
    pub should_quit: bool,
}

impl App {
    pub fn new() -> Self {
        let mut app = Self {
            ports: Vec::new(),
            filtered: Vec::new(),
            selected: 0,
            search_query: String::new(),
            mode: Mode::Normal,
            message: None,
            message_expires_at: None,
            should_quit: false,
        };
        let _ = app.refresh_ports();
        app
    }

    pub fn refresh_ports(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        match get_open_ports() {
            Ok(all_ports) => {
                self.ports = all_ports;
                self.apply_filter();
                Ok(())
            }
            Err(e) => Err(Box::new(e)),
        }
    }

    fn apply_filter(&mut self) {
        self.filtered = if self.search_query.is_empty() {
            (0..self.ports.len()).collect()
        } else {
            let query = self.search_query.to_lowercase();
            self.ports
                .iter()
                .enumerate()
                .filter(|(_, p)| {
                    p.port.to_string().contains(&query)
                        || p.process_name.to_lowercase().contains(&query)
                        || p.process_path.to_lowercase().contains(&query)
                })
                .map(|(i, _)| i)
                .collect()
        };
        self.selected = self.selected.min(self.filtered.len().saturating_sub(1));
    }

    pub fn update_search(&mut self, query: String) {
        self.search_query = query;
        self.apply_filter();
    }

    pub fn next(&mut self) {
        if !self.filtered.is_empty() && self.selected < self.filtered.len() - 1 {
            self.selected += 1;
        }
    }

    pub fn previous(&mut self) {
        if self.selected > 0 {
            self.selected -= 1;
        }
    }

    pub fn enter_search_mode(&mut self) {
        self.mode = Mode::Search;
        self.search_query.clear();
        self.apply_filter();
    }

    pub fn exit_search_mode(&mut self) {
        self.mode = Mode::Normal;
    }

    pub fn confirm_kill(&mut self) {
        if let Some(&idx) = self.filtered.get(self.selected) {
            if let Some(port) = self.ports.get(idx) {
                self.mode = Mode::ConfirmKill {
                    pid: port.pid,
                    name: port.process_name.clone(),
                    container_id: port.container_id.clone(),
                };
            }
        }
    }

    pub fn execute_kill(&mut self) {
        let (pid, container_id) = match &self.mode {
            Mode::ConfirmKill {
                pid,
                name: _,
                container_id,
            } => (*pid, container_id.clone()),
            _ => return,
        };

        let result = if let Some(ref id) = container_id {
            kill_docker_container(id)
        } else {
            kill_process(pid)
        };

        match result {
            Ok(_) => {
                self.show_message(if container_id.is_some() {
                    "Stopped container".to_string()
                } else {
                    format!("Killed process {}", pid)
                });
                let _ = self.refresh_ports();
            }
            Err(e) => {
                self.show_message(format_error(&e));
            }
        }

        self.mode = Mode::Normal;
    }

    pub fn cancel_modal(&mut self) {
        self.mode = Mode::Normal;
    }

    pub fn tick(&mut self) {
        if matches!(self.message_expires_at, Some(deadline) if Instant::now() >= deadline) {
            self.clear_message();
        }
    }

    pub fn clear_message(&mut self) {
        self.message = None;
        self.message_expires_at = None;
    }

    fn show_message(&mut self, message: String) {
        self.message = Some(message);
        self.message_expires_at = Some(Instant::now() + MESSAGE_TIMEOUT);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_port(port: u16, pid: u32, name: &str) -> PortInfo {
        PortInfo {
            port,
            pid,
            process_name: name.to_string(),
            process_path: "/bin/test".to_string(),
            container_id: None,
        }
    }

    #[test]
    fn test_navigation() {
        let mut app = App::new();
        app.ports = vec![
            create_test_port(3000, 100, "test1"),
            create_test_port(3001, 101, "test2"),
            create_test_port(3002, 102, "test3"),
        ];
        app.filtered = vec![0, 1, 2];
        app.selected = 0;

        app.next();
        assert_eq!(app.selected, 1);

        app.next();
        assert_eq!(app.selected, 2);

        app.next();
        assert_eq!(app.selected, 2);

        app.previous();
        assert_eq!(app.selected, 1);

        app.previous();
        assert_eq!(app.selected, 0);

        app.previous();
        assert_eq!(app.selected, 0);
    }

    #[test]
    fn test_search_filter() {
        let mut app = App::new();
        app.ports = vec![
            create_test_port(8080, 100, "firefox"),
            create_test_port(9999, 101, "chrome"),
            create_test_port(7777, 102, "firefox"),
        ];
        app.filtered = vec![0, 1, 2];

        app.update_search("fire".to_string());
        assert_eq!(app.filtered.len(), 2);

        app.update_search("9999".to_string());
        assert_eq!(app.filtered.len(), 1);

        app.update_search("".to_string());
        assert_eq!(app.filtered.len(), 3);
    }

    #[test]
    fn test_mode_transitions() {
        let mut app = App::new();

        assert!(matches!(app.mode, Mode::Normal));

        app.enter_search_mode();
        assert!(matches!(app.mode, Mode::Search));

        app.exit_search_mode();
        assert!(matches!(app.mode, Mode::Normal));

        app.ports = vec![create_test_port(8080, 100, "test")];
        app.filtered = vec![0];
        app.confirm_kill();
        assert!(matches!(app.mode, Mode::ConfirmKill { pid: 100, .. }));

        app.cancel_modal();
        assert!(matches!(app.mode, Mode::Normal));
    }

    #[test]
    fn test_message_timeout_clears_popup() {
        let mut app = App::new();
        app.message = Some("Killed process 100".to_string());
        app.message_expires_at = Some(Instant::now() - Duration::from_millis(1));

        app.tick();

        assert!(app.message.is_none());
        assert!(app.message_expires_at.is_none());
    }
}
