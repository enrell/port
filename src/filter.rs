const BLOCKED_PORTS: &[u16] = &[
    22, 80, 443, 53, 3306, 5432, 6379, 8080, 3000, 5000, 8000, 9000,
];

const BLOCKED_PROCESSES: &[&str] = &[
    "systemd",
    "dockerd",
    "containerd",
    "sshd",
    "tor",
    "docker",
    "kubelet",
    "kube-proxy",
];

pub fn is_blocked_port(port: u16) -> bool {
    BLOCKED_PORTS.contains(&port)
}

pub fn is_blocked_process(name: &str) -> bool {
    let name_lower = name.to_lowercase();
    BLOCKED_PROCESSES
        .iter()
        .any(|blocked| name_lower.contains(blocked))
}

pub fn should_include_port(port: u16, process_name: &str) -> bool {
    !is_blocked_port(port) && !is_blocked_process(process_name)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_blocked_ports() {
        assert!(is_blocked_port(22));
        assert!(is_blocked_port(80));
        assert!(is_blocked_port(443));
        assert!(!is_blocked_port(9999));
        assert!(!is_blocked_port(12345));
    }

    #[test]
    fn test_blocked_processes() {
        assert!(is_blocked_process("systemd"));
        assert!(is_blocked_process("sshd"));
        assert!(is_blocked_process("dockerd"));
        assert!(is_blocked_process("/usr/bin/dockerd"));
        assert!(!is_blocked_process("firefox"));
        assert!(!is_blocked_process("myapp"));
    }

    #[test]
    fn test_should_include_port() {
        assert!(!should_include_port(22, "firefox"));
        assert!(!should_include_port(9999, "sshd"));
        assert!(!should_include_port(80, "systemd"));
        assert!(should_include_port(9999, "firefox"));
        assert!(should_include_port(12345, "myapp"));
    }
}
