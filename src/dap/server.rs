use std::io;
use std::path::PathBuf;
use tokio::io::{AsyncBufReadExt, AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt, BufReader};

use crate::dap::protocol::*;
use crate::dap::session::{DebugSession, StopReason};

pub struct DapServer {
    session: DebugSession,
    seq: i64,
    last_launch_args: Option<LaunchRequestArguments>,
}

impl Default for DapServer {
    fn default() -> Self {
        Self::new()
    }
}

impl DapServer {
    pub fn new() -> Self {
        Self {
            session: DebugSession::new(),
            seq: 1,
            last_launch_args: None,
        }
    }

    pub async fn run_stdio() -> io::Result<()> {
        let stdin = tokio::io::stdin();
        let stdout = tokio::io::stdout();
        let mut server = DapServer::new();
        server.run(stdin, stdout).await
    }

    pub async fn run<R, W>(&mut self, reader: R, mut writer: W) -> io::Result<()>
    where
        R: AsyncRead + Unpin,
        W: AsyncWrite + Unpin,
    {
        let mut reader = BufReader::new(reader);

        loop {
            // 1. Read headers
            let mut content_length: Option<usize> = None;
            loop {
                let mut line = String::new();
                let bytes_read = reader.read_line(&mut line).await?;
                if bytes_read == 0 {
                    return Ok(()); // EOF
                }
                let trimmed = line.trim();
                if trimmed.is_empty() {
                    break; // Header boundary
                }
                if let Some(val) = trimmed.strip_prefix("Content-Length:") {
                    if let Ok(len) = val.trim().parse::<usize>() {
                        content_length = Some(len);
                    }
                }
            }

            let len = match content_length {
                Some(l) => l,
                None => continue,
            };

            // 2. Read body
            let mut body_bytes = vec![0u8; len];
            reader.read_exact(&mut body_bytes).await?;

            let raw_str = String::from_utf8_lossy(&body_bytes);
            let req: Request = match serde_json::from_str(&raw_str) {
                Ok(r) => r,
                Err(e) => {
                    eprintln!("failed to parse DAP request: {e}");
                    continue;
                }
            };

            // 3. Handle request
            let (resp, events) = self.handle_request(req);

            // 4. Send any generated events
            for ev in events {
                self.send_message(&mut writer, &ProtocolMessage::Event(ev)).await?;
            }

            // 5. Send response
            self.send_message(&mut writer, &ProtocolMessage::Response(resp)).await?;
        }
    }

