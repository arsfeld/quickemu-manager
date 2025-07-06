use crate::models::{DisplayProtocol, VMId, VMStatus, VMTemplate, VM};
use crate::services::binary_discovery::BinaryDiscovery;
use crate::services::process_monitor::ProcessMonitor;
use crate::services::vnc_proxy::{ConsoleInfo, ConsoleProtocol, VncProxy};
use anyhow::{anyhow, Result};
use std::collections::HashMap;
use std::io::{BufRead, BufReader};
use std::net::TcpStream;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::mpsc;
use std::sync::Arc;
use std::thread;
use std::time::Duration;
use sysinfo::{ProcessesToUpdate, System};
use tokio::sync::RwLock;

#[derive(Clone)]
pub struct VMManager {
    quickemu_path: PathBuf,
    quickget_path: Option<PathBuf>,
    processes: Arc<RwLock<HashMap<VMId, Child>>>,
    process_monitor: Option<Arc<ProcessMonitor>>,
    vnc_proxy: Option<Arc<VncProxy>>,
}

impl VMManager {
    pub async fn new() -> Result<Self> {
        let binary_discovery = BinaryDiscovery::new().await;
        binary_discovery.validate()?;

        let quickemu_path = binary_discovery.quickemu_path()?.to_path_buf();
        let quickget_path = binary_discovery.quickget_path().map(|p| p.to_path_buf());

        Ok(Self {
            quickemu_path,
            quickget_path,
            processes: Arc::new(RwLock::new(HashMap::new())),
            process_monitor: None,
            vnc_proxy: None,
        })
    }

    pub fn with_paths(quickemu_path: PathBuf, quickget_path: Option<PathBuf>) -> Self {
        Self {
            quickemu_path,
            quickget_path,
            processes: Arc::new(RwLock::new(HashMap::new())),
            process_monitor: None,
            vnc_proxy: None,
        }
    }

    pub async fn from_binary_discovery(binary_discovery: BinaryDiscovery) -> Result<Self> {
        binary_discovery.validate()?;

        let quickemu_path = binary_discovery.quickemu_path()?.to_path_buf();
        let quickget_path = binary_discovery.quickget_path().map(|p| p.to_path_buf());

        Ok(Self {
            quickemu_path,
            quickget_path,
            processes: Arc::new(RwLock::new(HashMap::new())),
            process_monitor: None,
            vnc_proxy: None,
        })
    }

    pub fn set_process_monitor(&mut self, process_monitor: Arc<ProcessMonitor>) {
        self.process_monitor = Some(process_monitor);
    }

    pub async fn start_vm(&self, vm: &VM) -> Result<()> {
        if vm.is_running() {
            return Err(anyhow!("VM is already running"));
        }

        let config_dir = vm
            .config_path
            .parent()
            .ok_or_else(|| anyhow!("Invalid config path"))?;

        let mut cmd = Command::new(&self.quickemu_path);
        cmd.arg("--vm").arg(&vm.config_path);

        // Configure display and access based on the VM's display protocol
        match &vm.config.display {
            DisplayProtocol::Spice { .. } => {
                cmd.arg("--display").arg("spice");
                cmd.arg("--access").arg("remote");
            }
            DisplayProtocol::Vnc { port } => {
                // Enable VNC using extra QEMU arguments
                // Use none display to avoid conflicts, VNC will be the display
                cmd.arg("--display").arg("none");
                // Add VNC server on the specified port (or auto-assign if port is 0)
                let vnc_arg = if *port > 0 {
                    format!("-vnc :{}", port - 5900) // VNC uses display number, not port
                } else {
                    "-vnc :0".to_string() // Default to display :0 (port 5900)
                };
                println!("Enabling VNC with args: {vnc_arg}");
                cmd.arg("--extra_args").arg(vnc_arg);
            }
            DisplayProtocol::Sdl => {
                cmd.arg("--display").arg("sdl");
            }
            DisplayProtocol::None => {
                cmd.arg("--display").arg("none");
            }
        }

        cmd.current_dir(config_dir);

        // Log the full command for debugging
        println!("Starting VM {} with command: {:?}", vm.id.0, cmd);

        let child = cmd.spawn()?;
        let wrapper_pid = child.id();

        println!(
            "Starting VM {}: quickemu wrapper launched with PID {}",
            vm.id.0, wrapper_pid
        );

        // Wait a bit for the actual QEMU process to start
        tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;

        // Find the actual QEMU process PID
        let actual_status = self.get_vm_status(&vm.id).await;

        // Register with process monitor if available
        if let Some(monitor) = &self.process_monitor {
            if let VMStatus::Running { pid } = actual_status {
                println!(
                    "Registering VM {} with ProcessMonitor: QEMU PID {}",
                    vm.id.0, pid
                );
                monitor.register_vm_process(vm.id.clone(), pid).await;
            } else {
                println!(
                    "Warning: Could not find running QEMU process for VM {}",
                    vm.id.0
                );
            }
        }

        Ok(())
    }

