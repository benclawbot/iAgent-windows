//! iAgent ambient service with Windows named-pipe IPC server.
//!
//! This binary runs the ambient background agent with an optional named-pipe
//! interface for external clients (e.g. the iAgent desktop app).

use iagent::protocol::ipc::{
    CancelRequest, ClientMessage, DoneEvent, ErrorCode, ErrorEvent, ServerEvent, StatusEvent,
    StatusRequest, TaskRequest, TextEvent, ThinkingEvent,
};
use std::io::{BufRead, BufReader, Write};

const PIPE_NAME: &str = r"\\.\pipe\iagent";

/// Dispatch a client message and return a vector of server events to send back.
fn dispatch_message(msg: ClientMessage) -> Vec<ServerEvent> {
    match msg {
        ClientMessage::Task(TaskRequest { id, prompt, .. }) => {
            // Stub: return a thinking event followed by a done event.
            vec![
                ServerEvent::Thinking(ThinkingEvent {
                    task_id: id.clone(),
                }),
                ServerEvent::Text(TextEvent {
                    task_id: id.clone(),
                    chunk: format!("Received task: {}", prompt),
                }),
                ServerEvent::Done(DoneEvent {
                    task_id: id,
                    tokens_used: None,
                }),
            ]
        }
        ClientMessage::Cancel(CancelRequest { task_id }) => {
            vec![ServerEvent::Error(ErrorEvent {
                task_id,
                message: "Task cancelled".to_string(),
                code: ErrorCode::Cancelled,
            })]
        }
        ClientMessage::Status(StatusRequest {}) => {
            vec![ServerEvent::Status(StatusEvent {
                version: env!("CARGO_PKG_VERSION").to_string(),
                active_tasks: 0,
                default_provider: "openai".to_string(),
                providers_available: vec!["openai".to_string(), "openrouter".to_string()],
            })]
        }
    }
}

/// Handle a single client connection.
fn handle_connection<H>(mut reader: BufReader<H>, writer: &mut dyn Write) -> std::io::Result<()>
where
    H: std::io::Read,
{
    let mut line = String::new();
    while reader.read_line(&mut line)? > 0 {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            line.clear();
            continue;
        }

        // Parse the client message.
        let msg: ClientMessage = match serde_json::from_str(trimmed) {
            Ok(m) => m,
            Err(e) => {
                let err = ServerEvent::Error(ErrorEvent {
                    task_id: String::new(),
                    message: format!("Parse error: {}", e),
                    code: ErrorCode::Internal,
                });
                writeln!(writer, "{}", serde_json::to_string(&err).unwrap())?;
                line.clear();
                continue;
            }
        };

        // Dispatch and write each resulting event.
        let events = dispatch_message(msg);
        for event in events {
            writeln!(writer, "{}", serde_json::to_string(&event).unwrap())?;
        }
        line.clear();
    }
    Ok(())
}

/// Run the IPC server using Windows named pipes.
#[cfg(target_os = "windows")]
async fn run_windows_pipe_server() -> anyhow::Result<()> {
    loop {
        // Create and listen on the named pipe.
        let mut pipe = windows_pipe::Pipe::create_server(PIPE_NAME)?;

        let reader_pipe = pipe.try_clone()?;
        let reader = BufReader::new(reader_pipe);
        let _ = handle_connection(reader, &mut pipe);
    }
}

// -----------------------------------------------------------------------
// Unix stub (for local development on non-Windows machines)
// -----------------------------------------------------------------------

/// Unix stub: named pipes are Windows-only.  On Unix this is a no-op.
#[cfg(not(target_os = "windows"))]
async fn run_windows_pipe_server() -> anyhow::Result<()> {
    eprintln!("Named-pipe IPC is Windows-only; skipping pipe server.");
    Ok(())
}

/// Unix stub socket server for development.
#[cfg(not(target_os = "windows"))]
async fn run_unix_socket_server() -> anyhow::Result<()> {
    eprintln!("Unix socket server not implemented; use Windows for IPC.");
    Ok(())
}

// -----------------------------------------------------------------------
// Windows FFI helpers
// -----------------------------------------------------------------------

