use crate::models::{DisplayProtocol, VMConfig};
use anyhow::Result;
use std::path::Path;
use std::collections::HashMap;

pub struct ConfigParser;

impl ConfigParser {
    pub fn parse_quickemu_config(path: &Path) -> Result<VMConfig> {
        let content = std::fs::read_to_string(path)?;
        let mut config = VMConfig {
            guest_os: String::new(),
            disk_img: None,
            iso: None,
            ram: "2G".to_string(),
            cpu_cores: 2,
            disk_size: None,
            display: DisplayProtocol::Spice { port: 5930 },
            ssh_port: None,
            raw_config: content.clone(),
        };
        
        let vars = Self::extract_variables(&content);
        
        if let Some(guest_os) = vars.get("guest_os") {
            config.guest_os = guest_os.trim_matches('"').to_string();
        }
        
        if let Some(disk_img) = vars.get("disk_img") {
            config.disk_img = Some(disk_img.trim_matches('"').into());
        }
        
        if let Some(iso) = vars.get("iso") {
            config.iso = Some(iso.trim_matches('"').into());
        }
        
        if let Some(ram) = vars.get("ram") {
            config.ram = ram.trim_matches('"').to_string();
        }
        
        if let Some(cpu_cores) = vars.get("cpu_cores") {
            if let Ok(cores) = cpu_cores.parse::<u32>() {
                config.cpu_cores = cores;
            }
        }
        
        if let Some(disk_size) = vars.get("disk_size") {
            config.disk_size = Some(disk_size.trim_matches('"').to_string());
        }
        
        // Parse display settings
        if let Some(display_server) = vars.get("display_server") {
            config.display = match display_server.trim_matches('"') {
                "spice" => DisplayProtocol::Spice { port: 5930 },
                "vnc" => DisplayProtocol::Vnc { port: 5900 },
                _ => config.display,
            };
        }
        
        if let Some(ssh_port) = vars.get("ssh_port") {
            if let Ok(port) = ssh_port.parse::<u16>() {
                config.ssh_port = Some(port);
            }
        }
        
        Ok(config)
    }
    
    pub fn save_config(path: &Path, config: &VMConfig) -> Result<()> {
        let mut lines = Vec::new();
        
        // Basic config
        lines.push(format!("guest_os=\"{}\"", config.guest_os));
        lines.push(format!("ram=\"{}\"", config.ram));
        lines.push(format!("cpu_cores={}", config.cpu_cores));
        
        if let Some(disk_img) = &config.disk_img {
            lines.push(format!("disk_img=\"{}\"", disk_img.display()));
        }
        
        if let Some(iso) = &config.iso {
            lines.push(format!("iso=\"{}\"", iso.display()));
        }
        
        if let Some(disk_size) = &config.disk_size {
            lines.push(format!("disk_size=\"{}\"", disk_size));
        }
        
        // Display settings
        match &config.display {
            DisplayProtocol::Spice { port: _ } => {
                lines.push("display_server=\"spice\"".to_string());
            }
            DisplayProtocol::Vnc { port: _ } => {
                lines.push("display_server=\"vnc\"".to_string());
            }
            _ => {}
        }
        
        if let Some(ssh_port) = config.ssh_port {
            lines.push(format!("ssh_port={}", ssh_port));
        }
        
        let content = lines.join("\n") + "\n";
        std::fs::write(path, content)?;
        
        Ok(())
    }
    
    fn extract_variables(content: &str) -> HashMap<String, String> {
        let mut vars = HashMap::new();
        
        for line in content.lines() {
            let line = line.trim();
            if line.starts_with('#') || line.is_empty() {
                continue;
            }
            
            if let Some(eq_pos) = line.find('=') {
                let key = line[..eq_pos].trim().to_string();
                let value = line[eq_pos + 1..].trim().to_string();
                vars.insert(key, value);
            }
        }
        
        vars
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::NamedTempFile;

    #[test]
    fn test_extract_variables() {
        let content = r#"
# Comment line
guest_os="ubuntu"
cpu_cores=4
ram="4G"

disk_img="/path/to/disk.qcow2"
        "#;
        
        let vars = ConfigParser::extract_variables(content);
        
        assert_eq!(vars.get("guest_os"), Some(&"\"ubuntu\"".to_string()));
        assert_eq!(vars.get("cpu_cores"), Some(&"4".to_string()));
        assert_eq!(vars.get("ram"), Some(&"\"4G\"".to_string()));
        assert_eq!(vars.get("disk_img"), Some(&"\"/path/to/disk.qcow2\"".to_string()));
        assert!(!vars.contains_key("# Comment line"));
    }

    #[test]
    fn test_parse_basic_config() -> Result<()> {
        let content = r#"
guest_os="ubuntu"
cpu_cores=4
ram="4G"
disk_img="/path/to/disk.qcow2"
display_server="spice"
ssh_port=22220
        "#;
        
        let temp_file = NamedTempFile::new()?;
        fs::write(&temp_file, content)?;
        
        let config = ConfigParser::parse_quickemu_config(temp_file.path())?;
        
        assert_eq!(config.guest_os, "ubuntu");
        assert_eq!(config.cpu_cores, 4);
        assert_eq!(config.ram, "4G");
        assert_eq!(config.disk_img, Some("/path/to/disk.qcow2".into()));
        assert_eq!(config.ssh_port, Some(22220));
        
        match config.display {
            DisplayProtocol::Spice { port } => assert_eq!(port, 5930),
            _ => panic!("Expected Spice display protocol"),
        }
        
        Ok(())
    }

    #[test]
    fn test_parse_minimal_config() -> Result<()> {
        let content = r#"
guest_os="fedora"
        "#;
        
        let temp_file = NamedTempFile::new()?;
        fs::write(&temp_file, content)?;
        
        let config = ConfigParser::parse_quickemu_config(temp_file.path())?;
        
        assert_eq!(config.guest_os, "fedora");
        assert_eq!(config.cpu_cores, 2); // Default value
        assert_eq!(config.ram, "2G"); // Default value
        assert_eq!(config.disk_img, None);
        
        Ok(())
    }

    #[test]
    fn test_parse_vnc_display() -> Result<()> {
        let content = r#"
guest_os="debian"
display_server="vnc"
        "#;
        
        let temp_file = NamedTempFile::new()?;
        fs::write(&temp_file, content)?;
        
        let config = ConfigParser::parse_quickemu_config(temp_file.path())?;
        
        match config.display {
            DisplayProtocol::Vnc { port } => assert_eq!(port, 5900),
            _ => panic!("Expected VNC display protocol"),
        }
        
        Ok(())
    }

    #[test]
    fn test_parse_invalid_cpu_cores() -> Result<()> {
        let content = r#"
guest_os="arch"
cpu_cores=invalid
        "#;
        
        let temp_file = NamedTempFile::new()?;
        fs::write(&temp_file, content)?;
        
        let config = ConfigParser::parse_quickemu_config(temp_file.path())?;
        
        // Should fall back to default when parsing fails
        assert_eq!(config.cpu_cores, 2);
        
        Ok(())
    }

    #[test]
    fn test_parse_nonexistent_file() {
        let result = ConfigParser::parse_quickemu_config(Path::new("/nonexistent/file.conf"));
        assert!(result.is_err());
    }
}