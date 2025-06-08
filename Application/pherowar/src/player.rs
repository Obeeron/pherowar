use anyhow::Result;
use rkyv::{from_bytes, rancor::Error, to_bytes};
use shared::{AntRequest, AntResponse, PlayerSetup};
use std::fs::{self, OpenOptions};
use std::io::{Read, Write};
use std::os::unix::net::UnixStream;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::thread;
use std::time::Duration;

use crate::config::PlayerConfig;

/// Represents a handle to a running Podman container.
pub struct ContainerHandle {
    /// The ID of the Podman container.
    pub container_id: String,
    // The child process for following logs.
    pub log_child: Option<std::process::Child>,
}

impl ContainerHandle {
    /// Stops the Podman container.
    pub fn stop(&self) {
        if let Err(e) = Command::new("podman")
            .args(["stop", "-t", "0", &self.container_id])
            .output()
        {
            eprintln!("Failed to stop container {}: {}", self.container_id, e);
        } else {
            println!("Container {} stopped", self.container_id);
        }
    }
}

impl Drop for ContainerHandle {
    /// Ensures the container is stopped when the handle is dropped.
    fn drop(&mut self) {
        self.stop();
        if let Some(mut child) = self.log_child.take() {
            // First, try to kill the “podman logs -f” process
            let _ = child.kill();
            // Optionally wait for it so it doesn’t become a zombie:
            let _ = child.wait();
        }
    }
}

/// Manages the connection to a player's AI, running in a Podman container.
pub struct PlayerConnection {
    /// The ID of the colony this player controls.
    pub colony_id: u32,
    /// Handle to the Podman container running the player's AI.
    #[allow(dead_code)]
    pub container: ContainerHandle,
    /// The Unix stream used to communicate with the player's AI.
    pub stream: UnixStream,
    /// Player setup information received from the AI upon connection.
    pub setup: PlayerSetup,
}

impl Drop for PlayerConnection {
    /// Cleans up resources (socket file and directory) when the connection is dropped.
    fn drop(&mut self) {
        let socket_dir = PathBuf::from(format!("/tmp/ant_sockets/{}", self.colony_id));
        let socket_path = socket_dir.join("pherowar.sock"); // Corrected socket file name
        if socket_path.exists() {
            if let Err(e) = fs::remove_file(&socket_path) {
                // Check result of remove_file
                eprintln!("Failed to remove socket file {:?}: {}", socket_path, e);
            }
        }
        if socket_dir.exists() {
            if let Err(e) = fs::remove_dir(&socket_dir) {
                // It's common for this to fail if the directory isn't empty (e.g. logs still being written or other files)
                // So, this might be more of a warning or debug log.
                println!(
                    "Attempted to remove socket dir {:?}, result: {:?}",
                    socket_dir, e
                );
            }
        }
        println!(
            "Cleaned up socket and directory for colony {}",
            self.colony_id
        );
    }
}