    pub async fn stop_vm(&self, vm_id: &VMId) -> Result<()> {
        // First try using sysinfo crate
        let mut system = sysinfo::System::new();
        system.refresh_processes(sysinfo::ProcessesToUpdate::All, false);

        for process in system.processes().values() {
            if let Some(cmd) = process.cmd().first() {
                let cmd_str = cmd.to_string_lossy();
                // Look for qemu processes with our VM config
                if cmd_str.contains("qemu-system")
                    && process.cmd().iter().any(|arg| {
                        let arg_str = arg.to_string_lossy();
                        arg_str.contains(&vm_id.0) || arg_str.contains(&format!("{}.conf", vm_id.0))
                    })
                {
                    println!(
                        "Stopping VM {}: Killing qemu process PID {}",
                        vm_id.0,
                        process.pid().as_u32()
                    );

                    // Unregister from process monitor BEFORE killing if available
                    if let Some(monitor) = &self.process_monitor {
                        monitor.unregister_vm_process(vm_id).await;
                    }

                    process.kill();

                    return Ok(());
                }
            }
        }

        // Fallback: Use ps and kill directly
        if let Ok(output) = std::process::Command::new("ps").args(["aux"]).output() {
            let ps_output = String::from_utf8_lossy(&output.stdout);
            for line in ps_output.lines() {
                if line.contains("qemu-system") && line.contains(&vm_id.0) {
                    // Extract PID from ps output (second column)
                    if let Some(pid_str) = line.split_whitespace().nth(1) {
                        if let Ok(pid) = pid_str.parse::<u32>() {
                            println!("Stopping VM {}: Killing qemu process PID {}", vm_id.0, pid);

                            // Use kill command
                            if std::process::Command::new("kill")
                                .arg(pid.to_string())
                                .output()
                                .is_ok()
                            {
                                // Unregister from process monitor if available
                                if let Some(monitor) = &self.process_monitor {
                                    monitor.unregister_vm_process(vm_id).await;
                                }
                                return Ok(());
                            }
                        }
                    }
                }
            }
        }

        Err(anyhow!("VM process not found"))
    }

    pub async fn restart_vm(&self, vm: &VM) -> Result<()> {
        self.stop_vm(&vm.id).await?;
        tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
        self.start_vm(vm).await
    }

    pub async fn get_vm_status(&self, vm_id: &VMId) -> VMStatus {
        // Always use external process detection since quickemu wrapper exits quickly
        self.check_vm_running_externally(vm_id).await
    }

