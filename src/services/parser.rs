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
        
        if let Some(display) = vars.get("display") {
            config.display = Self::parse_display_protocol(display.trim_matches('"'));
        }
        
        if let Some(ssh_port) = vars.get("ssh_port") {
            if let Ok(port) = ssh_port.parse::<u16>() {
                config.ssh_port = Some(port);
            }
        }
        
        Ok(config)
    }
    
    fn extract_variables(content: &str) -> HashMap<String, String> {
        let mut vars = HashMap::new();
        
        for line in content.lines() {
            let line = line.trim();
            
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            
            if let Some((key, value)) = line.split_once('=') {
                let key = key.trim();
                let value = value.trim();
                vars.insert(key.to_string(), value.to_string());
            }
        }
        
        vars
    }
    
    fn parse_display_protocol(display: &str) -> DisplayProtocol {
        match display.to_lowercase().as_str() {
            "spice" => DisplayProtocol::Spice { port: 5930 },
            "vnc" => DisplayProtocol::Vnc { port: 5900 },
            "sdl" => DisplayProtocol::Sdl,
            _ => DisplayProtocol::None,
        }
    }
    
    pub fn save_config(path: &Path, config: &VMConfig) -> Result<()> {
        let mut content = String::new();
        
        content.push_str(&format!("guest_os=\"{}\"\n", config.guest_os));
        
        if let Some(disk_img) = &config.disk_img {
            content.push_str(&format!("disk_img=\"{}\"\n", disk_img.display()));
        }
        
        if let Some(iso) = &config.iso {
            content.push_str(&format!("iso=\"{}\"\n", iso.display()));
        }
        
        content.push_str(&format!("ram=\"{}\"\n", config.ram));
        content.push_str(&format!("cpu_cores={}\n", config.cpu_cores));
        
        if let Some(disk_size) = &config.disk_size {
            content.push_str(&format!("disk_size=\"{}\"\n", disk_size));
        }
        
        match &config.display {
            DisplayProtocol::Spice { .. } => content.push_str("display=\"spice\"\n"),
            DisplayProtocol::Vnc { .. } => content.push_str("display=\"vnc\"\n"),
            DisplayProtocol::Sdl => content.push_str("display=\"sdl\"\n"),
            DisplayProtocol::None => content.push_str("display=\"none\"\n"),
        }
        
        if let Some(ssh_port) = config.ssh_port {
            content.push_str(&format!("ssh_port={}\n", ssh_port));
        }
        
        std::fs::write(path, content)?;
        Ok(())
    }
}