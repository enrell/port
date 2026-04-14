use std::io;

pub fn kill_process(pid: u32) -> io::Result<()> {
    let result = unsafe { libc::kill(pid as libc::pid_t, libc::SIGKILL) };
    if result == 0 {
        Ok(())
    } else {
        Err(io::Error::last_os_error())
    }
}

pub fn format_error(e: &io::Error) -> String {
    if e.raw_os_error() == Some(libc::ESRCH) {
        "Process not found (may have already exited)".to_string()
    } else if e.raw_os_error() == Some(libc::EPERM) {
        "Permission denied".to_string()
    } else {
        e.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_kill_nonexistent_process() {
        let result = kill_process(999999);
        assert!(result.is_err());

        if let Err(e) = result {
            let formatted = format_error(&e);
            assert!(formatted.contains("not found") || formatted.contains("No such process"));
        }
    }
}