    fn handle_request(&mut self, req: Request) -> (Response, Vec<Event>) {
        let mut events = Vec::new();
        let command = req.command.clone();
        let req_seq = req.seq;

        let response = match command.as_str() {
            "initialize" => {
                let caps = Capabilities {
                    supports_configuration_done_request: Some(true),
                    supports_function_breakpoints: Some(true),
                    supports_conditional_breakpoints: Some(true),
                    supports_hit_conditional_breakpoints: Some(true),
                    supports_evaluate_for_hovers: Some(true),
                    supports_step_back: Some(false),
                    supports_set_variable: Some(true),
                    supports_restart_request: Some(true),
                    supports_disassemble_request: Some(true),
                    supports_read_memory_request: Some(true),
                    supports_write_memory_request: Some(true),
                    supports_terminate_request: Some(true),
                    supports_instruction_breakpoints: Some(false),
                };
                events.push(self.create_event("initialized", None));
                self.create_response(req_seq, &command, true, None, Some(serde_json::to_value(caps).unwrap()))
            }
            "launch" => {
                let args: LaunchRequestArguments = req.arguments.and_then(|v| serde_json::from_value(v).ok()).unwrap_or(LaunchRequestArguments {
                    program: "".to_string(),
                    stop_on_entry: Some(true),
                    libraries: None,
                    t_state_limit: None,
                    step_mode: None,
                    console: None,
                    terminal_port: None,
                });
                self.last_launch_args = Some(args.clone());

                match self.session.launch(&args) {
                    Ok(reason) => {
                        if let Some(r) = reason {
                            self.handle_stop_reason(r, &mut events);
                        }
                        self.create_response(req_seq, &command, true, None, None)
                    }
                    Err(err) => self.create_response(req_seq, &command, false, Some(err), None),
                }
            }
            "restart" => {
                let restart_args: Option<RestartArguments> = req.arguments.and_then(|v| serde_json::from_value(v).ok());
                let launch_args = restart_args
                    .and_then(|r| r.arguments)
                    .or_else(|| self.last_launch_args.clone())
                    .unwrap_or(LaunchRequestArguments {
                        program: "".to_string(),
                        stop_on_entry: Some(true),
                        libraries: None,
                        t_state_limit: None,
                        step_mode: None,
                        console: None,
                        terminal_port: None,
                    });
                self.last_launch_args = Some(launch_args.clone());

                match self.session.restart(&launch_args) {
                    Ok(reason) => {
                        if let Some(r) = reason {
                            self.handle_stop_reason(r, &mut events);
                        }
                        self.create_response(req_seq, &command, true, None, None)
                    }
                    Err(err) => self.create_response(req_seq, &command, false, Some(err), None),
                }
            }
            "setBreakpoints" => {
                let args: SetBreakpointsArguments = req.arguments.and_then(|v| serde_json::from_value(v).ok()).unwrap_or(SetBreakpointsArguments {
                    source: Source { name: None, path: None },
                    breakpoints: None,
                    lines: None,
                });

                let file_path = PathBuf::from(args.source.path.as_deref().unwrap_or(""));
                let req_bps = args.breakpoints.unwrap_or_else(|| {
                    args.lines.unwrap_or_default().into_iter().map(|l| SourceBreakpoint {
                        line: l,
                        column: None,
                        condition: None,
                        hit_condition: None,
                        log_message: None,
                    }).collect()
                });

                let verified = self.session.set_line_breakpoints(&file_path, &req_bps);
                let body = SetBreakpointsResponseBody { breakpoints: verified };
                self.create_response(req_seq, &command, true, None, Some(serde_json::to_value(body).unwrap()))
            }
            "setFunctionBreakpoints" => {
                let args: SetFunctionBreakpointsArguments = req.arguments.and_then(|v| serde_json::from_value(v).ok()).unwrap_or(SetFunctionBreakpointsArguments {
                    breakpoints: Vec::new(),
                });
                let verified = self.session.set_function_breakpoints(&args.breakpoints);
                let body = SetFunctionBreakpointsResponseBody { breakpoints: verified };
                self.create_response(req_seq, &command, true, None, Some(serde_json::to_value(body).unwrap()))
            }
            "configurationDone" => {
                self.create_response(req_seq, &command, true, None, None)
            }
            "threads" => {
                let body = ThreadsResponseBody {
                    threads: vec![Thread {
                        id: 1,
                        name: "8085 Microprocessor Thread".to_string(),
                    }],
                };
                self.create_response(req_seq, &command, true, None, Some(serde_json::to_value(body).unwrap()))
            }
            "stackTrace" => {
                let frames = self.session.get_stack_trace();
                let body = StackTraceResponseBody {
                    total_frames: Some(frames.len()),
                    stack_frames: frames,
                };
                self.create_response(req_seq, &command, true, None, Some(serde_json::to_value(body).unwrap()))
            }
            "scopes" => {
                let args: ScopesArguments = req.arguments.and_then(|v| serde_json::from_value(v).ok()).unwrap_or(ScopesArguments { frame_id: 0 });
                let scopes = self.session.get_scopes(args.frame_id);
                let body = ScopesResponseBody { scopes };
                self.create_response(req_seq, &command, true, None, Some(serde_json::to_value(body).unwrap()))
            }
            "variables" => {
                let args: VariablesArguments = req.arguments.and_then(|v| serde_json::from_value(v).ok()).unwrap_or(VariablesArguments {
                    variables_reference: 1000,
                    start: None,
                    count: None,
                });
                let variables = self.session.get_variables(args.variables_reference);
                let body = VariablesResponseBody { variables };
                self.create_response(req_seq, &command, true, None, Some(serde_json::to_value(body).unwrap()))
            }
            "setVariable" => {
                let args: SetVariableArguments = req.arguments.and_then(|v| serde_json::from_value(v).ok()).unwrap_or(SetVariableArguments {
                    variables_reference: 1000,
                    name: "".to_string(),
                    value: "".to_string(),
                });
                match self.session.set_variable(args.variables_reference, &args.name, &args.value) {
                    Ok((val, typ)) => {
                        let body = SetVariableResponseBody {
                            value: val,
                            r#type: Some(typ),
                        };
                        self.create_response(req_seq, &command, true, None, Some(serde_json::to_value(body).unwrap()))
                    }
                    Err(err) => self.create_response(req_seq, &command, false, Some(err), None),
                }
            }
            "continue" => {
                if self.session.is_terminated || self.session.machine.cpu.is_halt {
                    events.push(self.create_event("exited", Some(serde_json::to_value(ExitedEventBody { exit_code: 0 }).unwrap())));
                    events.push(self.create_event("terminated", None));
                    let body = ContinueResponseBody { all_threads_continued: Some(true) };
                    self.create_response(req_seq, &command, true, None, Some(serde_json::to_value(body).unwrap()))
                } else {
                    events.push(self.create_event("continued", Some(serde_json::to_value(ContinuedEventBody { thread_id: 1, all_threads_continued: Some(true) }).unwrap())));
                    match self.session.continue_exec() {
                        Ok(reason) => {
                            if let Some(r) = reason {
                                self.handle_stop_reason(r, &mut events);
                            } else if self.session.is_terminated {
                                events.push(self.create_event("exited", Some(serde_json::to_value(ExitedEventBody { exit_code: 0 }).unwrap())));
                                events.push(self.create_event("terminated", None));
                            }
                            let body = ContinueResponseBody { all_threads_continued: Some(true) };
                            self.create_response(req_seq, &command, true, None, Some(serde_json::to_value(body).unwrap()))
                        }
                        Err(err) => self.create_response(req_seq, &command, false, Some(err), None),
                    }
                }
            }
            "next" => {
                if self.session.is_terminated || self.session.machine.cpu.is_halt {
                    events.push(self.create_event("exited", Some(serde_json::to_value(ExitedEventBody { exit_code: 0 }).unwrap())));
                    events.push(self.create_event("terminated", None));
                    self.create_response(req_seq, &command, true, None, None)
                } else {
                    match self.session.step_over() {
                        Ok(reason) => {
                            if let Some(r) = reason {
                                self.handle_stop_reason(r, &mut events);
                            } else if self.session.is_terminated {
                                events.push(self.create_event("exited", Some(serde_json::to_value(ExitedEventBody { exit_code: 0 }).unwrap())));
                                events.push(self.create_event("terminated", None));
                            }
                            self.create_response(req_seq, &command, true, None, None)
                        }
                        Err(err) => self.create_response(req_seq, &command, false, Some(err), None),
                    }
                }
            }
            "stepIn" => {
                if self.session.is_terminated || self.session.machine.cpu.is_halt {
                    events.push(self.create_event("exited", Some(serde_json::to_value(ExitedEventBody { exit_code: 0 }).unwrap())));
                    events.push(self.create_event("terminated", None));
                    self.create_response(req_seq, &command, true, None, None)
                } else {
                    match self.session.step_in() {
                        Ok(reason) => {
                            if let Some(r) = reason {
                                self.handle_stop_reason(r, &mut events);
                            } else if self.session.is_terminated {
                                events.push(self.create_event("exited", Some(serde_json::to_value(ExitedEventBody { exit_code: 0 }).unwrap())));
                                events.push(self.create_event("terminated", None));
                            }
                            self.create_response(req_seq, &command, true, None, None)
                        }
                        Err(err) => self.create_response(req_seq, &command, false, Some(err), None),
                    }
                }
            }
            "stepOut" => {
                if self.session.is_terminated || self.session.machine.cpu.is_halt {
                    events.push(self.create_event("exited", Some(serde_json::to_value(ExitedEventBody { exit_code: 0 }).unwrap())));
                    events.push(self.create_event("terminated", None));
                    self.create_response(req_seq, &command, true, None, None)
                } else {
                    match self.session.step_out() {
                        Ok(reason) => {
                            if let Some(r) = reason {
                                self.handle_stop_reason(r, &mut events);
                            } else if self.session.is_terminated {
                                events.push(self.create_event("exited", Some(serde_json::to_value(ExitedEventBody { exit_code: 0 }).unwrap())));
                                events.push(self.create_event("terminated", None));
                            }
                            self.create_response(req_seq, &command, true, None, None)
                        }
                        Err(err) => self.create_response(req_seq, &command, false, Some(err), None),
                    }
                }
            }
            "stepBack" | "reverseContinue" => {
                self.create_response(req_seq, &command, false, Some("reverse execution is disabled".to_string()), None)
            }
            "pause" => {
                match self.session.pause() {
                    Ok(reason) => {
                        if let Some(r) = reason {
                            self.handle_stop_reason(r, &mut events);
                        }
                        self.create_response(req_seq, &command, true, None, None)
                    }
                    Err(err) => self.create_response(req_seq, &command, false, Some(err), None),
                }
            }
            "evaluate" => {
                let args: EvaluateArguments = req.arguments.and_then(|v| serde_json::from_value(v).ok()).unwrap_or(EvaluateArguments {
                    expression: "".to_string(),
                    frame_id: None,
                    context: None,
                });
                match self.session.evaluate(&args.expression) {
                    Ok(res) => {
                        let body = EvaluateResponseBody {
                            result: res.display,
                            r#type: Some(res.r#type),
                            variables_reference: 0,
                            memory_reference: None,
                        };
                        self.create_response(req_seq, &command, true, None, Some(serde_json::to_value(body).unwrap()))
                    }
                    Err(err) => self.create_response(req_seq, &command, false, Some(err), None),
                }
            }
            "disassemble" => {
                let args: DisassembleArguments = req.arguments.and_then(|v| serde_json::from_value(v).ok()).unwrap_or(DisassembleArguments {
                    memory_reference: "0x0000".to_string(),
                    offset: None,
                    instruction_offset: None,
                    instruction_count: 10,
                    resolve_symbols: Some(true),
                });
                let start_addr = crate::dap::eval::eval_term(&args.memory_reference, &self.session.machine, Some(&self.session.source_map)).unwrap_or(0);
                let instructions = self.session.disassemble_memory(start_addr, args.instruction_count);
                let body = DisassembleResponseBody { instructions };
                self.create_response(req_seq, &command, true, None, Some(serde_json::to_value(body).unwrap()))
            }
            "disconnect" | "terminate" => {
                self.session.is_running = false;
                self.session.is_terminated = true;
                events.push(self.create_event("exited", Some(serde_json::to_value(ExitedEventBody { exit_code: 0 }).unwrap())));
                events.push(self.create_event("terminated", None));
                self.create_response(req_seq, &command, true, None, None)
            }
            "cancel" => {
                self.create_response(req_seq, &command, true, None, None)
            }
            _ => {
                self.create_response(req_seq, &command, false, Some(format!("unsupported DAP command '{command}'")), None)
            }
        };

        // Drain any stdout/device output events generated during execution
        for out_text in self.session.drain_output_events() {
            events.push(self.create_event("output", Some(serde_json::to_value(OutputEventBody {
                category: Some("stdout".to_string()),
                output: out_text,
            }).unwrap())));
        }

        (response, events)
    }