    async fn check_vm_running_externally(&self, vm_id: &VMId) -> VMStatus {
        // First try using sysinfo crate
        let mut system = System::new();
        system.refresh_processes(ProcessesToUpdate::All, false);

        // First priority: Look for qemu-system processes directly
        for process in system.processes().values() {
            if let Some(cmd) = process.cmd().first() {
                let cmd_str = cmd.to_string_lossy();

                // Look for qemu processes with our VM config
                if cmd_str.contains("qemu-system")
                    && process.cmd().iter().any(|arg| {
                        let arg_str = arg.to_string_lossy();
                        arg_str.contains(&vm_id.0) || arg_str.contains(&format!("{}.conf", vm_id.0))
                    })
                {
                    println!("Found qemu-system process for VM '{}' with PID {}", vm_id.0, process.pid().as_u32());
                    return VMStatus::Running {
                        pid: process.pid().as_u32(),
                    };
                }
            }
        }

        // If no qemu-system found, check for quickemu wrapper and try to find its child
        for process in system.processes().values() {
            if let Some(cmd) = process.cmd().first() {
                let cmd_str = cmd.to_string_lossy();
                
                if cmd_str.contains("quickemu")
                    && process
                        .cmd()
                        .iter()
                        .any(|arg| arg.to_string_lossy().contains(&vm_id.0))
                {
                    // Found quickemu wrapper, now look for its qemu-system child
                    let wrapper_pid = process.pid().as_u32();
                    println!("Found quickemu wrapper for VM '{}' with PID {}, looking for qemu-system child", vm_id.0, wrapper_pid);
                    
                    // Refresh processes again to ensure we have the latest info
                    system.refresh_processes(ProcessesToUpdate::All, false);
                    
                    // Look for qemu-system processes that might be children of this quickemu
                    for child_process in system.processes().values() {
                        if let Some(child_cmd) = child_process.cmd().first() {
                            if child_cmd.to_string_lossy().contains("qemu-system")
                                && child_process.cmd().iter().any(|arg| {
                                    let arg_str = arg.to_string_lossy();
                                    arg_str.contains(&vm_id.0) || arg_str.contains(&format!("{}.conf", vm_id.0))
                                })
                            {
                                println!("Found qemu-system child process for VM '{}' with PID {}", vm_id.0, child_process.pid().as_u32());
                                return VMStatus::Running {
                                    pid: child_process.pid().as_u32(),
                                };
                            }
                        }
                    }
                    
                    // If we couldn't find the qemu-system child, return the wrapper PID as fallback
                    println!("Warning: Could not find qemu-system child for VM '{}', using wrapper PID {} (metrics may not work)", vm_id.0, wrapper_pid);
                    return VMStatus::Running {
                        pid: wrapper_pid,
                    };
                }
            }
        }

        // Fallback: Use ps command directly since sysinfo might have container issues
        if let Ok(output) = std::process::Command::new("ps").args(["aux"]).output() {
            let ps_output = String::from_utf8_lossy(&output.stdout);
            for line in ps_output.lines() {
                if line.contains("qemu-system") && line.contains(&vm_id.0) {
                    // Extract PID from ps output (second column)
                    if let Some(pid_str) = line.split_whitespace().nth(1) {
                        if let Ok(pid) = pid_str.parse::<u32>() {
                            println!("Found qemu-system via ps command for VM '{}' with PID {}", vm_id.0, pid);
                            return VMStatus::Running { pid };
                        }
                    }
                }
            }
        }

        VMStatus::Stopped
    }

