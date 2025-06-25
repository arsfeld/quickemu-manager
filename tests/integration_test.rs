use quickemu_manager::models::{VM, VMId, VMStatus, VMConfig, DisplayProtocol, VMTemplate};
use quickemu_manager::services::{VMManager, ConfigParser};
use std::path::{Path, PathBuf};
use tempfile::TempDir;
use std::fs;

#[tokio::test]
async fn test_end_to_end_vm_workflow() {
    let temp_dir = TempDir::new().unwrap();
    
    // Create a test VM configuration file
    let config_content = r#"
guest_os="ubuntu"
cpu_cores=4
ram="4G"
disk_img="/path/to/ubuntu.qcow2"
display_server="spice"
ssh_port=22220
    "#;
    
    let config_path = temp_dir.path().join("ubuntu-22.04.conf");
    fs::write(&config_path, config_content).unwrap();
    
    // Test config parsing
    let config = ConfigParser::parse_quickemu_config(&config_path).unwrap();
    assert_eq!(config.guest_os, "ubuntu");
    assert_eq!(config.cpu_cores, 4);
    assert_eq!(config.ram, "4G");
    
    // Create VM from parsed config
    let vm = VM {
        id: VMId("ubuntu-22.04".to_string()),
        name: "Ubuntu 22.04".to_string(),
        os: "Ubuntu".to_string(),
        version: "22.04".to_string(),
        config_path: config_path.clone(),
        status: VMStatus::Stopped,
        config: config.clone(),
        cpu_cores: config.cpu_cores,
        ram_mb: 4096,
    };
    
    // Test VM manager operations
    let vm_manager = VMManager::with_paths(
        PathBuf::from("/usr/bin/echo"), // Mock command
        Some(PathBuf::from("/usr/bin/echo"))
    );
    
    // Test status checking
    let status = vm_manager.get_vm_status(&vm.id).await;
    assert_eq!(status, VMStatus::Stopped);
    
    // Test VM running check
    let is_running = vm_manager.is_vm_running(&vm.id).await;
    assert!(!is_running);
}

#[tokio::test]
async fn test_vm_template_integration() {
    let template = VMTemplate {
        os: "ubuntu".to_string(),
        version: "22.04".to_string(),
        name: Some("Ubuntu 22.04 Desktop".to_string()),
    };
    
    assert_eq!(template.os, "ubuntu");
    assert_eq!(template.version, "22.04");
    
    // Test VM manager with quickget (using mock)
    let vm_manager = VMManager::with_paths(
        PathBuf::from("/usr/bin/echo"),
        Some(PathBuf::from("/usr/bin/echo"))
    );
    
    assert!(vm_manager.get_quickget_path().is_some());
}

#[test]
fn test_config_parsing_edge_cases() {
    let temp_dir = TempDir::new().unwrap();
    
    // Test empty config
    let empty_config = temp_dir.path().join("empty.conf");
    fs::write(&empty_config, "").unwrap();
    
    let config = ConfigParser::parse_quickemu_config(&empty_config).unwrap();
    assert_eq!(config.guest_os, ""); // Should be empty
    assert_eq!(config.cpu_cores, 2); // Should use default
    
    // Test config with comments and whitespace
    let complex_config = temp_dir.path().join("complex.conf");
    let complex_content = r#"
# This is a comment
guest_os="fedora"

# Another comment
cpu_cores=8
    ram="8G"    
# More comments
display_server="vnc"
    "#;
    fs::write(&complex_config, complex_content).unwrap();
    
    let config = ConfigParser::parse_quickemu_config(&complex_config).unwrap();
    assert_eq!(config.guest_os, "fedora");
    assert_eq!(config.cpu_cores, 8);
    assert_eq!(config.ram, "8G");
    
    match config.display {
        DisplayProtocol::VNC { port } => assert_eq!(port, 5900),
        _ => panic!("Expected VNC display protocol"),
    }
}

