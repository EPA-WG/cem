//! Native stdio/TCP host for the common DAP session projection.

use std::collections::BTreeMap;
use std::io::{self, BufRead, BufReader, Read, Write};
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::sync::{mpsc, Arc, Mutex};
use std::thread;
use std::time::Duration;

use cem_ml::dap::{
    DapAdapterError, DapOperationHost, DapRequest, DapSession, MAX_DAP_MESSAGE_BYTES,
};
use cem_ml::operation_control::{ControlCause, ControlFailure, OperationControl, OperationId};
use cem_ml::operation_handle::{ArtifactDisposition, OperationHandle, OperationOutcome};
use cem_ml::real::RealCemMlEngine;
use cem_ml::scheduler::AbortSignal;
use cem_ml_transform_cem_ql::CemQlDebugConditionEvaluator;
use clap::Parser;
use serde_json::Value;

use crate::cli::{Cli, Command, DebugArgs};
use crate::dispatch::{self, Streams};

const TRANSPORT_POLL: Duration = Duration::from_millis(20);

enum DebugHostEvent {
    Output {
        category: &'static str,
        output: String,
    },
}

struct CliDapHost {
    events: mpsc::Sender<DebugHostEvent>,
    active_abort: Arc<Mutex<Option<AbortSignal>>>,
    operations: BTreeMap<OperationId, OperationHandle<u8>>,
    pending_launch: Option<mpsc::Sender<()>>,
    wait_after_disconnect: bool,
}

impl CliDapHost {
    fn new(
        events: mpsc::Sender<DebugHostEvent>,
        active_abort: Arc<Mutex<Option<AbortSignal>>>,
    ) -> Self {
        Self {
            events,
            active_abort,
            operations: BTreeMap::new(),
            pending_launch: None,
            wait_after_disconnect: false,
        }
    }

    fn wait_for_detached_operations(&self) {
        if !self.wait_after_disconnect {
            return;
        }
        for operation in self.operations.values() {
            while matches!(
                operation.blocking_result_timeout(Duration::from_secs(1)),
                Ok(None)
            ) {}
        }
    }
}

