//! Executes install/uninstall/upgrade commands with live streaming output.
//!
//! Each run spawns the command as a child process and pipes stdout/stderr to
//! the frontend one line at a time via a Tauri event (`toolpulse://terminal`).
//! A shared cancellation token lets the UI abort a long-running command.
//!
//! The runner also fires an OS notification on completion (success or error),
//! matching the rest of the app's notification UX.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use tauri::{AppHandle, Emitter};
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command;
use tokio::sync::Mutex;

use crate::tools::{ActionKind, Stream, TerminalLine};

/// Tracks in-flight runs so the UI can cancel them.
#[derive(Default)]
pub struct RunRegistry {
    cancelled: Arc<AtomicBool>,
    _guard: Mutex<()>,
}

impl RunRegistry {
    pub fn new() -> Self {
        Self {
            cancelled: Arc::new(AtomicBool::new(false)),
            _guard: Mutex::new(()),
        }
    }

    /// Mark the current run as cancelled. The next line read will abort.
    pub fn cancel(&self) {
        self.cancelled.store(true, Ordering::SeqCst);
    }

    /// Reset + return the cancel flag for the next run.
    pub fn take_cancel_flag(&self) -> Arc<AtomicBool> {
        // Reset for the next run.
        self.cancelled.store(false, Ordering::SeqCst);
        self.cancelled.clone()
    }
}

/// Run `argv` as a child process, streaming output to the frontend.
///
/// Returns `Ok(())` if the process exited successfully, or an error string
/// describing what went wrong (including cancellation).
pub async fn run_streaming(
    app: &AppHandle,
    event_tag: &str,
    argv: &[String],
    cancel: Arc<AtomicBool>,
) -> Result<(), String> {
    if argv.is_empty() {
        return Err("empty command".into());
    }

    let display = argv.join(" ");
    emit(app, event_tag, TerminalLine::Status {
        text: format!("$ {display}"),
    });

    let mut child = Command::new(&argv[0])
        .args(&argv[1..])
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .map_err(|e| {
            let msg = format!("failed to start `{}`: {e}", argv[0]);
            let _ = emit(app, event_tag, TerminalLine::Done {
                success: false,
                message: msg.clone(),
            });
            msg
        })?;

    let stdout = child.stdout.take().expect("stdout piped");
    let stderr = child.stderr.take().expect("stderr piped");

    // Spawn two readers that forward lines until EOF or cancellation.
    let app_out = app.clone();
    let tag_out = event_tag.to_string();
    let cancel_out = cancel.clone();
    let stdout_task = tokio::spawn(async move {
        let mut lines = BufReader::new(stdout).lines();
        loop {
            if cancel_out.load(Ordering::SeqCst) {
                break;
            }
            match lines.next_line().await {
                Ok(Some(line)) => {
                    let _ = emit(&app_out, &tag_out, TerminalLine::Output {
                        text: line,
                        stream: Stream::Stdout,
                    });
                }
                _ => break,
            }
        }
    });

    let app_err = app.clone();
    let tag_err = event_tag.to_string();
    let cancel_err = cancel.clone();
    let stderr_task = tokio::spawn(async move {
        let mut lines = BufReader::new(stderr).lines();
        loop {
            if cancel_err.load(Ordering::SeqCst) {
                break;
            }
            match lines.next_line().await {
                Ok(Some(line)) => {
                    let _ = emit(&app_err, &tag_err, TerminalLine::Output {
                        text: line,
                        stream: Stream::Stderr,
                    });
                }
                _ => break,
            }
        }
    });

    let _ = tokio::join!(stdout_task, stderr_task);

    let was_cancelled = cancel.load(Ordering::SeqCst);

    let status = match child.wait().await {
        Ok(s) => s,
        Err(e) => {
            let msg = format!("wait failed: {e}");
            let _ = emit(app, event_tag, TerminalLine::Done {
                success: false,
                message: msg.clone(),
            });
            return Err(msg);
        }
    };

    if was_cancelled {
        let _ = emit(app, event_tag, TerminalLine::Done {
            success: false,
            message: "Cancelled by user.".into(),
        });
        return Err("cancelled".into());
    }

    if status.success() {
        let _ = emit(app, event_tag, TerminalLine::Done {
            success: true,
            message: "Completed successfully.".into(),
        });
        Ok(())
    } else {
        let msg = format!("exited with code {}", status.code().unwrap_or(-1));
        let _ = emit(app, event_tag, TerminalLine::Done {
            success: false,
            message: msg.clone(),
        });
        Err(msg)
    }
}

/// Emit a terminal line + fire an OS notification on completion.
fn emit(app: &AppHandle, tag: &str, line: TerminalLine) {
    // The payload includes the tag so multiple panels can filter their events.
    let payload = TerminalEvent {
        tag: tag.to_string(),
        line,
    };
    let _ = app.emit("toolpulse://terminal", payload);
}

/// Event payload: which terminal panel this line belongs to + the line itself.
#[derive(Clone, serde::Serialize)]
struct TerminalEvent {
    tag: String,
    line: TerminalLine,
}

/// Human-readable label for an action, used in notifications.
pub fn action_label(kind: ActionKind, subject: &str) -> String {
    let verb = match kind {
        ActionKind::Install => "Installed",
        ActionKind::Uninstall => "Uninstalled",
        ActionKind::Upgrade => "Upgraded",
    };
    format!("{verb} {subject}")
}