#[cfg(target_os = "windows")]
mod windows_pipe {
    use std::ffi::OsStr;
    use std::fs::File;
    use std::io::{Read, Write};
    use std::os::windows::ffi::OsStrExt;
    use std::os::windows::io::{FromRawHandle, RawHandle};
    use std::ptr;

    const PIPE_ACCESS_DUPLEX: u32 = 0x00000003;
    const PIPE_TYPE_BYTE: u32 = 0x00000000;
    const PIPE_READMODE_BYTE: u32 = 0x00000000;
    const PIPE_WAIT: u32 = 0x00000000;
    const PIPE_UNLIMITED_INSTANCES: u32 = 0x000000FF;
    const INVALID_HANDLE_VALUE: *mut std::ffi::c_void = -1isize as *mut std::ffi::c_void;

    #[link(name = "kernel32")]
    unsafe extern "system" {
        fn CreateNamedPipeW(
            name: *const u16,
            open_mode: u32,
            pipe_mode: u32,
            max_instances: u32,
            out_buffer_size: u32,
            in_buffer_size: u32,
            default_timeout: u32,
            security_attributes: *mut std::ffi::c_void,
        ) -> *mut std::ffi::c_void;

        fn ConnectNamedPipe(
            hNamedPipe: *mut std::ffi::c_void,
            lpOverlapped: *mut std::ffi::c_void,
        ) -> i32;
        fn CloseHandle(hObject: *mut std::ffi::c_void) -> i32;
    }

    /// Represents a Windows named pipe handle.
    #[derive(Debug)]
    pub struct Pipe {
        file: File,
    }

    impl Pipe {
        pub fn create_server(pipe_name: &str) -> std::io::Result<Self> {
            let wide: Vec<u16> = OsStr::new(pipe_name)
                .encode_wide()
                .chain(std::iter::once(0))
                .collect();

            let handle = unsafe {
                CreateNamedPipeW(
                    wide.as_ptr(),
                    PIPE_ACCESS_DUPLEX,
                    PIPE_TYPE_BYTE | PIPE_READMODE_BYTE | PIPE_WAIT,
                    PIPE_UNLIMITED_INSTANCES,
                    65536,
                    65536,
                    0,
                    ptr::null_mut(),
                )
            };
            if handle == INVALID_HANDLE_VALUE {
                return Err(std::io::Error::last_os_error());
            }

            let connected = unsafe { ConnectNamedPipe(handle as *mut _, ptr::null_mut()) };
            if connected == 0 {
                let err = std::io::Error::last_os_error();
                unsafe {
                    CloseHandle(handle);
                }
                return Err(err);
            }

            let file = unsafe { File::from_raw_handle(handle as RawHandle) };
            Ok(Self { file })
        }

        pub fn try_clone(&self) -> std::io::Result<Self> {
            Ok(Self {
                file: self.file.try_clone()?,
            })
        }

        /// Connect to an existing pipe (client-side).
        #[allow(dead_code)]
        pub fn connect(pipe_name: &str) -> std::io::Result<Self> {
            Self::create_server(pipe_name)
        }
    }

    impl Read for Pipe {
        fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
            self.file.read(buf)
        }
    }

    impl Write for Pipe {
        fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
            self.file.write(buf)
        }

        fn flush(&mut self) -> std::io::Result<()> {
            self.file.flush()
        }
    }

    /// RAII guard that disconnects the pipe when dropped.
    #[allow(dead_code)]
    pub struct PipeConnection {
        pipe: Pipe,
    }

    impl Drop for PipeConnection {
        fn drop(&mut self) {
            let _ = self.pipe.flush();
        }
    }
}

#[cfg(target_os = "windows")]
use windows_pipe::Pipe;

#[cfg(target_os = "windows")]
impl Pipe {
    /// Connect client-side to the iAgent pipe.
    #[allow(dead_code)]
    pub fn connect_to_server() -> std::io::Result<Self> {
        Pipe::connect(PIPE_NAME)
    }
}

// -----------------------------------------------------------------------
// Main entry point
// -----------------------------------------------------------------------

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Spawn the IPC server task (Windows named pipe).
    let _pipe_server = tokio::spawn(async {
        if let Err(e) = run_windows_pipe_server().await {
            eprintln!("Pipe server error: {}", e);
        }
    });

    // Run the desktop ambient service alongside the IPC server.
    iagent::desktop_ambient::run(false).await
}