    async fn send_message<W: AsyncWrite + Unpin>(&mut self, writer: &mut W, msg: &ProtocolMessage) -> io::Result<()> {
        let json = serde_json::to_string(msg).unwrap();
        let payload = format!("Content-Length: {}\r\n\r\n{}", json.len(), json);
        writer.write_all(payload.as_bytes()).await?;
        writer.flush().await
    }

    fn create_response(&mut self, req_seq: i64, command: &str, success: bool, message: Option<String>, body: Option<serde_json::Value>) -> Response {
        let seq = self.seq;
        self.seq += 1;
        Response {
            seq,
            request_seq: req_seq,
            success,
            command: command.to_string(),
            message,
            body,
        }
    }

    fn create_event(&mut self, event: &str, body: Option<serde_json::Value>) -> Event {
        let seq = self.seq;
        self.seq += 1;
        Event {
            seq,
            event: event.to_string(),
            body,
        }
    }

    fn handle_stop_reason(&mut self, reason: StopReason, events: &mut Vec<Event>) {
        if matches!(reason, StopReason::Halt) {
            events.push(self.create_event("exited", Some(serde_json::to_value(ExitedEventBody { exit_code: 0 }).unwrap())));
            events.push(self.create_event("terminated", None));
        } else {
            events.push(self.stop_reason_to_event(reason));
        }
    }

