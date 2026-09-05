//! Authenticated loopback broker. Only renderer-owned fixture operations are exposed.
//! Events are wake hints; the main renderer must claim the canonical request.
#[path = "agent_bridge_ledger.rs"]
mod ledger;
#[path = "agent_bridge_storage.rs"]
mod storage;
#[path = "agent_bridge_wire.rs"]
mod wire;
use serde::Serialize;
use std::{
    io::{BufRead, BufReader, Write},
    net::{TcpListener, TcpStream},
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc, Mutex,
    },
    time::Duration,
};
use tauri::Emitter;
use wire::{Command, Request, Response};

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct AgentBridgeDispatch {
    pub renderer_generation: u64,
    pub request_id: String,
    pub method: String,
    pub params: serde_json::Value,
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct Descriptor {
    protocol_version: u32,
    port: u16,
    token: String,
    process_id: u32,
    executable_path: String,
    instance_id: String,
}
struct Inner {
    token: String,
    ledger: Mutex<ledger::Ledger>,
    emit: Box<dyn Fn(&AgentBridgeDispatch) -> Result<(), String> + Send + Sync>,
    connections: AtomicUsize,
    _lock: std::fs::File,
    descriptor_path: PathBuf,
}
impl Drop for Inner {
    fn drop(&mut self) {
        // The exclusive instance lock is still held, so this is our descriptor.
        let _ = std::fs::remove_file(&self.descriptor_path);
    }
}
pub(crate) struct AgentBridge {
    inner: Arc<Inner>,
}
impl AgentBridge {
    pub(crate) fn start(app: tauri::AppHandle, directory: &Path) -> Result<Self, String> {
        std::fs::create_dir_all(directory).map_err(|_| "agent_directory_unavailable")?;
        let lock = storage::instance_lock(directory)?;
        let listener = TcpListener::bind((std::net::Ipv4Addr::LOCALHOST, 0))
            .map_err(|_| "agent_bind_failed")?;
        listener
            .set_nonblocking(true)
            .map_err(|_| "agent_listener_failed")?;
        let port = listener
            .local_addr()
            .map_err(|_| "agent_address_failed")?
            .port();
        let token = storage::random_hex(32)?;
        let descriptor_path = directory.join("agent-bridge-v1.json");
        let ledger = ledger::Ledger::new(Some(directory.join("agent-bridge-ledger-v1.json")))?;
        let descriptor = Descriptor {
            protocol_version: 1,
            port,
            token: token.clone(),
            process_id: std::process::id(),
            executable_path: std::env::current_exe()
                .map_err(|_| "agent_executable_unknown")?
                .to_string_lossy()
                .into_owned(),
            instance_id: storage::random_hex(16)?,
        };
        let inner = Arc::new(Inner {
            token,
            ledger: Mutex::new(ledger),
            emit: Box::new(move |request| {
                app.emit_to("main", "syndocal://agent-request-v1", request)
                    .map_err(|_| "renderer_dispatch_failed".to_string())
            }),
            connections: AtomicUsize::new(0),
            _lock: lock,
            descriptor_path,
        });
        let weak = Arc::downgrade(&inner);
        std::thread::Builder::new()
            .name("syndocal-agent-listener".to_string())
            .spawn(move || loop {
                let Some(inner) = weak.upgrade() else {
                    break;
                };
                match listener.accept() {
                    Ok((stream, peer)) if peer.ip().is_loopback() => {
                        if inner.connections.fetch_add(1, Ordering::AcqRel) >= 8 {
                            inner.connections.fetch_sub(1, Ordering::AcqRel);
                            drop(stream);
                            continue;
                        }
                        let connection = Arc::clone(&inner);
                        if std::thread::Builder::new()
                            .name("syndocal-agent-request".to_string())
                            .spawn(move || {
                                serve(stream, &connection);
                                connection.connections.fetch_sub(1, Ordering::AcqRel);
                            })
                            .is_err()
                        {
                            inner.connections.fetch_sub(1, Ordering::AcqRel);
                        }
                    }
                    Ok(_) => {}
                    Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                        drop(inner);
                        std::thread::sleep(Duration::from_millis(50));
                    }
                    Err(_) => break,
                }
            })
            .map_err(|_| "agent_listener_start_failed")?;
        storage::atomic_write(
            &inner.descriptor_path,
            &serde_json::to_vec(&descriptor).map_err(|_| "descriptor_encode_failed")?,
        )?;
        Ok(Self { inner })
    }
    pub(crate) fn register(&self, window_label: &str) -> Result<u64, String> {
        main_only(window_label)?;
        self.inner
            .ledger
            .lock()
            .map_err(|_| "agent_state_poisoned")?
            .register()
    }
    pub(crate) fn claim(
        &self,
        window_label: &str,
        renderer_generation: u64,
        request_id: &str,
    ) -> Result<AgentBridgeDispatch, String> {
        main_only(window_label)?;
        self.inner
            .ledger
            .lock()
            .map_err(|_| "agent_state_poisoned")?
            .claim(renderer_generation, request_id)
    }
    pub(crate) fn complete(
        &self,
        window_label: &str,
        renderer_generation: u64,
        request_id: &str,
        result: serde_json::Value,
    ) -> Result<(), String> {
        main_only(window_label)?;
        self.inner
            .ledger
            .lock()
            .map_err(|_| "agent_state_poisoned")?
            .complete(renderer_generation, request_id, result)
    }
}
fn main_only(label: &str) -> Result<(), String> {
    if label == "main" {
        Ok(())
    } else {
        Err("main_renderer_required".to_string())
    }
}
fn token_matches(expected: &str, supplied: &str) -> bool {
    let mismatch = expected
        .as_bytes()
        .iter()
        .zip(supplied.as_bytes())
        .fold(0u8, |value, (a, b)| value | (a ^ b));
    expected.len() == supplied.len() && mismatch == 0
}
fn process(inner: &Inner, bytes: &[u8]) -> Response {
    if bytes.len() > wire::MAX_REQUEST_BYTES {
        return Response::rejected("", "request_oversize");
    }
    let request: Request = match serde_json::from_slice(bytes) {
        Ok(value) => value,
        Err(_) => return Response::rejected("", "invalid_request"),
    };
    if !token_matches(&inner.token, &request.token) {
        return Response::rejected("", "auth_failed");
    }
    let command = match request.command() {
        Ok(value) => value,
        Err(error) => return Response::rejected(&request.request_id, error),
    };
    let mut ledger = match inner.ledger.lock() {
        Ok(value) => value,
        Err(_) => return Response::rejected(&request.request_id, "agent_state_poisoned"),
    };
    if let Command::Status(status) = &command {
        return ledger.status(&status.request_id);
    }
    let (response, dispatch) = match ledger.begin(&request.request_id, &command) {
        Ok(value) => value,
        Err(error) => return Response::rejected(&request.request_id, &error),
    };
    drop(ledger);
    if let Some(dispatch) = dispatch {
        if (inner.emit)(&dispatch).is_err() {
            if let Ok(mut ledger) = inner.ledger.lock() {
                ledger.dispatch_failed(&request.request_id);
            }
            return Response::status(&request.request_id, "unknown");
        }
    }
    response
}
fn serve(mut stream: TcpStream, inner: &Inner) {
    let _ = stream.set_read_timeout(Some(Duration::from_secs(2)));
    let _ = stream.set_write_timeout(Some(Duration::from_secs(2)));
    let mut bytes = Vec::new();
    // Take bounds memory even when a peer never sends a newline.
    let response = {
        use std::io::Read;
        let mut reader = BufReader::new((&mut stream).take((wire::MAX_REQUEST_BYTES + 1) as u64));
        match reader.read_until(b'\n', &mut bytes) {
            Ok(_) if bytes.last() == Some(&b'\n') && bytes.len() <= wire::MAX_REQUEST_BYTES => {
                process(inner, &bytes)
            }
            Ok(_) => Response::rejected("", "request_oversize_or_incomplete"),
            Err(_) => Response::status("", "unknown"),
        }
    };
    if let Ok(mut bytes) = serde_json::to_vec(&response) {
        bytes.push(b'\n');
        let _ = stream.write_all(&bytes);
    }
}
#[cfg(test)]
#[path = "agent_bridge_tests.rs"]
mod tests;
