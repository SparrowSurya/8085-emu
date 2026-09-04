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
    assert_eq!(scopes_body.scopes.len(), 7);

    // 6. Variables (CPU Registers scope)
    let (vars_resp, _) = client.send_request("variables", Some(serde_json::json!({ "variablesReference": 1000 }))).await;
    assert!(vars_resp.success);
    let vars_body: VariablesResponseBody = serde_json::from_value(vars_resp.body.unwrap()).unwrap();
    assert!(vars_body.variables.iter().any(|v| v.name == "A"));
    assert!(vars_body.variables.iter().any(|v| v.name == "PC"));

    // Check Data Segment scope (4000)
    let (data_resp, _) = client.send_request("variables", Some(serde_json::json!({ "variablesReference": 4000 }))).await;
    assert!(data_resp.success);
    let data_body: VariablesResponseBody = serde_json::from_value(data_resp.body.unwrap()).unwrap();
    assert!(data_body.variables.iter().any(|v| v.name.starts_with("prompt 0x") && v.name.ends_with("(20B)")));

    // Check BSS Segment scope (5000)
    let (bss_resp, _) = client.send_request("variables", Some(serde_json::json!({ "variablesReference": 5000 }))).await;
    assert!(bss_resp.success);

    // Check Stack scope (6000)
    let (stack_scope_resp, _) = client.send_request("variables", Some(serde_json::json!({ "variablesReference": 6000 }))).await;
    assert!(stack_scope_resp.success);

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
    assert_eq!(top_frame.line, 19); // lxi HL, askSize

    // 5. Step over until line 21: call print
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

    // 7. Set breakpoint at line 42 of triangle_pattern.e8085 (call draw)
    let (bp_resp, _) = client.send_request("setBreakpoints", Some(serde_json::json!({
        "source": { "path": triangle_path.to_string_lossy().to_string() },
        "breakpoints": [{ "line": 42 }]
    }))).await;
    assert!(bp_resp.success);

    // 8. Continue to breakpoint at line 42 (which executes print and input using user-supplied terminal input "3\n")
    let (cont_resp, cont_events) = client.send_request("continue", Some(serde_json::json!({ "threadId": 1 }))).await;
    assert!(cont_resp.success);
    assert!(cont_events.iter().any(|e| e.event == "stopped"));

    // 9. Verify prompt was received across the TCP terminal socket
    {
        let rec = received_output.lock().unwrap();
        let s = String::from_utf8_lossy(&rec);
        assert!(s.contains("Enter size of triangle: "), "Expected prompt on terminal, got: {s}");
    }

    // 10. Verify we are stopped at line 42
    let (stack_resp, _) = client.send_request("stackTrace", Some(serde_json::json!({ "threadId": 1 }))).await;
    let stack_body: StackTraceResponseBody = serde_json::from_value(stack_resp.body.unwrap()).unwrap();
    assert_eq!(stack_body.stack_frames[0].line, 42);

    // 11. Continue past `draw` to HLT -> Debugger automatically stops and terminates on HLT
    let (cont_hlt_resp, hlt_events) = client.send_request("continue", Some(serde_json::json!({ "threadId": 1 }))).await;
    assert!(cont_hlt_resp.success);
    assert!(hlt_events.iter().any(|e| e.event == "terminated"), "Expected terminated event on HLT, got: {hlt_events:?}");
    assert!(hlt_events.iter().any(|e| e.event == "exited"), "Expected exited event on HLT, got: {hlt_events:?}");

    // 12. Verify pattern was drawn to the terminal socket
    {
        let rec = received_output.lock().unwrap();
        let s = String::from_utf8_lossy(&rec);
        assert!(s.contains("***\n**\n*\n"), "Expected triangle pattern on terminal, got: {s}");
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_stack_word_and_parent_child_label_formatting() {
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

    client.send_request("initialize", Some(serde_json::json!({
        "adapterID": "e8085"
    }))).await;

    let demo_path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("programs/demo.e8085");
    client.send_request("launch", Some(serde_json::json!({
        "program": demo_path.to_string_lossy().to_string(),
        "stopOnEntry": true
    }))).await;

    // Check Data Segment scope (4000)
    let (data_resp, _) = client.send_request("variables", Some(serde_json::json!({ "variablesReference": 4000 }))).await;
    assert!(data_resp.success);
    let data_body: VariablesResponseBody = serde_json::from_value(data_resp.body.unwrap()).unwrap();
    let prompt_var = data_body.variables.iter().find(|v| v.name.starts_with("prompt")).unwrap();
    assert!(prompt_var.name.starts_with("prompt 0x") && prompt_var.name.ends_with("(20B)"));
    assert!(prompt_var.value.contains("What is your name? "));

    let hello_var = data_body.variables.iter().find(|v| v.name.starts_with("hello")).unwrap();
    assert!(hello_var.name.starts_with("hello 0x") && hello_var.name.ends_with("(3B)"));
    assert_eq!(hello_var.value, "\"Hi \"");

    // Check BSS Segment scope (5000)
    let (bss_resp, _) = client.send_request("variables", Some(serde_json::json!({ "variablesReference": 5000 }))).await;
    assert!(bss_resp.success);
    let bss_body: VariablesResponseBody = serde_json::from_value(bss_resp.body.unwrap()).unwrap();
    let name_buf_var = bss_body.variables.iter().find(|v| v.name.starts_with("name_buf")).unwrap();
    assert!(name_buf_var.name.starts_with("name_buf 0x") && name_buf_var.name.ends_with("(64B)"));
    assert!(name_buf_var.value.starts_with("\"\\x00\\x00"));

    // Check Register Pairs scope (2000)
    let (pairs_resp, _) = client.send_request("variables", Some(serde_json::json!({ "variablesReference": 2000 }))).await;
    assert!(pairs_resp.success);
    let pairs_body: VariablesResponseBody = serde_json::from_value(pairs_resp.body.unwrap()).unwrap();
    let bc_var = pairs_body.variables.iter().find(|v| v.name == "BC").unwrap();
    assert!(bc_var.value.contains("("));

    // Step 5 times to execute up to and including `lxi HL, prompt` (line 26 of demo.e8085)
    for _ in 0..5 {
        client.send_request("stepIn", Some(serde_json::json!({ "threadId": 1 }))).await;
    }
    let (pairs_resp2, _) = client.send_request("variables", Some(serde_json::json!({ "variablesReference": 2000 }))).await;
    let pairs_body2: VariablesResponseBody = serde_json::from_value(pairs_resp2.body.unwrap()).unwrap();
    let hl_var = pairs_body2.variables.iter().find(|v| v.name == "HL").unwrap();
    assert!(hl_var.value.contains("(prompt)"), "Expected HL value to contain (prompt), got: {}", hl_var.value);

    // Check Stack scope (6000)
    let (stack_resp, _) = client.send_request("variables", Some(serde_json::json!({ "variablesReference": 6000 }))).await;
    assert!(stack_resp.success);
    let stack_body: VariablesResponseBody = serde_json::from_value(stack_resp.body.unwrap()).unwrap();
    assert!(!stack_body.variables.is_empty());
}