    fn stop_reason_to_event(&mut self, reason: StopReason) -> Event {
        let body = match reason {
            StopReason::Entry => StoppedEventBody {
                reason: "entry".to_string(),
                description: Some("Paused at entry point".to_string()),
                thread_id: Some(1),
                text: None,
                all_threads_stopped: Some(true),
                preserve_focus_hint: Some(false),
            },
            StopReason::Step => StoppedEventBody {
                reason: "step".to_string(),
                description: Some("Step complete".to_string()),
                thread_id: Some(1),
                text: None,
                all_threads_stopped: Some(true),
                preserve_focus_hint: Some(false),
            },
            StopReason::Breakpoint(id) => StoppedEventBody {
                reason: "breakpoint".to_string(),
                description: Some(format!("Hit breakpoint {id}")),
                thread_id: Some(1),
                text: None,
                all_threads_stopped: Some(true),
                preserve_focus_hint: Some(false),
            },
            StopReason::Pause => StoppedEventBody {
                reason: "pause".to_string(),
                description: Some("Paused by user".to_string()),
                thread_id: Some(1),
                text: None,
                all_threads_stopped: Some(true),
                preserve_focus_hint: Some(false),
            },
            StopReason::Exception(e) => StoppedEventBody {
                reason: "exception".to_string(),
                description: Some(e),
                thread_id: Some(1),
                text: None,
                all_threads_stopped: Some(true),
                preserve_focus_hint: Some(false),
            },
            StopReason::Halt => StoppedEventBody {
                reason: "pause".to_string(),
                description: Some("CPU Halted (HLT)".to_string()),
                thread_id: Some(1),
                text: None,
                all_threads_stopped: Some(true),
                preserve_focus_hint: Some(false),
            },
        };
        self.create_event("stopped", Some(serde_json::to_value(body).unwrap()))
    }
}
