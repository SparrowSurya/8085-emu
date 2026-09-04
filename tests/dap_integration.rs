use std::path::PathBuf;
use emu8085::dap::protocol::*;
use emu8085::dap::DapServer;
use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader};

struct MockDapClient {
    reader: BufReader<tokio::io::DuplexStream>,
    writer: tokio::io::DuplexStream,
    seq: i64,
}

impl MockDapClient {
    async fn send_request(&mut self, command: &str, arguments: Option<serde_json::Value>) -> (Response, Vec<Event>) {
        let req_seq = self.seq;
        self.seq += 1;
        let req = Request {
            seq: req_seq,
            command: command.to_string(),
            arguments,
        };
        let json = serde_json::to_string(&ProtocolMessage::Request(req)).unwrap();
        let payload = format!("Content-Length: {}\r\n\r\n{}", json.len(), json);
        self.writer.write_all(payload.as_bytes()).await.unwrap();
        self.writer.flush().await.unwrap();

        // Read response and any preceding or following events until response for req_seq is received
        let mut events = Vec::new();
        loop {
            let msg = self.read_message().await;
            match msg {
                ProtocolMessage::Response(resp) if resp.request_seq == req_seq => {
                    return (resp, events);
                }
                ProtocolMessage::Event(ev) => {
                    events.push(ev);
                }
                _ => {}
            }
        }
    }

    async fn read_message(&mut self) -> ProtocolMessage {
        let mut content_length: Option<usize> = None;
        loop {
            let mut line = String::new();
            self.reader.read_line(&mut line).await.unwrap();
            let trimmed = line.trim();
            if trimmed.is_empty() {
                break;
            }
            if let Some(val) = trimmed.strip_prefix("Content-Length:") {
                content_length = Some(val.trim().parse().unwrap());
            }
        }

        let len = content_length.expect("missing Content-Length");
        let mut body = vec![0u8; len];
        self.reader.read_exact(&mut body).await.unwrap();
        serde_json::from_slice(&body).expect("invalid JSON protocol message")
    }
}