    pub async fn create_vm_from_template(
        &self,
        template: &VMTemplate,
        output_dir: &Path,
    ) -> Result<PathBuf> {
        let quickget_path = self
            .quickget_path
            .as_ref()
            .ok_or_else(|| anyhow!("quickget not available"))?;

        let (tx, rx) = mpsc::channel();
        let quickget_path = quickget_path.clone();
        let template = template.clone();
        let template_os = template.os.clone();
        let template_version = template.version.clone();
        let output_dir_original = output_dir.to_path_buf();
        let output_dir = output_dir_original.clone();

        // Run quickget in a separate thread to avoid blocking
        thread::spawn(move || {
            let mut cmd = Command::new(&quickget_path);
            cmd.arg(&template.os)
                .arg(&template.version)
                .current_dir(&output_dir)
                .stdout(Stdio::piped())
                .stderr(Stdio::piped());

            match cmd.spawn() {
                Ok(mut child) => {
                    if let Some(stdout) = child.stdout.take() {
                        let reader = BufReader::new(stdout);
                        for line in reader.lines().map_while(Result::ok) {
                            let _ = tx.send(line);
                        }
                    }

                    match child.wait() {
                        Ok(status) => {
                            if status.success() {
                                let config_path = output_dir
                                    .join(format!("{}-{}.conf", template.os, template.version));
                                let _ = tx.send(format!(
                                    "VM created successfully: {}",
                                    config_path.display()
                                ));
                            } else {
                                let _ = tx.send("VM creation failed".to_string());
                            }
                        }
                        Err(e) => {
                            let _ = tx.send(format!("Error waiting for process: {e}"));
                        }
                    }
                }
                Err(e) => {
                    let _ = tx.send(format!("Failed to start quickget: {e}"));
                }
            }
        });

        // Wait for completion and return the config path
        let mut output_lines = Vec::new();
        while let Ok(line) = rx.recv() {
            output_lines.push(line.clone());
            if line.starts_with("VM created successfully:") || line.contains("failed") {
                break;
            }
        }

        let config_path =
            output_dir_original.join(format!("{template_os}-{template_version}.conf"));
        if config_path.exists() {
            Ok(config_path)
        } else {
            Err(anyhow!("VM creation failed: {}", output_lines.join("\n")))
        }
    }

    pub async fn is_vm_running(&self, vm_id: &VMId) -> bool {
        matches!(self.get_vm_status(vm_id).await, VMStatus::Running { .. })
    }

    pub async fn cleanup_finished_processes(&self) {
        let processes = self.processes.write().await;
        drop(processes);
    }

    pub fn get_quickemu_path(&self) -> &Path {
        &self.quickemu_path
    }

    pub fn get_quickget_path(&self) -> Option<&Path> {
        self.quickget_path.as_deref()
    }

    pub fn is_quickget_available(&self) -> bool {
        self.quickget_path.is_some()
    }

    pub async fn update_vm_status_from_running_processes(&self, vm: &mut VM) {
        vm.status = self.get_vm_status(&vm.id).await;
    }

    pub fn spawn_vm_creation_with_output(
        &self,
        template: VMTemplate,
        output_dir: PathBuf,
    ) -> Result<std::sync::mpsc::Receiver<String>> {
        let quickget_path = self
            .quickget_path
            .as_ref()
            .ok_or_else(|| anyhow!("quickget not available"))?;

        let (tx, rx) = mpsc::channel();
        let quickget_path = quickget_path.clone();
        let template_os = template.os.clone();
        let template_version = template.version.clone();

        thread::spawn(move || {
            println!("Attempting to run quickget at: {}", quickget_path.display());
            println!("Working directory: {}", output_dir.display());
            let cmd_str = if let Some(ref edition) = template.edition {
                format!(
                    "{} {} {} {}",
                    quickget_path.display(),
                    template_os,
                    template_version,
                    edition
                )
            } else {
                format!(
                    "{} {} {}",
                    quickget_path.display(),
                    template_os,
                    template_version
                )
            };
            println!("Command: {cmd_str}");

            // Ensure output directory exists
            if let Err(e) = std::fs::create_dir_all(&output_dir) {
                let _ = tx.send(format!(
                    "Failed to create directory {}: {}",
                    output_dir.display(),
                    e
                ));
                return;
            }

            let mut cmd = Command::new(&quickget_path);
            cmd.arg(&template_os).arg(&template_version);

            // Add edition if specified
            if let Some(ref edition) = template.edition {
                cmd.arg(edition);
            }

            cmd.current_dir(&output_dir)
                .stdout(Stdio::piped())
                .stderr(Stdio::piped());

            match cmd.spawn() {
                Ok(mut child) => {
                    let _ = tx.send("Quickget process started successfully".to_string());

                    // Handle stdout
                    if let Some(stdout) = child.stdout.take() {
                        let reader = BufReader::new(stdout);
                        for line in reader.lines().map_while(Result::ok) {
                            let _ = tx.send(format!("STDOUT: {line}"));
                        }
                    }

                    // Handle stderr
                    if let Some(stderr) = child.stderr.take() {
                        let reader = BufReader::new(stderr);
                        for line in reader.lines().map_while(Result::ok) {
                            let _ = tx.send(format!("STDERR: {line}"));
                        }
                    }

                    match child.wait() {
                        Ok(status) => {
                            if status.success() {
                                let config_path = output_dir
                                    .join(format!("{template_os}-{template_version}.conf"));
                                let _ = tx.send(format!(
                                    "VM created successfully: {}",
                                    config_path.display()
                                ));
                            } else {
                                let _ = tx.send(format!(
                                    "VM creation failed with exit code: {}",
                                    status.code().unwrap_or(-1)
                                ));
                            }
                        }
                        Err(e) => {
                            let _ = tx.send(format!("Error waiting for process: {e}"));
                        }
                    }
                }
                Err(e) => {
                    let _ = tx.send(format!(
                        "Failed to start quickget: {} (working dir: {})",
                        e,
                        output_dir.display()
                    ));
                }
            }
        });

        Ok(rx)
    }

