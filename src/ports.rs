use std::collections::HashMap;
use std::fs;
use std::io::{self, BufRead, BufReader};
use std::process::Command;

#[derive(Debug, Clone, PartialEq)]
pub struct PortInfo {
    pub port: u16,
    pub pid: u32,
    pub process_name: String,
    pub process_path: String,
    pub container_id: Option<String>,
}

pub fn get_open_ports() -> io::Result<Vec<PortInfo>> {
    let mut ports = Vec::new();
    let mut inode_pid_map = build_inode_pid_map()?;

    parse_tcp_file("/proc/net/tcp", &mut ports, &mut inode_pid_map, false)?;
    let _ = parse_tcp_file("/proc/net/tcp6", &mut ports, &mut inode_pid_map, true);

    let mut ss_ports = parse_ss_ports()?;
    ports.append(&mut ss_ports);

    let mut docker_ports = parse_docker_ports()?;
    ports.append(&mut docker_ports);

    ports.sort_by(|a, b| {
        match a.port.cmp(&b.port) {
            std::cmp::Ordering::Equal => b.pid.cmp(&a.pid),
            other => other,
        }
    });
    ports.dedup_by(|a, b| a.port == b.port);
    Ok(ports)
}

fn build_inode_pid_map() -> io::Result<HashMap<u64, u32>> {
    let mut map = HashMap::new();

    for entry in fs::read_dir("/proc")? {
        let entry = entry?;
        let name = entry.file_name().to_string_lossy().to_string();

        if let Ok(pid) = name.parse::<u32>() {
            let fd_dir = format!("/proc/{}/fd", pid);
            if let Ok(entries) = fs::read_dir(&fd_dir) {
                for fd_entry in entries.flatten() {
                    if let Ok(link) = fs::read_link(fd_entry.path()) {
                        let link_str = link.to_string_lossy();
                        if link_str.starts_with("socket:[") {
                            if let Some(inode) = link_str.strip_prefix("socket:[").and_then(|s| s.strip_suffix("]")) {
                                if let Ok(inode_num) = inode.parse::<u64>() {
                                    map.insert(inode_num, pid);
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    Ok(map)
}

fn parse_tcp_file(
    path: &str,
    ports: &mut Vec<PortInfo>,
    inode_pid_map: &mut HashMap<u64, u32>,
    _is_ipv6: bool,
) -> io::Result<()> {
    let file = fs::File::open(path)?;
    let reader = BufReader::new(file);

    for line in reader.lines().skip(1) {
        let line = line?;
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() < 10 {
            continue;
        }

        let _local_addr = parts[1];
        let _rem_addr = parts[2];
        let state = parts[3];
        let inode_str = parts[9];

        if state != "0A" {
            continue;
        }

        let port = parse_port(_local_addr);

        let inode: u64 = match inode_str.parse() {
            Ok(n) => n,
            Err(_) => continue,
        };

        if inode == 0 {
            continue;
        }

        if let Some(&pid) = inode_pid_map.get(&inode) {
            let (name, path) = get_process_info(pid);

            ports.push(PortInfo {
                port,
                pid,
                process_name: name,
                process_path: path,
                container_id: None,
            });
        }
    }

    Ok(())
}

fn parse_port(hex_addr: &str) -> u16 {
    if let Some(colon_pos) = hex_addr.rfind(':') {
        let port_hex = &hex_addr[colon_pos + 1..];
        if let Ok(port) = u16::from_str_radix(port_hex, 16) {
            return port;
        }
    }
    0
}

fn get_process_info(pid: u32) -> (String, String) {
    let exe_link = format!("/proc/{}/exe", pid);
    let cmdline_path = format!("/proc/{}/comm", pid);

    let path = fs::read_link(&exe_link)
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_default();

    let name = fs::read_to_string(&cmdline_path)
        .map(|s| s.trim().to_string())
        .unwrap_or_else(|_| format!("pid_{}", pid));

    (name, path)
}

fn parse_ss_ports() -> io::Result<Vec<PortInfo>> {
    let output = Command::new("ss").args(["-tlnpe"]).output()?;
    if !output.status.success() {
        return Ok(Vec::new());
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut ports = Vec::new();

    for line in stdout.lines().skip(1) {
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() < 6 {
            continue;
        }

        let local_addr = parts[3];
        let port_str = match local_addr.rsplit(':').next() {
            Some(s) => s,
            None => continue,
        };

        let port: u16 = match port_str.parse() {
            Ok(p) => p,
            Err(_) => continue,
        };

        let (pid, name) = parse_ss_process_info(&parts);

        if pid == 0 {
            continue;
        }

        ports.push(PortInfo {
            port,
            pid,
            process_name: name,
            process_path: String::new(),
            container_id: None,
        });
    }

    Ok(ports)
}

fn parse_docker_ports() -> io::Result<Vec<PortInfo>> {
    let output = Command::new("docker")
        .args(["ps", "--format", "{{.ID}}|{{.Names}}|{{.Ports}}"])
        .output()?;
    if !output.status.success() {
        return Ok(Vec::new());
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut ports = Vec::new();

    for line in stdout.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }

        let parts: Vec<&str> = line.splitn(3, '|').collect();
        if parts.len() != 3 {
            continue;
        }

        let container_id = parts[0].to_string();
        let container_name = parts[1].to_string();
        let port_mappings = parts[2];

        for mapping in port_mappings.split(',') {
            let mapping = mapping.trim();
            if let Some((host_port, _)) = parse_docker_port_mapping(mapping) {
                if host_port > 0 {
                    ports.push(PortInfo {
                        port: host_port,
                        pid: 0,
                        process_name: format!("docker: {}", container_name),
                        process_path: String::new(),
                        container_id: Some(container_id.clone()),
                    });
                }
            }
        }
    }

    Ok(ports)
}

fn parse_docker_port_mapping(mapping: &str) -> Option<(u16, String)> {
    let parts: Vec<&str> = mapping.split("->").collect();
    if parts.is_empty() {
        return None;
    }

    let host_part = parts[0];
    let container_port = extract_port_from_host_part(host_part)?;

    Some((container_port, mapping.to_string()))
}

fn extract_port_from_host_part(host_part: &str) -> Option<u16> {
    let port_str = host_part.rsplit(':').next()?;
    port_str.parse().ok()
}

fn parse_ss_process_info(parts: &[&str]) -> (u32, String) {
    for part in parts {
        if part.starts_with("pid=") {
            if let Ok(pid) = part[4..].split(',').next().unwrap_or("0").parse() {
                let name = parts.iter()
                    .find(|p| p.starts_with("users:("))
                    .and_then(|s| {
                        let s = s.strip_prefix("users:(\"")?;
                        s.split('"').next().map(|n| n.to_string())
                    })
                    .unwrap_or_else(|| format!("pid_{}", pid));
                return (pid, name);
            }
        }
    }
    (0, String::new())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_port() {
        assert_eq!(parse_port("0100007F:1F90"), 8080);
        assert_eq!(parse_port("0000000000000000FFFF00000100007F:270F"), 9999);
        assert_eq!(parse_port("00000000:0050"), 80);
        assert_eq!(parse_port("00000000:0016"), 22);
        assert_eq!(parse_port("invalid"), 0);
    }
}