impl DapOperationHost<u8> for CliDapHost {
    fn launch(&mut self, arguments: &Value) -> Result<OperationHandle<u8>, DapAdapterError> {
        let command = arguments
            .get("command")
            .and_then(Value::as_str)
            .filter(|command| !command.is_empty())
            .ok_or_else(|| {
                DapAdapterError::new(
                    "cem.dap.launch_command_required",
                    "launch.command must name an existing cem-ml command",
                )
            })?;
        let mut argv = vec!["cem-ml".to_owned(), command.to_owned()];
        if let Some(arguments) = arguments.get("args") {
            let arguments = arguments.as_array().ok_or_else(|| {
                DapAdapterError::new(
                    "cem.dap.launch_args_invalid",
                    "launch.args must be an array of strings",
                )
            })?;
            for argument in arguments {
                argv.push(
                    argument
                        .as_str()
                        .ok_or_else(|| {
                            DapAdapterError::new(
                                "cem.dap.launch_args_invalid",
                                "launch.args must contain only strings",
                            )
                        })?
                        .to_owned(),
                );
            }
        }
        let parsed = Cli::try_parse_from(argv)
            .map_err(|error| DapAdapterError::new("cem.dap.launch_arguments", error.to_string()))?;
        if matches!(parsed.command, Command::Debug(_)) {
            return Err(DapAdapterError::new(
                "cem.dap.recursive_debug_launch",
                "a debug adapter cannot launch another debug transport",
            ));
        }

        let abort_signal = AbortSignal::new();
        let control = OperationControl::new(abort_signal.clone());
        let (handle, terminal) = OperationHandle::with_defaults(control.clone())?;
        handle.activate_debug_control(Some(Arc::new(CemQlDebugConditionEvaluator)))?;
        *self
            .active_abort
            .lock()
            .expect("poisoned debug abort mutex") = Some(abort_signal.clone());
        self.operations
            .insert(handle.operation_id(), handle.clone());

        let (start_tx, start_rx) = mpsc::channel();
        self.pending_launch = Some(start_tx);
        let host_events = self.events.clone();
        let active_abort = Arc::clone(&self.active_abort);
        thread::spawn(move || {
            let configured = loop {
                match start_rx.recv_timeout(TRANSPORT_POLL) {
                    Ok(()) => break !control.is_cancelled(),
                    Err(mpsc::RecvTimeoutError::Timeout) if control.is_cancelled() => break false,
                    Err(mpsc::RecvTimeoutError::Timeout) => {}
                    Err(mpsc::RecvTimeoutError::Disconnected) => break false,
                }
            };
            if !configured {
                let _ = terminal.settle(OperationOutcome::cancelled(
                    Some("debug launch ended before configuration completed".to_owned()),
                    Vec::new(),
                    ArtifactDisposition::default(),
                ));
                *active_abort.lock().expect("poisoned debug abort mutex") = None;
                return;
            }
            let quiet = parsed.quiet;
            let no_color = parsed.no_color;
            let mut stdout = Vec::new();
            let mut stderr = Vec::new();
            let mut streams = Streams {
                stdout: &mut stdout,
                stderr: &mut stderr,
                quiet,
                no_color,
                abort_signal: abort_signal.clone(),
                operation_control: Some(control.clone()),
            };
            let outcome = dispatch::dispatch(&RealCemMlEngine::new(), parsed, &mut streams);
            drop(streams);
            if !stdout.is_empty() {
                let _ = host_events.send(DebugHostEvent::Output {
                    category: "stdout",
                    output: String::from_utf8_lossy(&stdout).into_owned(),
                });
            }
            if !stderr.is_empty() {
                let _ = host_events.send(DebugHostEvent::Output {
                    category: "stderr",
                    output: String::from_utf8_lossy(&stderr).into_owned(),
                });
            }
            let terminal_outcome = match outcome.exit_code {
                dispatch::EXIT_OK => OperationOutcome::succeeded(
                    outcome.exit_code,
                    Vec::new(),
                    ArtifactDisposition::default(),
                ),
                dispatch::EXIT_CANCELLED => OperationOutcome::cancelled(
                    Some("operation cancelled".to_owned()),
                    Vec::new(),
                    ArtifactDisposition::default(),
                ),
                exit_code => OperationOutcome::failed(
                    ControlFailure {
                        operation_id: control.operation_id(),
                        affected_scope: control.root_scope(),
                        cause: ControlCause::InternalFailure {
                            diagnostic_code: format!("cem.cli.exit.{exit_code}"),
                        },
                        source_map: None,
                    },
                    Vec::new(),
                    ArtifactDisposition::default(),
                ),
            };
            let _ = terminal.settle(terminal_outcome);
            *active_abort.lock().expect("poisoned debug abort mutex") = None;
        });
        Ok(handle)
    }

    fn attach(&mut self, arguments: &Value) -> Result<OperationHandle<u8>, DapAdapterError> {
        let operation_id = arguments
            .get("operationId")
            .and_then(Value::as_u64)
            .map(OperationId::from_raw)
            .ok_or_else(|| {
                DapAdapterError::new(
                    "cem.dap.attach_operation_required",
                    "attach.operationId is required",
                )
            })?;
        self.operations.get(&operation_id).cloned().ok_or_else(|| {
            DapAdapterError::new(
                "cem.dap.attach_operation_unknown",
                format!("operation {operation_id} is not hosted by this adapter"),
            )
        })
    }

    fn configuration_done(&mut self) -> Result<(), DapAdapterError> {
        if let Some(start) = self.pending_launch.take() {
            start.send(()).map_err(|_| {
                DapAdapterError::new(
                    "cem.dap.launch_start_failed",
                    "launched command ended before configuration completed",
                )
            })?;
        }
        Ok(())
    }

    fn disconnect(&mut self, terminate: bool) -> Result<(), DapAdapterError> {
        if !terminate {
            self.configuration_done()?;
            self.wait_after_disconnect = true;
        }
        Ok(())
    }

    fn supports_conditional_breakpoints(&self) -> bool {
        true
    }
}

pub fn run(arguments: &DebugArgs) -> u8 {
    let active_abort = Arc::new(Mutex::new(None::<AbortSignal>));
    let signal_abort = Arc::clone(&active_abort);
    if let Err(error) = ctrlc::set_handler(move || {
        if let Some(abort) = signal_abort
            .lock()
            .expect("poisoned debug abort mutex")
            .as_ref()
        {
            abort.abort();
        }
    }) {
        eprintln!("cem-ml: cannot install signal handler: {error}");
        return dispatch::EXIT_INTERNAL;
    }

    let result = if arguments.stdio {
        serve_connection(io::stdin(), io::stdout(), active_abort)
    } else if let Some(endpoint) = &arguments.listen {
        serve_tcp(endpoint, active_abort)
    } else {
        Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "one debug transport is required",
        ))
    };
    match result {
        Ok(()) => dispatch::EXIT_OK,
        Err(error) => {
            eprintln!("cem-ml: debug transport failed: {error}");
            dispatch::EXIT_INTERNAL
        }
    }
}