    pub async fn update_vm_status(&self, vm: &mut VM) {
        vm.status = self.get_vm_status(&vm.id).await;
    }

    pub async fn launch_display(&self, vm: &VM) -> Result<()> {
        match &vm.config.display {
            DisplayProtocol::Spice { port } => {
                let url = format!("spice://localhost:{port}");
                // Try to open with system's default spice client
                if let Err(e) = std::process::Command::new("spicy").arg(&url).spawn() {
                    return Err(anyhow!("Failed to launch spice client: {}", e));
                }
            }
            DisplayProtocol::Vnc { port } => {
                let _url = format!("vnc://localhost:{port}");
                // Try to open with system's default vnc client
                if let Err(e) = std::process::Command::new("vncviewer")
                    .arg(format!("localhost:{port}"))
                    .spawn()
                {
                    return Err(anyhow!("Failed to launch VNC client: {}", e));
                }
            }
            _ => {
                return Err(anyhow!("Display protocol not supported for remote viewing"));
            }
        }
        Ok(())
    }

    /// Set the VNC proxy service for this VM manager
    pub fn set_vnc_proxy(&mut self, vnc_proxy: Arc<VncProxy>) {
        self.vnc_proxy = Some(vnc_proxy);
    }

    /// Detect the console port and protocol being used by a running VM
    pub async fn detect_console_port(
        &self,
        vm_id: &VMId,
    ) -> Result<Option<(u16, ConsoleProtocol)>> {
        println!("Detecting console port for VM '{}'", vm_id.0);

        // First check if VM is running
        let status = self.get_vm_status(vm_id).await;
        if !matches!(status, VMStatus::Running { .. }) {
            println!(
                "VM '{}' is not running, cannot detect console port",
                vm_id.0
            );
            return Ok(None);
        }

        println!("VM '{}' is running, scanning for console ports", vm_id.0);

        // Try common VNC ports first - QEMU uses ports starting from 5900
        // VNC typically uses ports 5900-5929 range (before SPICE range)
        for port in 5900..5930 {
            if self.is_port_open("127.0.0.1", port).await {
                println!("Found open port {} for VM '{}' in VNC range", port, vm_id.0);

                // Try to verify if this is actually a VNC port by checking if it's used by QEMU
                let ps_output = std::process::Command::new("lsof")
                    .args(["-i", &format!(":{port}"), "-n", "-P"])
                    .output();

                if let Ok(output) = ps_output {
                    let output_str = String::from_utf8_lossy(&output.stdout);
                    if output_str.contains("qemu") {
                        println!(
                            "Confirmed port {} is used by QEMU for VM '{}', assuming VNC",
                            port, vm_id.0
                        );
                        return Ok(Some((port, ConsoleProtocol::Vnc)));
                    }
                }

                // Even if lsof fails, still return the port as it might be VNC
                println!(
                    "Port {} is open in VNC range, assuming it's VNC for VM '{}'",
                    port, vm_id.0
                );
                return Ok(Some((port, ConsoleProtocol::Vnc)));
            }
        }

        println!("No VNC ports found in range 5900-5929 for VM '{}'", vm_id.0);

        // Check SPICE ports (5930-5999 range)
        for port in 5930..6000 {
            if self.is_port_open("127.0.0.1", port).await {
                println!("Found SPICE port {} for VM '{}'", port, vm_id.0);
                return Ok(Some((port, ConsoleProtocol::Spice)));
            }
        }

        println!("No console ports found for VM '{}'", vm_id.0);
        Ok(None)
    }

