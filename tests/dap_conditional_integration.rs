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
async fn test_dap_conditional_compilation_stepping_and_breakpoints() {
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

    // 1. Initialize
    let (init_resp, init_events) = client.send_request("initialize", Some(serde_json::json!({
        "adapterID": "e8085",
        "linesStartAt1": true,
        "columnsStartAt1": true
    }))).await;
    assert!(init_resp.success);
    assert!(init_events.iter().any(|e| e.event == "initialized"));

    // 2. Launch program with stopOnEntry
    let prog_path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("programs/conditional.e8085");
    let (launch_resp, launch_events) = client.send_request("launch", Some(serde_json::json!({
        "program": prog_path.to_string_lossy().to_string(),
        "stopOnEntry": true
    }))).await;
    assert!(launch_resp.success);
    assert!(launch_events.iter().any(|e| e.event == "stopped"));

    // 3. Set breakpoints: active line 39 (hlt), and inactive line 31 (%else banner)
    let (bp_resp, _) = client.send_request("setBreakpoints", Some(serde_json::json!({
        "source": { "path": prog_path.to_string_lossy().to_string() },
        "breakpoints": [
            { "line": 39 }, // hlt
            { "line": 31 }  // inactive banner in %else branch
        ]
    }))).await;
    assert!(bp_resp.success);
    let bp_body: SetBreakpointsResponseBody = serde_json::from_value(bp_resp.body.unwrap()).unwrap();
    assert_eq!(bp_body.breakpoints.len(), 2);
    // Line 39 is executable (hlt)
    assert!(bp_body.breakpoints[0].verified);
    assert_eq!(bp_body.breakpoints[0].line, Some(39));
    // Line 31 is in inactive data block, snaps forward to next data line 33 (ready_msg)
    assert!(bp_body.breakpoints[1].verified);
    assert_eq!(bp_body.breakpoints[1].line, Some(33));

    // 4. Check initial stack frame (at entry point: line 37)
    let (stack_resp, _) = client.send_request("stackTrace", Some(serde_json::json!({ "threadId": 1 }))).await;
    assert!(stack_resp.success);
    let stack_body: StackTraceResponseBody = serde_json::from_value(stack_resp.body.unwrap()).unwrap();
    assert!(!stack_body.stack_frames.is_empty());
    assert_eq!(stack_body.stack_frames[0].line, 37);

    // 5. Continue to the next breakpoint (hlt at line 39)
    let (cont_resp, cont_events) = client.send_request("continue", Some(serde_json::json!({ "threadId": 1 }))).await;
    assert!(cont_resp.success);
    assert!(cont_events.iter().any(|e| e.event == "stopped"));

    // Check that we stopped at line 39
    let (stack_resp2, _) = client.send_request("stackTrace", Some(serde_json::json!({ "threadId": 1 }))).await;
    let stack_body2: StackTraceResponseBody = serde_json::from_value(stack_resp2.body.unwrap()).unwrap();
    assert_eq!(stack_body2.stack_frames[0].line, 39);

    // 6. Evaluate expression: test comparison and arithmetic in watch/repl
    let (eval_resp, _) = client.send_request("evaluate", Some(serde_json::json!({
        "expression": "A == A",
        "frameId": 0
    }))).await;
    assert!(eval_resp.success);
    let eval_body: EvaluateResponseBody = serde_json::from_value(eval_resp.body.unwrap()).unwrap();
    assert_eq!(eval_body.result, "true (1)");
}