impl PlayerConnection {
    /// Starts a new player AI instance in a Podman container and establishes a connection.
    pub fn start(colony_id: u32, player_cfg: &PlayerConfig) -> Result<Self> {
        let socket_dir = PathBuf::from(format!("/tmp/ant_sockets/{}", colony_id));
        fs::create_dir_all(&socket_dir)?;
        let socket_path = socket_dir.join("pherowar.sock");
        if socket_path.exists() {
            fs::remove_file(&socket_path)?;
        }

        println!("Creating player container with socket at {:?}", socket_path);

        // Create container, mount the directory instead of the socket file
        let output = Command::new("podman")
            .args([
                "create",
                "--rm",
                "--security-opt",
                "no-new-privileges",
                "--cap-drop",
                "all",
                "--cpus=0.25",
                "-v",
                &format!("{}:/tmp/pherowar:z", socket_dir.to_string_lossy()),
                "-v",
                &format!("{}:/app/brain.so:z", player_cfg.so_path),
                "localhost/pherowar-player",
            ])
            .output()?;

        if !output.status.success() {
            anyhow::bail!(
                "Failed to create player container: {}",
                String::from_utf8_lossy(&output.stderr)
            );
        }

        let container_id = String::from_utf8_lossy(&output.stdout).trim().to_string();

        // Start following logs into a file
        let log_file_name = format!("{}_{}.log", player_cfg.name, colony_id);

        let log_file = OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(true)
            .open(&log_file_name)?;

        println!("Log file created: {}", log_file_name);
        let file_for_stderr = log_file.try_clone()?;

        println!(
            "Starting player container {} with logs in {}",
            container_id, log_file_name
        );
        let child = Command::new("podman")
            .args(&["logs", "-f", &container_id])
            .stdout(Stdio::from(log_file))
            .stderr(Stdio::from(file_for_stderr))
            .spawn()?;

        let container = ContainerHandle {
            container_id: container_id.clone(),
            log_child: Some(child),
        };

        // Start the container
        let start_output = Command::new("podman")
            .args(["start", &container_id])
            .output()?;

        if !start_output.status.success() {
            anyhow::bail!(
                "Failed to start player container: {}",
                String::from_utf8_lossy(&start_output.stderr)
            );
        }

        println!("Waiting for socket to become available...");

        // Wait for the socket file to appear and connect to it
        let mut retries = 30; // wait up to ~3 seconds
        let mut stream = loop {
            if socket_path.exists() {
                match UnixStream::connect(&socket_path) {
                    Ok(s) => break s,
                    Err(e) => {
                        retries -= 1;
                        if retries == 0 {
                            return Err(anyhow::anyhow!(
                                "Failed to connect to player socket: {}",
                                e
                            ));
                        }
                    }
                }
            } else {
                retries -= 1;
                if retries == 0 {
                    return Err(anyhow::anyhow!("Socket file not created by container"));
                }
            }
            thread::sleep(Duration::from_millis(100));
        };

        println!("Connected to player container!");

        // Send hello message to player
        stream.write_all(b"hello player")?;

        // receive length‑prefixed PlayerSetup
        let mut len_buf = [0u8; 4];
        stream.read_exact(&mut len_buf)?;
        let n = u32::from_le_bytes(len_buf) as usize;
        if n > 256 {
            anyhow::bail!("player sent oversized setup ({n} bytes)");
        }
        let mut setup_buf = vec![0u8; n];
        stream.read_exact(&mut setup_buf)?;

        let setup: PlayerSetup = from_bytes::<PlayerSetup, Error>(&setup_buf)
            .map_err(|e| anyhow::anyhow!("invalid PlayerSetup: {e}"))?;
        println!("Received PlayerSetup from player: {:?}", setup);

        Ok(PlayerConnection {
            colony_id,
            container,
            stream,
            setup,
        })
    }

    /// Sends a request to the player's AI and receives a response.
    pub fn player_update(&mut self, req: AntRequest) -> Result<AntResponse> {
        /* ---------- encode & send ---------- */
        let bytes = to_bytes::<Error>(&req)?;
        let len = bytes.len() as u32;

        self.stream.write_all(&len.to_le_bytes())?;
        self.stream.write_all(&bytes)?;

        /* ---------- receive & validate ------ */
        let mut len_buf = [0u8; 4];
        self.stream.read_exact(&mut len_buf)?;
        let resp_len = u32::from_le_bytes(len_buf) as usize;
        if resp_len > 256 {
            anyhow::bail!("player sent oversized response ({resp_len} bytes)");
        }

        let mut buf = vec![0u8; resp_len];
        self.stream.read_exact(&mut buf)?;

        // Safe: checked by rkyv + bytecheck
        let resp = from_bytes::<AntResponse, Error>(&buf) // docs.rs pattern :contentReference[oaicite:1]{index=1}
            .map_err(|e| anyhow::anyhow!("rkyv validation failed: {e}"))?;

        Ok(resp)
    }
}