    /// Detect the actual VNC port being used by a running VM (backward compatibility)
    pub async fn detect_vnc_port(&self, vm_id: &VMId) -> Result<Option<u16>> {
        println!("Detecting VNC port for VM '{}'", vm_id.0);

        // First check if VM is running
        let status = self.get_vm_status(vm_id).await;
        if !matches!(status, VMStatus::Running { .. }) {
            println!("VM '{}' is not running, cannot detect VNC port", vm_id.0);
            return Ok(None);
        }

        println!("VM '{}' is running, scanning for VNC ports", vm_id.0);

        // Try common VNC ports first - QEMU uses ports starting from 5900
        // VNC typically uses ports 5900-5929 range (before SPICE range)
        for port in 5900..5930 {
            if self.is_port_open("127.0.0.1", port).await {
                println!("Found open port {} for VM '{}' in VNC range", port, vm_id.0);

                // Try to verify if this is actually a VNC port by checking if it's used by QEMU
                let ps_output = std::process::Command::new("lsof")
                    .args(["-i", &format!(":{port}"), "-n", "-P"])
                    .output();

                if let Ok(output) = ps_output {
                    let output_str = String::from_utf8_lossy(&output.stdout);
                    if output_str.contains("qemu") {
                        println!(
                            "Confirmed port {} is used by QEMU for VM '{}', assuming VNC",
                            port, vm_id.0
                        );
                        return Ok(Some(port));
                    }
                }

                // Even if lsof fails, still return the port as it might be VNC
                println!(
                    "Port {} is open in VNC range, assuming it's VNC for VM '{}'",
                    port, vm_id.0
                );
                return Ok(Some(port));
            }
        }

        println!("No VNC ports found in range 5900-5929 for VM '{}'", vm_id.0);

        // Check if VM is using SPICE instead (5930-5999 range)
        for port in 5930..6000 {
            if self.is_port_open("127.0.0.1", port).await {
                println!(
                    "Found SPICE port {} for VM '{}' - VM is using SPICE, not VNC",
                    port, vm_id.0
                );
                // This VM is using SPICE, not VNC
                return Err(anyhow!("VM '{}' is using SPICE display (port {}), not VNC. Please change the VM display protocol to VNC.", vm_id.0, port));
            }
        }

        println!("No VNC or SPICE ports found for VM '{}'", vm_id.0);
        Ok(None)
    }

    /// Check if a port is open and responsive
    async fn is_port_open(&self, host: &str, port: u16) -> bool {
        TcpStream::connect_timeout(
            &format!("{host}:{port}").parse().unwrap(),
            Duration::from_millis(100),
        )
        .is_ok()
    }

    /// Create a console session for a VM
    pub async fn create_console_session(&self, vm_id: &VMId) -> Result<ConsoleInfo> {
        self.create_console_session_with_host(vm_id, None).await
    }