fn serve_tcp(endpoint: &str, active_abort: Arc<Mutex<Option<AbortSignal>>>) -> io::Result<()> {
    let endpoint = endpoint.parse::<SocketAddr>().map_err(|error| {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("invalid --listen endpoint: {error}"),
        )
    })?;
    if !endpoint.ip().is_loopback() {
        return Err(io::Error::new(
            io::ErrorKind::PermissionDenied,
            "--listen accepts loopback addresses only",
        ));
    }
    let listener = TcpListener::bind(endpoint)?;
    eprintln!("cem-ml debug listening on {}", listener.local_addr()?);
    let (stream, _) = listener.accept()?;
    let reader = stream.try_clone()?;
    serve_connection(reader, stream, active_abort)
}

fn serve_connection<R, W>(
    reader: R,
    mut writer: W,
    active_abort: Arc<Mutex<Option<AbortSignal>>>,
) -> io::Result<()>
where
    R: Read + Send + 'static,
    W: Write,
{
    let (input_tx, input_rx) = mpsc::channel();
    thread::spawn(move || {
        let mut reader = BufReader::new(reader);
        loop {
            match read_dap_message(&mut reader) {
                Ok(Some(request)) => {
                    if input_tx.send(Ok(request)).is_err() {
                        break;
                    }
                }
                Ok(None) => break,
                Err(error) => {
                    let _ = input_tx.send(Err(error));
                    break;
                }
            }
        }
    });

    let (host_event_tx, host_event_rx) = mpsc::channel();
    let mut host = CliDapHost::new(host_event_tx, active_abort);
    let mut session = DapSession::<u8>::new(true);
    loop {
        match input_rx.recv_timeout(TRANSPORT_POLL) {
            Ok(Ok(request)) => {
                for message in session.handle_request(request, &mut host) {
                    write_dap_message(&mut writer, &message)?;
                }
            }
            Ok(Err(error)) => {
                session.transport_lost();
                return Err(error);
            }
            Err(mpsc::RecvTimeoutError::Disconnected) => {
                session.transport_lost();
                break;
            }
            Err(mpsc::RecvTimeoutError::Timeout) => {}
        }
        while let Ok(event) = host_event_rx.try_recv() {
            match event {
                DebugHostEvent::Output { category, output } => {
                    let message = session.output_event(category, &output);
                    write_dap_message(&mut writer, &message)?;
                }
            }
        }
        for message in session.poll_events() {
            write_dap_message(&mut writer, &message)?;
        }
        if session.disconnected() {
            break;
        }
    }
    host.wait_for_detached_operations();
    Ok(())
}

pub fn read_dap_message(reader: &mut dyn BufRead) -> io::Result<Option<DapRequest>> {
    let mut content_length = None;
    let mut header_bytes = 0usize;
    loop {
        let mut line = String::new();
        let read = reader.read_line(&mut line)?;
        if read == 0 {
            return if content_length.is_none() {
                Ok(None)
            } else {
                Err(io::Error::new(
                    io::ErrorKind::UnexpectedEof,
                    "DAP header ended before the blank line",
                ))
            };
        }
        header_bytes = header_bytes.saturating_add(read);
        if header_bytes > 8 * 1024 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "DAP header exceeds 8 KiB",
            ));
        }
        if line == "\r\n" || line == "\n" {
            break;
        }
        if let Some((name, value)) = line.split_once(':') {
            if name.eq_ignore_ascii_case("Content-Length") {
                if content_length.is_some() {
                    return Err(io::Error::new(
                        io::ErrorKind::InvalidData,
                        "duplicate DAP Content-Length",
                    ));
                }
                content_length = Some(value.trim().parse::<usize>().map_err(|_| {
                    io::Error::new(io::ErrorKind::InvalidData, "invalid DAP Content-Length")
                })?);
            }
        }
    }
    let content_length = content_length.ok_or_else(|| {
        io::Error::new(io::ErrorKind::InvalidData, "DAP Content-Length is required")
    })?;
    if content_length == 0 || content_length > MAX_DAP_MESSAGE_BYTES {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("DAP body length must be 1..={MAX_DAP_MESSAGE_BYTES}"),
        ));
    }
    let mut body = vec![0; content_length];
    reader.read_exact(&mut body)?;
    let request = serde_json::from_slice::<DapRequest>(&body).map_err(|error| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            format!("invalid DAP request JSON: {error}"),
        )
    })?;
    if request.seq <= 0 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "DAP request seq must be a positive int32",
        ));
    }
    Ok(Some(request))
}

