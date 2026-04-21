use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use std::io;

use crate::app::{App, Mode};

pub fn handle(app: &mut App) -> io::Result<bool> {
    if event::poll(std::time::Duration::from_millis(100))? {
        if let Event::Key(key) = event::read()? {
            if key.kind == KeyEventKind::Press {
                return Ok(handle_key_event(app, key));
            }
        }
    }
    Ok(false)
}

fn handle_key_event(app: &mut App, key: KeyEvent) -> bool {
    match &app.mode {
        Mode::Normal => handle_normal_mode(app, key),
        Mode::Search => handle_search_mode(app, key),
        Mode::ConfirmKill { .. } => handle_confirm_mode(app, key),
    }
}

fn handle_normal_mode(app: &mut App, key: KeyEvent) -> bool {
    match key.code {
        KeyCode::Char('q') | KeyCode::Char('Q') => {
            app.should_quit = true;
            true
        }
        KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            app.should_quit = true;
            true
        }
        KeyCode::Char('/') | KeyCode::Char('i') => {
            app.enter_search_mode();
            false
        }
        KeyCode::Down | KeyCode::Char('j') => {
            app.next();
            false
        }
        KeyCode::Up | KeyCode::Char('k') => {
            app.previous();
            false
        }
        KeyCode::Enter => {
            app.confirm_kill();
            false
        }
        KeyCode::Esc => {
            false
        }
        KeyCode::Char('r') => {
            let _ = app.refresh_ports();
            false
        }
        _ => false,
    }
}

fn handle_search_mode(app: &mut App, key: KeyEvent) -> bool {
    match key.code {
        KeyCode::Esc => {
            app.exit_search_mode();
            false
        }
        KeyCode::Enter => {
            app.exit_search_mode();
            false
        }
        KeyCode::Backspace => {
            let mut query = app.search_query.clone();
            query.pop();
            app.update_search(query);
            false
        }
        KeyCode::Char(c) => {
            let mut query = app.search_query.clone();
            query.push(c);
            app.update_search(query);
            false
        }
        _ => false,
    }
}

fn handle_confirm_mode(app: &mut App, key: KeyEvent) -> bool {
    match key.code {
        KeyCode::Char('y') | KeyCode::Char('Y') => {
            app.execute_kill();
            false
        }
        KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc => {
            app.cancel_modal();
            false
        }
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use crate::app::App;
    use crate::ports::PortInfo;
    use crate::events::{handle_normal_mode, handle_search_mode, handle_confirm_mode};
    use crossterm::event::KeyCode;

    fn make_key(code: KeyCode) -> crossterm::event::KeyEvent {
        crossterm::event::KeyEvent::from(code)
    }

    fn make_char_key(c: char) -> crossterm::event::KeyEvent {
        crossterm::event::KeyEvent::from(KeyCode::Char(c))
    }

 #[test]
 fn test_quit_with_q() {
 let mut app = App::new();
 let should_quit = handle_normal_mode(&mut app, make_char_key('q'));
 assert!(should_quit);
 assert!(app.should_quit);
 }

 #[test]
 fn test_navigation() {
 let mut app = App::new();
 app.filtered = vec![0, 1, 2];
 app.selected = 0;

        handle_normal_mode(&mut app, make_char_key('j'));
        assert_eq!(app.selected, 1);

        handle_normal_mode(&mut app, make_key(KeyCode::Down));
        assert_eq!(app.selected, 2);

        handle_normal_mode(&mut app, make_char_key('k'));
        assert_eq!(app.selected, 1);

        handle_normal_mode(&mut app, make_key(KeyCode::Up));
        assert_eq!(app.selected, 0);
    }

 #[test]
 fn test_search_mode_activation() {
 let mut app = App::new();
 handle_normal_mode(&mut app, make_char_key('/'));
 assert!(matches!(app.mode, crate::app::Mode::Search));
 }

 #[test]
 fn test_search_mode_exit() {
 let mut app = App::new();
 app.mode = crate::app::Mode::Search;
 handle_search_mode(&mut app, make_key(KeyCode::Esc));
 assert!(matches!(app.mode, crate::app::Mode::Normal));
 }

 #[test]
 fn test_search_typing() {
 let mut app = App::new();
 app.mode = crate::app::Mode::Search;

 handle_search_mode(&mut app, make_char_key('t'));
 handle_search_mode(&mut app, make_char_key('e'));
 handle_search_mode(&mut app, make_char_key('s'));
 handle_search_mode(&mut app, make_char_key('t'));

 assert_eq!(app.search_query, "test");
 }

 #[test]
 fn test_confirm_mode_yes() {
 let mut app = App::new();
 app.ports = vec![PortInfo {
 port: 8080,
 pid: 99999,
 process_name: "nonexistent".to_string(),
 process_path: "/bin/test".to_string(),
 }];
 app.filtered = vec![0];
 app.confirm_kill();

 assert!(matches!(app.mode, crate::app::Mode::ConfirmKill { .. }));

 handle_confirm_mode(&mut app, make_char_key('y'));
 assert!(matches!(app.mode, crate::app::Mode::Normal));
 }

 #[test]
 fn test_confirm_mode_no() {
 let mut app = App::new();
 app.ports = vec![PortInfo {
            port: 8080,
            pid: 99999,
            process_name: "nonexistent".to_string(),
            process_path: "/bin/test".to_string(),
        }];
        app.filtered = vec![0];
        app.confirm_kill();

        handle_confirm_mode(&mut app, make_char_key('n'));
        assert!(matches!(app.mode, crate::app::Mode::Normal));
    }
}