    /// Create a console session for a VM with optional host override
    pub async fn create_console_session_with_host(
        &self,
        vm_id: &VMId,
        host_override: Option<String>,
    ) -> Result<ConsoleInfo> {
        println!("Creating console session for VM '{}'", vm_id.0);

        let vnc_proxy = self
            .vnc_proxy
            .as_ref()
            .ok_or_else(|| anyhow!("Console proxy not initialized"))?;

        // Detect the console port and protocol
        println!("Detecting console port for VM '{}'", vm_id.0);
        let (console_port, protocol) = self.detect_console_port(vm_id).await?.ok_or_else(|| {
            println!("No console server found for VM '{}'", vm_id.0);
            anyhow!("VM '{}' does not have an active console server", vm_id.0)
        })?;

        println!(
            "Detected {:?} console on port {} for VM '{}'",
            protocol, console_port, vm_id.0
        );

        // Create console connection
        let hostname = host_override.unwrap_or_else(|| "localhost".to_string());
        println!(
            "Creating {:?} proxy connection to {}:{} for VM '{}'",
            protocol, hostname, console_port, vm_id.0
        );

        let connection = vnc_proxy
            .create_connection(vm_id.0.clone(), hostname.clone(), console_port)
            .await?;

        let console_info = ConsoleInfo {
            websocket_url: format!("ws://{}:{}", hostname, connection.websocket_port),
            auth_token: connection.auth_token,
            connection_id: connection.id,
            protocol,
        };

        println!(
            "Console session created successfully for VM '{}' using {:?}: WebSocket URL: {}",
            vm_id.0, protocol, console_info.websocket_url
        );

        Ok(console_info)
    }

    /// Remove a console session
    pub async fn remove_console_session(&self, connection_id: &str) -> Result<()> {
        let vnc_proxy = self
            .vnc_proxy
            .as_ref()
            .ok_or_else(|| anyhow!("VNC proxy not initialized"))?;

        vnc_proxy.stop_connection(connection_id).await
    }

    /// Get console session status
    pub async fn get_console_status(
        &self,
        connection_id: &str,
    ) -> Result<Option<crate::services::vnc_proxy::ConnectionStatus>> {
        let vnc_proxy = self
            .vnc_proxy
            .as_ref()
            .ok_or_else(|| anyhow!("VNC proxy not initialized"))?;

        let status = vnc_proxy.get_connection_status(connection_id).await;
        Ok(status.map(|s| match s.as_str() {
            "authenticating" => crate::services::vnc_proxy::ConnectionStatus::Authenticating,
            "connected" => crate::services::vnc_proxy::ConnectionStatus::Connected,
            "disconnected" => crate::services::vnc_proxy::ConnectionStatus::Disconnected,
            error => crate::services::vnc_proxy::ConnectionStatus::Error(error.to_string()),
        }))
    }

    /// Check if a VM supports console access (VNC or SPICE)
    pub async fn supports_console_access(&self, vm: &VM) -> bool {
        // If VM is not running, it doesn't support console access
        if !vm.is_running() {
            println!(
                "VM '{}' is not running, console access not supported",
                vm.id.0
            );
            return false;
        }

        // Check if we can detect a console port (VNC or SPICE)
        match self.detect_console_port(&vm.id).await {
            Ok(Some((port, protocol))) => {
                println!(
                    "VM '{}' has {:?} console on port {}, console access supported",
                    vm.id.0, protocol, port
                );
                true
            }
            Ok(None) => {
                println!(
                    "VM '{}' has no console port detected, console access not supported",
                    vm.id.0
                );
                false
            }
            Err(e) => {
                println!(
                    "VM '{}' console detection error: {}, console access not supported",
                    vm.id.0, e
                );
                false
            }
        }
    }
}