pub fn write_dap_message(writer: &mut dyn Write, message: &Value) -> io::Result<()> {
    let body = serde_json::to_vec(message).map_err(|error| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            format!("cannot serialize DAP message: {error}"),
        )
    })?;
    if body.len() > MAX_DAP_MESSAGE_BYTES {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "DAP message exceeds the disclosed transport limit",
        ));
    }
    write!(writer, "Content-Length: {}\r\n\r\n", body.len())?;
    writer.write_all(&body)?;
    writer.flush()
}

#[allow(dead_code)]
fn _assert_tcp_stream_send(_: TcpStream) {}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;
    use std::time::Duration;

    use cem_ml::operation_handle::OperationTerminalStatus;

    #[test]
    fn debug_launch_routes_existing_command_output_and_terminal_state() {
        let parsed = Cli::try_parse_from(["cem-ml", "version"]).unwrap();
        let mut expected_stdout = Vec::new();
        let mut expected_stderr = Vec::new();
        let mut streams = Streams {
            stdout: &mut expected_stdout,
            stderr: &mut expected_stderr,
            quiet: parsed.quiet,
            no_color: parsed.no_color,
            abort_signal: AbortSignal::new(),
            operation_control: None,
        };
        let expected = dispatch::dispatch(&RealCemMlEngine::new(), parsed, &mut streams);
        drop(streams);
        assert_eq!(expected.exit_code, dispatch::EXIT_OK);
        assert!(expected_stderr.is_empty());

        let (event_tx, event_rx) = mpsc::channel();
        let active_abort = Arc::new(Mutex::new(None::<AbortSignal>));
        let mut host = CliDapHost::new(event_tx, Arc::clone(&active_abort));
        let handle = host
            .launch(&serde_json::json!({ "command": "version", "args": [] }))
            .unwrap();

        assert!(handle.debug_control_active());
        assert!(active_abort.lock().unwrap().is_some());
        assert!(handle
            .blocking_result_timeout(Duration::from_millis(20))
            .unwrap()
            .is_none());
        host.configuration_done().unwrap();
        let outcome = handle
            .blocking_result_timeout(Duration::from_secs(5))
            .unwrap()
            .expect("version command must settle");
        assert_eq!(outcome.status(), OperationTerminalStatus::Succeeded);
        let DebugHostEvent::Output { category, output } = event_rx
            .recv_timeout(Duration::from_secs(5))
            .expect("version stdout must become a host event");
        assert_eq!(category, "stdout");
        assert_eq!(output.as_bytes(), expected_stdout);
    }

    #[test]
    fn dap_framing_round_trips_without_non_protocol_stdout_bytes() {
        let request = serde_json::json!({
            "seq": 7,
            "type": "request",
            "command": "initialize",
            "arguments": {}
        });
        let mut framed = Vec::new();
        write_dap_message(&mut framed, &request).unwrap();
        assert!(framed.starts_with(b"Content-Length: "));
        let parsed = read_dap_message(&mut Cursor::new(framed)).unwrap().unwrap();
        assert_eq!(parsed.seq, 7);
        assert_eq!(parsed.command, "initialize");
    }

    #[test]
    fn framing_rejects_missing_and_oversized_lengths() {
        assert!(read_dap_message(&mut Cursor::new(b"X: 1\r\n\r\n{}"))
            .unwrap_err()
            .to_string()
            .contains("Content-Length"));
        let input = format!("Content-Length: {}\r\n\r\n", MAX_DAP_MESSAGE_BYTES + 1);
        assert!(read_dap_message(&mut Cursor::new(input.into_bytes())).is_err());
        assert!(read_dap_message(&mut Cursor::new(
            b"Content-Length: 2\r\nContent-Length: 2\r\n\r\n{}",
        ))
        .unwrap_err()
        .to_string()
        .contains("duplicate"));
    }

    #[test]
    fn tcp_v1_rejects_non_loopback_endpoints_before_binding() {
        let result = serve_tcp("0.0.0.0:0", Arc::new(Mutex::new(None::<AbortSignal>)));
        assert_eq!(result.unwrap_err().kind(), io::ErrorKind::PermissionDenied);
    }
}
