use std::collections::HashMap;
use std::fs;
use std::io::{self, BufRead, BufReader};

#[derive(Debug, Clone, PartialEq)]
pub struct PortInfo {
    pub port: u16,
    pub pid: u32,
    pub process_name: String,
    pub process_path: String,
}

pub fn get_open_ports() -> io::Result<Vec<PortInfo>> {
    let mut ports = Vec::new();
    let mut inode_pid_map = build_inode_pid_map()?;

    parse_tcp_file("/proc/net/tcp", &mut ports, &mut inode_pid_map, false)?;
    let _ = parse_tcp_file("/proc/net/tcp6", &mut ports, &mut inode_pid_map, true);

    ports.sort_by(|a, b| a.port.cmp(&b.port));
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