#[tokio::test]
async fn test_full_dap_session_workflow() {
    let (client_stream, server_stream_client) = tokio::io::duplex(64 * 1024);
    let (server_stream_server, client_stream_server) = tokio::io::duplex(64 * 1024);

    let mut client = MockDapClient {
        reader: BufReader::new(client_stream_server),
        writer: client_stream,
        seq: 1,
    };

    // Spawn server in background task
    tokio::spawn(async move {
        let mut server = DapServer::new();
        server.run(server_stream_client, server_stream_server).await.unwrap();
    });

    // 1. Initialize
    let (init_resp, init_events) = client.send_request("initialize", Some(serde_json::json!({
        "adapterID": "e8085",
        "linesStartAt1": true,
        "columnsStartAt1": true
    }))).await;
    assert!(init_resp.success);
    assert_eq!(init_resp.command, "initialize");
    assert!(init_events.iter().any(|e| e.event == "initialized"));

    // 2. Launch demo program with stopOnEntry
    let demo_path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("programs/demo.e8085");
    let (launch_resp, launch_events) = client.send_request("launch", Some(serde_json::json!({
        "program": demo_path.to_string_lossy().to_string(),
        "stopOnEntry": true
    }))).await;
    assert!(launch_resp.success);
    assert!(launch_events.iter().any(|e| e.event == "stopped"));

    // 3. Threads
    let (threads_resp, _) = client.send_request("threads", None).await;
    assert!(threads_resp.success);
    let threads_body: ThreadsResponseBody = serde_json::from_value(threads_resp.body.unwrap()).unwrap();
    assert_eq!(threads_body.threads.len(), 1);
    assert_eq!(threads_body.threads[0].id, 1);

    // 4. Stack Trace
    let (stack_resp, _) = client.send_request("stackTrace", Some(serde_json::json!({ "threadId": 1 }))).await;
    assert!(stack_resp.success);
    let stack_body: StackTraceResponseBody = serde_json::from_value(stack_resp.body.unwrap()).unwrap();
    assert!(!stack_body.stack_frames.is_empty());
    assert_eq!(stack_body.stack_frames[0].id, 0);

    // 5. Scopes
    let (scopes_resp, _) = client.send_request("scopes", Some(serde_json::json!({ "frameId": 0 }))).await;
    assert!(scopes_resp.success);
    let scopes_body: ScopesResponseBody = serde_json::from_value(scopes_resp.body.unwrap()).unwrap();
    assert_eq!(scopes_body.scopes.len(), 5);

    // 6. Variables (CPU Registers scope)
    let (vars_resp, _) = client.send_request("variables", Some(serde_json::json!({ "variablesReference": 1000 }))).await;
    assert!(vars_resp.success);
    let vars_body: VariablesResponseBody = serde_json::from_value(vars_resp.body.unwrap()).unwrap();
    assert!(vars_body.variables.iter().any(|v| v.name == "A"));
    assert!(vars_body.variables.iter().any(|v| v.name == "PC"));

    // 7. Set Variable (live mutation of register A)
    let (set_var_resp, _) = client.send_request("setVariable", Some(serde_json::json!({
        "variablesReference": 1000,
        "name": "A",
        "value": "0x55"
    }))).await;
    assert!(set_var_resp.success);

    // 8. Evaluate expression
    let (eval_resp, _) = client.send_request("evaluate", Some(serde_json::json!({
        "expression": "A == 0x55"
    }))).await;
    assert!(eval_resp.success);
    let eval_body: EvaluateResponseBody = serde_json::from_value(eval_resp.body.unwrap()).unwrap();
    assert!(eval_body.result.contains("true"));

    // 9. Step In
    let (step_resp, step_events) = client.send_request("stepIn", Some(serde_json::json!({ "threadId": 1 }))).await;
    assert!(step_resp.success);
    assert!(step_events.iter().any(|e| e.event == "stopped"));

    // 10. Check Flags Byte (PSW) in Flags scope
    let (flags_resp, _) = client.send_request("variables", Some(serde_json::json!({ "variablesReference": 3000 }))).await;
    assert!(flags_resp.success);
    let flags_body: VariablesResponseBody = serde_json::from_value(flags_resp.body.unwrap()).unwrap();
    assert!(flags_body.variables.iter().any(|v| v.name == "Flags Byte (PSW)"));

    // 11. Set Breakpoints
    let (bp_resp, _) = client.send_request("setBreakpoints", Some(serde_json::json!({
        "source": { "path": demo_path.to_string_lossy().to_string() },
        "breakpoints": [{ "line": 26 }]
    }))).await;
    assert!(bp_resp.success);
    let bp_body: SetBreakpointsResponseBody = serde_json::from_value(bp_resp.body.unwrap()).unwrap();
    assert_eq!(bp_body.breakpoints.len(), 1);
    assert!(bp_body.breakpoints[0].verified);

    // 12. Continue until breakpoint
    let (cont_resp, cont_events) = client.send_request("continue", Some(serde_json::json!({ "threadId": 1 }))).await;
    assert!(cont_resp.success);
    assert!(cont_events.iter().any(|e| e.event == "stopped"));

    // 13. Disassemble memory
    let (disasm_resp, _) = client.send_request("disassemble", Some(serde_json::json!({
        "memoryReference": "PC",
        "instructionCount": 5
    }))).await;
    assert!(disasm_resp.success);
    let disasm_body: DisassembleResponseBody = serde_json::from_value(disasm_resp.body.unwrap()).unwrap();
    assert_eq!(disasm_body.instructions.len(), 5);

    // 14. Restart debug session
    let (restart_resp, restart_events) = client.send_request("restart", None).await;
    assert!(restart_resp.success);
    assert!(restart_events.iter().any(|e| e.event == "stopped"));

    // 15. Disconnect / Terminate
    let (disc_resp, disc_events) = client.send_request("disconnect", None).await;
    assert!(disc_resp.success);
    assert!(disc_events.iter().any(|e| e.event == "terminated"));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_multifile_terminal_bridge_and_halt_termination() {
    // 1. Start mock TCP terminal server simulating VS Code "8085 Terminal" on 127.0.0.1:18085
    let listener = tokio::net::TcpListener::bind("127.0.0.1:18085").await.unwrap();

    let (client_stream, server_stream_client) = tokio::io::duplex(64 * 1024);
    let (server_stream_server, client_stream_server) = tokio::io::duplex(64 * 1024);

    let mut client = MockDapClient {
        reader: BufReader::new(client_stream_server),
        writer: client_stream,
        seq: 1,
    };

    tokio::spawn(async move {
        let mut server = DapServer::new();
        server.run(server_stream_client, server_stream_server).await.unwrap();
    });

    // 2. Initialize
    let (init_resp, _) = client.send_request("initialize", Some(serde_json::json!({
        "adapterID": "e8085",
        "linesStartAt1": true,
        "columnsStartAt1": true
    }))).await;
    assert!(init_resp.success);

    // 3. Launch triangle_pattern pointing to terminalPort 18085
    let triangle_path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("programs/triangle_pattern.e8085");
    let (launch_resp, launch_events) = client.send_request("launch", Some(serde_json::json!({
        "program": triangle_path.to_string_lossy().to_string(),
        "stopOnEntry": true,
        "console": "integratedTerminal",
        "terminalPort": 18085
    }))).await;
    assert!(launch_resp.success);
    assert!(launch_events.iter().any(|e| e.event == "stopped"));

    // Accept TCP connection from DAP session
    let (mut terminal_socket, _) = listener.accept().await.unwrap();

    // Spawn a background task to record all bytes received by the terminal socket and feed "3\n" after prompt
    let received_output = std::sync::Arc::new(std::sync::Mutex::new(Vec::<u8>::new()));
    let received_clone = received_output.clone();
    tokio::spawn(async move {
        use tokio::io::{AsyncReadExt, AsyncWriteExt};
        let mut buf = [0u8; 128];
        let mut input_sent = false;
        loop {
            match terminal_socket.read(&mut buf).await {
                Ok(0) => break,
                Ok(n) => {
                    {
                        let mut rec = received_clone.lock().unwrap();
                        rec.extend_from_slice(&buf[..n]);
                    }
                    // When prompt is received, simulate user typing "3\n"
                    if !input_sent {
                        let current_text = {
                            let rec = received_clone.lock().unwrap();
                            String::from_utf8_lossy(&rec).to_string()
                        };
                        if current_text.contains("Enter size of triangle: ") {
                            input_sent = true;
                            tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
                            let _ = terminal_socket.write_all(b"3\n").await;
                            let _ = terminal_socket.flush().await;
                        }
                    }
                }
                Err(_) => break,
            }
        }
    });

    // 4. Verify entry stack frame is in triangle_pattern.e8085
    let (stack_resp, _) = client.send_request("stackTrace", Some(serde_json::json!({ "threadId": 1 }))).await;
    assert!(stack_resp.success);
    let stack_body: StackTraceResponseBody = serde_json::from_value(stack_resp.body.unwrap()).unwrap();
    let top_frame = &stack_body.stack_frames[0];
    let top_path = top_frame.source.as_ref().unwrap().path.as_ref().unwrap();
    assert!(top_path.ends_with("triangle_pattern.e8085"));
    assert_eq!(top_frame.line, 17); // lxi HL, askSize

    // 5. Step over until line 19: call print
    client.send_request("next", Some(serde_json::json!({ "threadId": 1 }))).await;
    let (step_resp, _) = client.send_request("next", Some(serde_json::json!({ "threadId": 1 }))).await;
    assert!(step_resp.success);

    // 6. Multi-File Source Navigation: Step In to `print`
    let (step_in_resp, _) = client.send_request("stepIn", Some(serde_json::json!({ "threadId": 1 }))).await;
    assert!(step_in_resp.success);

    let (stack_resp, _) = client.send_request("stackTrace", Some(serde_json::json!({ "threadId": 1 }))).await;
    let stack_body: StackTraceResponseBody = serde_json::from_value(stack_resp.body.unwrap()).unwrap();
    let stepped_path = stack_body.stack_frames[0].source.as_ref().unwrap().path.as_ref().unwrap();
    assert!(stepped_path.ends_with("terminal.e8085"), "Expected path to end with terminal.e8085, got: {stepped_path}");

    // 7. Set breakpoint at line 40 of triangle_pattern.e8085 (call draw)
    let (bp_resp, _) = client.send_request("setBreakpoints", Some(serde_json::json!({
        "source": { "path": triangle_path.to_string_lossy().to_string() },
        "breakpoints": [{ "line": 40 }]
    }))).await;
    assert!(bp_resp.success);

    // 8. Continue to breakpoint at line 40 (which executes print and input using user-supplied terminal input "3\n")
    let (cont_resp, cont_events) = client.send_request("continue", Some(serde_json::json!({ "threadId": 1 }))).await;
    assert!(cont_resp.success);
    assert!(cont_events.iter().any(|e| e.event == "stopped"));

    // 9. Verify prompt was received across the TCP terminal socket
    {
        let rec = received_output.lock().unwrap();
        let s = String::from_utf8_lossy(&rec);
        assert!(s.contains("Enter size of triangle: "), "Expected prompt on terminal, got: {s}");
    }

    // 10. Verify we are stopped at line 40
    let (stack_resp, _) = client.send_request("stackTrace", Some(serde_json::json!({ "threadId": 1 }))).await;
    let stack_body: StackTraceResponseBody = serde_json::from_value(stack_resp.body.unwrap()).unwrap();
    assert_eq!(stack_body.stack_frames[0].line, 40);

    // 11. Continue past `draw` to HLT
    let (cont_hlt_resp, hlt_events) = client.send_request("continue", Some(serde_json::json!({ "threadId": 1 }))).await;
    assert!(cont_hlt_resp.success);
    assert!(hlt_events.iter().any(|e| e.event == "stopped"));

    // 12. Verify pattern was drawn to the terminal socket
    {
        let rec = received_output.lock().unwrap();
        let s = String::from_utf8_lossy(&rec);
        assert!(s.contains("***\n**\n*\n"), "Expected triangle pattern on terminal, got: {s}");
    }

    // 13. Check Flags Byte (PSW) in Flags scope
    let (flags_resp, _) = client.send_request("variables", Some(serde_json::json!({ "variablesReference": 3000 }))).await;
    assert!(flags_resp.success);
    let flags_body: VariablesResponseBody = serde_json::from_value(flags_resp.body.unwrap()).unwrap();
    assert!(flags_body.variables.iter().any(|v| v.name == "Flags Byte (PSW)"));

    // 14. Continue after HLT -> Debugger should cleanly terminate session!
    let (cont_after_hlt_resp, after_hlt_events) = client.send_request("continue", Some(serde_json::json!({ "threadId": 1 }))).await;
    assert!(cont_after_hlt_resp.success);
    assert!(after_hlt_events.iter().any(|e| e.event == "terminated"));
    assert!(after_hlt_events.iter().any(|e| e.event == "exited"));
}

