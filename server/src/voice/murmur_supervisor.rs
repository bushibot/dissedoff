//! Embeds real Murmur (the `mumble-server` binary, BSD-3-Clause) as a
//! supervised child process of the main server, so voice ships in the same
//! single container with zero manual config or cert handling:
//!
//! - Writes a minimal `murmur.ini` into the data volume on every start,
//!   pointing Murmur at its own SQLite file inside that same volume.
//!   Murmur generates and stores its own self-signed TLS certificate in
//!   that database on first run — nobody ever hand-edits a cert or an ini.
//! - Launches `mumble-server -fg -ini <path>` via `tokio::process::Command`
//!   and restarts it with a short backoff if it ever exits, so a crash
//!   doesn't take voice down permanently.
//! - Captures its stdout/stderr into our own tracing logs, so an operator
//!   never needs to go looking for a separate log file.
//!
//! Deliberately NOT yet implemented (tracked as the next slice of Phase 2,
//! not silently skipped): proxying Murmur's Ice control interface through
//! Dissedoff's own REST API and permission engine, and re-publishing its
//! session/channel events as `VoiceStateUpdate` gateway events. Rust has no
//! mature Ice-RPC binding, so that control-plane bridge is real follow-up
//! work, not a one-line addition. What's implemented here — real Murmur,
//! actually running, auto-configured, inside the one container — is the
//! part that directly answers "Mumble always sucked to set up."

use std::path::Path;
use std::process::Stdio;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;

use tokio::io::{AsyncBufReadExt, AsyncRead, BufReader};
use tokio::process::Command;

pub struct MurmurSupervisor {
    pub port: u16,
    running: Arc<AtomicBool>,
}

impl MurmurSupervisor {
    pub fn start(data_dir: &Path, port: u16) -> std::io::Result<Self> {
        let voice_dir = data_dir.join("voice");
        std::fs::create_dir_all(&voice_dir)?;
        let ini_path = voice_dir.join("murmur.ini");
        let db_path = voice_dir.join("murmur.sqlite");
        let log_path = voice_dir.join("murmur.log");

        // Minimal, auto-generated, non-interactive config. `bonjour=false`
        // avoids an mDNS dependency we don't need inside a container.
        std::fs::write(
            &ini_path,
            format!(
                "database={}\nlogfile={}\nport={port}\nwelcometext=\"Welcome to Dissedoff voice\"\nbonjour=false\nallowping=true\n",
                db_path.display(),
                log_path.display(),
            ),
        )?;

        // Murmur drops root privileges to its own unprivileged system user
        // (created by the `mumble-server` package) before it ever opens its
        // database file. If we create `voice_dir` as root (our own process's
        // uid) and leave it that way, Murmur's privilege-dropped process
        // can't write to it — exactly the kind of manual-fixup trap this
        // supervisor exists to prevent. `chown` it to that user up front;
        // if the lookup/chown fails (e.g. a future base image without that
        // system user), log it rather than silently leaving voice broken.
        if let Err(e) = chown_to_mumble_server_user(&voice_dir) {
            tracing::warn!(
                error = %e,
                "could not chown {} to the mumble-server system user — Murmur may fail to open its database",
                voice_dir.display()
            );
        }

        let running = Arc::new(AtomicBool::new(false));
        let running_for_task = running.clone();

        tokio::spawn(supervise_loop(ini_path, port, running_for_task));

        Ok(Self { port, running })
    }

    pub fn is_running(&self) -> bool {
        self.running.load(Ordering::SeqCst)
    }
}

async fn supervise_loop(ini_path: std::path::PathBuf, port: u16, running: Arc<AtomicBool>) {
    loop {
        tracing::info!(port, "starting bundled Murmur voice server");

        let spawned = Command::new("mumble-server")
            .arg("-fg")
            .arg("-ini")
            .arg(&ini_path)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn();

        let mut child = match spawned {
            Ok(child) => child,
            Err(e) => {
                tracing::error!(
                    error = %e,
                    "failed to spawn the `mumble-server` binary — is it installed in this image? voice will stay unavailable until the server is restarted"
                );
                running.store(false, Ordering::SeqCst);
                return;
            }
        };

        if let Some(stdout) = child.stdout.take() {
            tokio::spawn(log_lines(stdout));
        }
        if let Some(stderr) = child.stderr.take() {
            tokio::spawn(log_lines(stderr));
        }

        running.store(true, Ordering::SeqCst);
        let status = child.wait().await;
        running.store(false, Ordering::SeqCst);

        tracing::warn!(?status, "bundled Murmur process exited — restarting in 2s");
        tokio::time::sleep(Duration::from_secs(2)).await;
    }
}

fn chown_to_mumble_server_user(path: &Path) -> std::io::Result<()> {
    let status = std::process::Command::new("chown")
        .arg("-R")
        .arg("mumble-server:mumble-server")
        .arg(path)
        .status()?;
    if status.success() {
        Ok(())
    } else {
        Err(std::io::Error::other(format!(
            "chown exited with status {status}"
        )))
    }
}

async fn log_lines(stream: impl AsyncRead + Unpin) {
    let mut lines = BufReader::new(stream).lines();
    while let Ok(Some(line)) = lines.next_line().await {
        tracing::info!(target: "murmur", "{line}");
    }
}