impl Default for VMManager {
    fn default() -> Self {
        // Since we can't make Default async, use a synchronous fallback
        Self::with_paths(PathBuf::from("quickemu"), Some(PathBuf::from("quickget")))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{DisplayProtocol, VMConfig};
    use std::fs;
    use std::time::SystemTime;
    use tempfile::TempDir;

    fn create_test_vm_manager() -> VMManager {
        // Use mock paths for testing
        VMManager::with_paths(
            PathBuf::from("/usr/bin/echo"), // Use echo as a harmless substitute
            Some(PathBuf::from("/usr/bin/echo")),
        )
    }

    fn create_test_vm(temp_dir: &TempDir) -> VM {
        let config_path = temp_dir.path().join("test-vm.conf");
        let config_content = r#"
guest_os="ubuntu"
cpu_cores=2
ram="2G"
        "#;
        fs::write(&config_path, config_content).unwrap();

        VM {
            id: VMId("test-vm".to_string()),
            name: "Test VM".to_string(),
            config_path,
            status: VMStatus::Stopped,
            config: VMConfig {
                guest_os: "ubuntu".to_string(),
                disk_img: None,
                iso: None,
                ram: "2G".to_string(),
                cpu_cores: 2,
                disk_size: None,
                display: DisplayProtocol::Spice { port: 5930 },
                ssh_port: None,
                raw_config: config_content.to_string(),
            },
            last_modified: SystemTime::now(),
        }
    }

    #[test]
    fn test_vm_manager_creation() {
        let vm_manager = create_test_vm_manager();
        assert_eq!(vm_manager.get_quickemu_path(), Path::new("/usr/bin/echo"));
        assert_eq!(
            vm_manager.get_quickget_path(),
            Some(Path::new("/usr/bin/echo"))
        );
    }

    #[test]
    fn test_vm_manager_with_paths() {
        let quickemu_path = PathBuf::from("/custom/quickemu");
        let quickget_path = Some(PathBuf::from("/custom/quickget"));

        let vm_manager = VMManager::with_paths(quickemu_path.clone(), quickget_path.clone());

        assert_eq!(vm_manager.get_quickemu_path(), quickemu_path);
        assert_eq!(vm_manager.get_quickget_path(), quickget_path.as_deref());
    }

    #[tokio::test]
    async fn test_vm_status_stopped_initially() {
        let vm_manager = create_test_vm_manager();
        let vm_id = VMId("non-existent-vm".to_string());

        let status = vm_manager.get_vm_status(&vm_id).await;
        assert_eq!(status, VMStatus::Stopped);
    }

    #[tokio::test]
    async fn test_stop_non_existent_vm() {
        let vm_manager = create_test_vm_manager();
        let vm_id = VMId("non-existent-vm".to_string());

        let result = vm_manager.stop_vm(&vm_id).await;
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("VM process not found"));
    }

    #[tokio::test]
    async fn test_start_already_running_vm() {
        let temp_dir = TempDir::new().unwrap();
        let mut vm = create_test_vm(&temp_dir);
        vm.status = VMStatus::Running { pid: 12345 };

        let vm_manager = create_test_vm_manager();
        let result = vm_manager.start_vm(&vm).await;

        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("already running"));
    }

    #[tokio::test]
    async fn test_is_vm_running() {
        let vm_manager = create_test_vm_manager();
        let vm_id = VMId("test-vm".to_string());

        let is_running = vm_manager.is_vm_running(&vm_id).await;
        assert!(!is_running);
    }

    #[test]
    fn test_vm_template_creation() {
        let template = VMTemplate {
            name: "Ubuntu 22.04 Desktop".to_string(),
            os: "ubuntu".to_string(),
            version: "22.04".to_string(),
            edition: None,
            ram: "4G".to_string(),
            disk_size: "20G".to_string(),
            cpu_cores: 2,
        };

        assert_eq!(template.os, "ubuntu");
        assert_eq!(template.version, "22.04");
        assert_eq!(template.name, "Ubuntu 22.04 Desktop".to_string());
    }

    #[tokio::test]
    async fn test_cleanup_finished_processes() {
        let vm_manager = create_test_vm_manager();
        // This should not panic or error
        vm_manager.cleanup_finished_processes().await;
    }

    #[test]
    fn test_default_vm_manager() {
        // This test may fail if quickemu is not installed, but should not panic
        let _vm_manager = VMManager::default();
        // Just ensure we can create it without panicking
    }
}