#[tokio::test]
async fn test_vm_lifecycle_simulation() {
    let temp_dir = TempDir::new().unwrap();
    let vm_manager = VMManager::with_paths(
        PathBuf::from("/usr/bin/echo"),
        Some(PathBuf::from("/usr/bin/echo"))
    );
    
    // Create a test VM
    let config_path = temp_dir.path().join("test-vm.conf");
    fs::write(&config_path, "guest_os=\"test\"").unwrap();
    
    let vm = VM {
        id: VMId("test-vm".to_string()),
        name: "Test VM".to_string(),
        os: "Test".to_string(),
        version: "1.0".to_string(),
        config_path,
        status: VMStatus::Stopped,
        config: VMConfig {
            guest_os: "test".to_string(),
            disk_img: None,
            iso: None,
            ram: "2G".to_string(),
            cpu_cores: 2,
            disk_size: None,
            display: DisplayProtocol::Spice { port: 5930 },
            ssh_port: None,
            raw_config: "guest_os=\"test\"".to_string(),
        },
        cpu_cores: 2,
        ram_mb: 2048,
    };
    
    // Test initial state
    assert_eq!(vm.status, VMStatus::Stopped);
    assert!(!vm.is_running());
    
    // Test status checking through manager
    let status = vm_manager.get_vm_status(&vm.id).await;
    assert_eq!(status, VMStatus::Stopped);
    
    // Test attempting to stop a non-running VM
    let result = vm_manager.stop_vm(&vm.id).await;
    assert!(result.is_err());
}

#[test]
fn test_vm_id_functionality() {
    let vm_id1 = VMId("ubuntu-22.04".to_string());
    let vm_id2 = VMId("ubuntu-22.04".to_string());
    let vm_id3 = VMId("fedora-38".to_string());
    
    assert_eq!(vm_id1, vm_id2);
    assert_ne!(vm_id1, vm_id3);
    
    assert_eq!(vm_id1.0, "ubuntu-22.04");
}

#[test]
fn test_display_protocol_variants() {
    let spice = DisplayProtocol::Spice { port: 5930 };
    let vnc = DisplayProtocol::VNC { port: 5900 };
    
    match spice {
        DisplayProtocol::Spice { port } => assert_eq!(port, 5930),
        _ => panic!("Expected Spice protocol"),
    }
    
    match vnc {
        DisplayProtocol::VNC { port } => assert_eq!(port, 5900),
        _ => panic!("Expected VNC protocol"),
    }
}

#[test]
fn test_vm_status_variants() {
    let statuses = vec![
        VMStatus::Stopped,
        VMStatus::Starting,
        VMStatus::Running,
        VMStatus::Stopping,
    ];
    
    for status in statuses {
        // Test that all status variants can be created
        match status {
            VMStatus::Stopped => {},
            VMStatus::Starting => {},
            VMStatus::Running => {},
            VMStatus::Stopping => {},
        }
    }
}

#[tokio::test]
async fn test_concurrent_vm_operations() {
    let vm_manager = VMManager::with_paths(
        PathBuf::from("/usr/bin/echo"),
        Some(PathBuf::from("/usr/bin/echo"))
    );
    
    let vm_ids = vec![
        VMId("vm1".to_string()),
        VMId("vm2".to_string()),
        VMId("vm3".to_string()),
    ];
    
    // Test concurrent status checks
    let mut handles = vec![];
    for vm_id in vm_ids {
        let manager = vm_manager.clone();
        let handle = tokio::spawn(async move {
            manager.get_vm_status(&vm_id).await
        });
        handles.push(handle);
    }
    
    // Wait for all status checks to complete
    for handle in handles {
        let status = handle.await.unwrap();
        assert_eq!(status, VMStatus::Stopped);
    }
}

// Helper function to create a VM manager that won't actually execute commands
fn create_safe_vm_manager() -> VMManager {
    VMManager::with_paths(
        PathBuf::from("/usr/bin/true"), // Always succeeds
        Some(PathBuf::from("/usr/bin/true"))
    )
}

#[tokio::test]
async fn test_vm_manager_safety() {
    let vm_manager = create_safe_vm_manager();
    
    // These operations should be safe and not actually execute dangerous commands
    assert_eq!(vm_manager.get_quickemu_path(), Path::new("/usr/bin/true"));
    assert_eq!(vm_manager.get_quickget_path(), Some(Path::new("/usr/bin/true")));
    
    let vm_id = VMId("safe-test".to_string());
    let status = vm_manager.get_vm_status(&vm_id).await;
    assert_eq!(status, VMStatus::Stopped);
    
    vm_manager.cleanup_finished_processes().await;
}