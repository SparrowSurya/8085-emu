//! Core Debug Session controller, execution state machine, and time-travel engine for 8085.
//!
//! [`DebugSession`] manages the complete execution lifecycle of an Intel 8085 program under debug:
//! - Multi-file source mapping, breakpoint resolution (line, function, conditional, hit-count), and stepping.
//! - Time-travel history snapshots (up to 256 states) supporting bidirectional stepping ([`step_back`](DebugSession::step_back))
//!   and reverse execution ([`reverse_continue`](DebugSession::reverse_continue)).
//! - Shadow call stack tracking nested subroutine invocations and returns without requiring hardware stack frame pointers.
//! - Peripheral device integration (virtual text I/O controller, printer, keyboard) and optional TCP socket bridging.

use std::collections::{BTreeMap, VecDeque};
use std::fs;
use std::path::{Path, PathBuf};

use crate::asm::{assemble_full_with_file, BinaryContainer, LoadImage};
use crate::cpu::MachineCycle;
use crate::dap::breakpoints::BreakpointManager;
use crate::dap::error::DapError;
use crate::dap::eval::{evaluate_expression, EvalResult};
use crate::dap::inspect::{get_scopes, get_variables, set_variable};
use crate::dap::protocol::{
    Breakpoint, DisassembledInstruction, FunctionBreakpoint, LaunchRequestArguments, Scope,
    Source, SourceBreakpoint, StackFrame, Variable,
};
use crate::dap::sourcemap::SourceMap;
use crate::instruction::disassemble_bytes;
use crate::machine::Machine;
use crate::value::Addr;

/// Maximum number of time-travel execution snapshots retained in the rewind buffer.
const MAX_HISTORY_SNAPSHOTS: usize = 256;

/// Reason why the debugger stopped execution and yielded control to the client.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StopReason {
    /// Paused at the program entry point upon launch.
    Entry,
    /// Paused after completing a single step (step-in, step-over, step-out, or step-back).
    Step,
    /// Paused because an active breakpoint (identified by its breakpoint ID) was hit.
    Breakpoint(i64),
    /// Paused asynchronously in response to an explicit user pause request.
    Pause,
    /// Paused because an illegal memory access or illegal opcode exception/fault occurred.
    Exception(String),
    /// Program executed a `HLT` instruction and terminated.
    Halt,
}

/// A frame on the debugger's shadow call stack, tracking subroutine calls across modules.
#[derive(Debug, Clone)]
pub struct ShadowFrame {
    /// Memory address of the `CALL` or `RST` instruction that initiated the subroutine call.
    pub call_address: u16,
    /// Expected return address pushed onto the stack (address immediately following the call instruction).
    pub return_address: u16,
    /// Symbolic subroutine name resolved from source map or symbol table.
    pub subroutine_name: String,
}

/// Complete machine snapshot for time-travel debugging and reverse execution.
#[derive(Debug, Clone)]
pub struct MachineSnapshot {
    /// Full CPU register and flag state.
    pub cpu: crate::cpu::Cpu,
    /// Complete snapshot of RAM memory dynamically sized to `machine.ram.len()`.
    pub ram: Box<[u8]>,
    /// Accumulated elapsed clock cycles (T-states) up to this point in execution.
    pub elapsed_t_states: u64,
    /// Total number of machine instructions executed up to this snapshot.
    pub instruction_count: u64,
    /// Copy of the shadow call stack at this execution point.
    pub shadow_stack: Vec<ShadowFrame>,
}

/// Core debugger session controller managing the 8085 virtual machine, breakpoints,
/// source mapping, time-travel history, and DAP client interactions.
pub struct DebugSession {
    /// The underlying 8085 microprocessor virtual machine.
    pub machine: Machine,
    /// Source map correlating memory addresses to source file lines, columns, and symbols.
    pub source_map: SourceMap,
    /// Breakpoint manager maintaining verified line, function, conditional, and temporary breakpoints.
    pub breakpoints: BreakpointManager,
    /// Whether the execution loop is currently actively running instructions forward.
    pub is_running: bool,
    /// Whether the target program has executed a `HLT` instruction or reached termination.
    pub is_terminated: bool,
    /// Current stop reason if execution is paused.
    pub stop_reason: Option<StopReason>,
    /// Total elapsed T-states across the entire debugging session.
    pub elapsed_t_states: u64,
    /// Total count of instructions executed since launch.
    pub instruction_count: u64,
    /// Shadow call stack recording active call nesting hierarchy.
    pub shadow_stack: Vec<ShadowFrame>,
    /// Ring buffer of execution snapshots for time-travel stepping (`step_back`, `reverse_continue`).
    pub history: VecDeque<MachineSnapshot>,
    /// Filesystem path to the primary assembly source or binary container being debugged.
    pub program_path: PathBuf,
    /// Thread-safe queue of captured text emitted by peripheral devices (TextIO, Printer).
    pub output_queue: std::sync::Arc<std::sync::Mutex<Vec<String>>>,
}

impl std::fmt::Debug for DebugSession {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DebugSession")
            .field("pc", &format_args!("0x{:04X}", self.machine.cpu.regs.pc.0))
            .field("sp", &format_args!("0x{:04X}", self.machine.cpu.regs.sp.0))
            .field("is_running", &self.is_running)
            .field("is_terminated", &self.is_terminated)
            .field("stop_reason", &self.stop_reason)
            .field("instruction_count", &self.instruction_count)
            .finish()
    }
}

impl Default for DebugSession {
    fn default() -> Self {
        Self::new()
    }
}

impl DebugSession {
    /// Creates a new, uninitialized [`DebugSession`] with a default 64KB machine and empty state.
    pub fn new() -> Self {
        Self {
            machine: Machine::default(),
            source_map: SourceMap::default(),
            breakpoints: BreakpointManager::new(),
            is_running: false,
            is_terminated: false,
            stop_reason: None,
            elapsed_t_states: 0,
            instruction_count: 0,
            shadow_stack: Vec::new(),
            history: VecDeque::with_capacity(MAX_HISTORY_SNAPSHOTS),
            program_path: PathBuf::new(),
            output_queue: std::sync::Arc::new(std::sync::Mutex::new(Vec::new())),
        }
    }

    /// Launches a debugging session for the program specified in `args`.
    ///
    /// This method:
    /// 1. Reads and compiles `.e8085` source code (or decodes `.8085.bin` containers) and builds the AST source map.
    /// 2. Initializes CPU registers (`PC`, `SP`) and loads segments into memory.
    /// 3. Attaches virtual hardware peripherals ([`TextIODevice`](crate::device::TextIODevice), [`KeyboardDevice`](crate::device::KeyboardDevice), [`PrinterDevice`](crate::device::PrinterDevice)) with optional TCP socket bridging.
    /// 4. Records the initial entry snapshot in time-travel history.
    /// 5. Pauses on entry if `stop_on_entry` is true, or begins forward execution.
    ///
    /// # Errors
    /// Returns [`DapError::FileNotFound`] if the target path does not exist,
    /// [`DapError::Assembly`] if source compilation fails, or
    /// [`DapError::ContainerDecode`] if binary container decoding fails.
    pub fn launch(&mut self, args: &LaunchRequestArguments) -> Result<Option<StopReason>, DapError> {
        let path = PathBuf::from(&args.program);
        if !path.exists() {
            return Err(DapError::FileNotFound(path));
        }
        self.program_path = path.clone();

        // 1. Compile or load program
        let ext = path.extension().and_then(|s| s.to_str()).unwrap_or("");
        let load_image = if ext == "bin" || ext == "8085" {
            let bytes = fs::read(&path).map_err(|e| DapError::Io(format!("failed to read binary {}: {e}", path.display())))?;
            let container = BinaryContainer::decode(&bytes).map_err(|e| DapError::ContainerDecode(format!("failed to decode binary container: {e}")))?;
            let image = LoadImage {
                bytes: container.text_bytes.clone(),
                entry: container.header.entry_pc,
                sp_init: container.header.sp_init,
                text_addr: container.header.text_addr,
                text_size: container.header.text_size,
                data_addr: container.header.data_addr,
                data_size: container.header.data_size,
                bss_addr: container.header.bss_addr,
                bss_size: container.header.bss_size,
                export_symbols: container.export_symbols.clone(),
            };
            let mut sym_map = BTreeMap::new();
            for (name, addr) in &container.export_symbols {
                sym_map.insert(name.clone(), *addr);
            }
            self.source_map = SourceMap {
                main_file: path.clone(),
                addr_to_loc: Default::default(),
                loc_to_addr: Default::default(),
                symbols: sym_map.clone(),
                reverse_symbols: sym_map.iter().map(|(k, v)| (*v, k.clone())).collect(),
                variables: Vec::new(),
                entry_pc: image.entry,
            };
            image
        } else {
            let source_text = fs::read_to_string(&path)
                .map_err(|e| DapError::Io(format!("failed to read source file {}: {e}", path.display())))?;
            let base_dir = path.parent();
            let (image, symbols, listing) = assemble_full_with_file(&path, &source_text, base_dir, &[])
                .map_err(|e| DapError::Assembly(format!("assembly failed at line {}:{}: {}", e.span.line, e.span.col, e.kind)))?;

            // Build AST to get data variables metadata across all files
            let raw_prog = crate::asm::parse(crate::asm::lex(&source_text).unwrap_or_default()).ok();
            let resolved_prog = if let Some(ref p) = raw_prog {
                crate::asm::include::resolve_includes_with_sources(&path, &source_text, base_dir.unwrap_or(Path::new(".")), p)
                    .map(|r| r.program)
                    .ok()
            } else {
                None
            };
            self.source_map = SourceMap::from_assembly(path.clone(), &image, &symbols, &listing, resolved_prog.as_ref().or(raw_prog.as_ref()));
            image
        };

        // 2. Initialize Machine RAM and Registers
        self.machine = Machine::default();
        crate::asm::load(&mut self.machine, &load_image)
            .map_err(|e| DapError::MachineLoad(format!("failed to load image into machine: {e:?}")))?;

        // Optional TCP bridge for integrated terminal (defaults to port 8085)
        let (term_rx, term_tcp_writer): (std::sync::mpsc::Receiver<u8>, Option<std::sync::Arc<std::sync::Mutex<std::net::TcpStream>>>) =
            if args.console.as_deref() != Some("internalConsole") {
                let port = args.terminal_port.unwrap_or(8085);
                let mut connected_stream = None;
                for _ in 0..10 {
                    if let Ok(stream) = std::net::TcpStream::connect(("127.0.0.1", port)) {
                        let _ = stream.set_nodelay(true);
                        connected_stream = Some(stream);
                        break;
                    }
                    std::thread::sleep(std::time::Duration::from_millis(25));
                }

                if let Some(stream) = connected_stream {
                    let (tx, rx) = std::sync::mpsc::channel();
                    let stream_clone = stream.try_clone().ok();
                    let stream_writer = std::sync::Arc::new(std::sync::Mutex::new(stream));

                    if let Some(mut reader_stream) = stream_clone {
                        std::thread::spawn(move || {
                            use std::io::Read;
                            let mut buf = [0u8; 256];
                            while let Ok(n) = reader_stream.read(&mut buf) {
                                if n == 0 {
                                    break;
                                }
                                for &b in &buf[..n] {
                                    let byte_to_send = if b == b'\r' { b'\n' } else { b };
                                    if tx.send(byte_to_send).is_err() {
                                        break;
                                    }
                                }
                            }
                        });
                    }

                    (rx, Some(stream_writer))
                } else {
                    (std::sync::mpsc::channel().1, None)
                }
            } else {
                (std::sync::mpsc::channel().1, None)
            };

        // Attach standard devices with stdout capturing and TCP terminal bridge
        let out_sink_term = self.output_queue.clone();
        let tcp_writer_term = term_tcp_writer.clone();
        let term = crate::device::TextIODevice::with_io(
            0x01, // data_port
            0x02, // cmd_port
            term_rx,
            move |b| {
                if let Some(ref writer_mutex) = tcp_writer_term {
                    if let Ok(mut w) = writer_mutex.lock() {
                        use std::io::Write;
                        let _ = w.write_all(&[b]);
                        let _ = w.flush();
                    }
                }
                let mut q = out_sink_term.lock().unwrap();
                if let Some(last) = q.last_mut() {
                    last.push(b as char);
                } else {
                    q.push((b as char).to_string());
                }
            },
        );
        self.machine.attach_device(Box::new(term), &[0x01, 0x02]);

        self.machine.attach_device(Box::new(crate::device::KeyboardDevice::new()), &[0x03]);

        let out_sink_printer = self.output_queue.clone();
        let tcp_writer_prn = term_tcp_writer.clone();
        let printer = crate::device::PrinterDevice::with_callback(move |c| {
            if let Some(ref writer_mutex) = tcp_writer_prn {
                if let Ok(mut w) = writer_mutex.lock() {
                    use std::io::Write;
                    let _ = w.write_all(&[c as u8]);
                    let _ = w.flush();
                }
            }
            let mut q = out_sink_printer.lock().unwrap();
            if let Some(last) = q.last_mut() {
                last.push(c);
            } else {
                q.push(c.to_string());
            }
        });
        self.machine.attach_device(Box::new(printer), &[0x04]);

        // Start execution at entry point or bootstrap
        self.machine.cpu.is_halt = false;
        self.machine.cpu.cycle = MachineCycle::Fetch;
        self.machine.cpu.t_state = 1;
        self.machine.cpu.regs.pc = Addr(load_image.entry);
        self.machine.cpu.regs.sp = Addr(load_image.sp_init);

        self.elapsed_t_states = 0;
        self.instruction_count = 0;
        self.shadow_stack.clear();
        self.history.clear();
        self.is_terminated = false;
        self.record_snapshot();

        let stop_on_entry = args.stop_on_entry.unwrap_or(true);
        if stop_on_entry {
            self.stop_reason = Some(StopReason::Entry);
            self.is_running = false;
            Ok(Some(StopReason::Entry))
        } else {
            self.is_running = true;
            self.run_execution_loop()
        }
    }

    /// Restarts program execution from the beginning while preserving all active breakpoints.
    ///
    /// # Errors
    /// Returns [`DapError`] if reloading the target program fails.
    pub fn restart(&mut self, args: &LaunchRequestArguments) -> Result<Option<StopReason>, DapError> {
        let saved_breakpoints = self.breakpoints.clone();
        let result = self.launch(args)?;
        self.breakpoints = saved_breakpoints;
        Ok(result)
    }

    /// Sets or clears verified source-line breakpoints for a specific file.
    ///
    /// Maps source line numbers to machine instruction memory addresses, validating conditions
    /// and hit-count specifications against the active [`SourceMap`].
    pub fn set_line_breakpoints(&mut self, file: &Path, requested: &[SourceBreakpoint]) -> Vec<Breakpoint> {
        self.breakpoints.set_line_breakpoints(file, requested, &self.source_map)
    }

    /// Sets or clears verified function/subroutine breakpoints by symbol name.
    ///
    /// Resolves global and local label names to addresses using the active [`SourceMap`].
    pub fn set_function_breakpoints(&mut self, requested: &[FunctionBreakpoint]) -> Vec<Breakpoint> {
        self.breakpoints.set_function_breakpoints(requested, &self.source_map)
    }

    /// Executes a single 8085 machine instruction at the current `PC` (Step In).
    ///
    /// If the instruction is a subroutine call (`CALL`, `CZ`, etc.), execution enters the body
    /// of the called subroutine and a [`ShadowFrame`] is pushed onto the shadow call stack.
    /// Records an execution snapshot before advancing to enable time-travel step-back.
    ///
    /// # Errors
    /// Returns [`DapError`] if stepping fails.
    pub fn step_in(&mut self) -> Result<Option<StopReason>, DapError> {
        if self.is_terminated {
            return Ok(None);
        }
        self.record_snapshot();
        let step_result = self.execute_single_step()?;
        if let Some(reason) = step_result {
            self.stop_reason = Some(reason.clone());
            Ok(Some(reason))
        } else {
            self.stop_reason = Some(StopReason::Step);
            Ok(Some(StopReason::Step))
        }
    }

    /// Steps over subroutine calls and software interrupts at the current `PC` (Step Over).
    ///
    /// If the current instruction is a `CALL` or `RST`, plants a temporary breakpoint at the return
    /// address immediately following the call instruction and continues running until it returns or
    /// encounters another breakpoint. For all other instructions, behaves identically to [`step_in`](Self::step_in).
    ///
    /// # Errors
    /// Returns [`DapError`] if stepping fails.
    pub fn step_over(&mut self) -> Result<Option<StopReason>, DapError> {
        if self.is_terminated {
            return Ok(None);
        }
        let current_pc = self.machine.cpu.regs.pc.0;
        let opcode_byte = self.machine.ram.read(Addr(current_pc));

        // If CALL or RST, set temporary breakpoint after call instruction
        if is_call_instruction(opcode_byte) {
            let instr_size = opcode_size_bytes(opcode_byte);
            let next_addr = current_pc.wrapping_add(instr_size);
            self.breakpoints.set_temp_breakpoint(next_addr);
            return self.continue_exec();
        }

        self.step_in()
    }

    /// Steps out of the currently executing subroutine (Step Out).
    ///
    /// Plants a temporary breakpoint at the return address recorded in the topmost [`ShadowFrame`]
    /// and resumes execution until the subroutine returns to its caller.
    ///
    /// # Errors
    /// Returns [`DapError`] if execution fails.
    pub fn step_out(&mut self) -> Result<Option<StopReason>, DapError> {
        if self.is_terminated {
            return Ok(None);
        }
        if let Some(frame) = self.shadow_stack.last() {
            let ret_addr = frame.return_address;
            self.breakpoints.set_temp_breakpoint(ret_addr);
            self.continue_exec()
        } else {
            self.continue_exec()
        }
    }

    /// Rewinds execution by exactly one machine instruction using the time-travel history buffer (Step Back).
    ///
    /// Restores the most recently captured [`MachineSnapshot`], including all CPU registers, flags,
    /// full RAM state via bulk slice copying, elapsed T-states, and shadow call stack frames.
    ///
    /// # Errors
    /// Returns [`DapError::NoHistory`] if execution is already at the earliest recorded snapshot.
    pub fn step_back(&mut self) -> Result<Option<StopReason>, DapError> {
        if let Some(snap) = self.history.pop_back() {
            self.machine.cpu = snap.cpu;
            self.machine.ram.copy_from_slice(&snap.ram);
            self.elapsed_t_states = snap.elapsed_t_states;
            self.instruction_count = snap.instruction_count;
            self.shadow_stack = snap.shadow_stack;
            self.stop_reason = Some(StopReason::Step);
            self.is_terminated = false;
            Ok(Some(StopReason::Step))
        } else {
            Err(DapError::NoHistory)
        }
    }

    /// Replays execution backward through the history buffer until a breakpoint is hit or the beginning is reached (Reverse Continue).
    ///
    /// Continuously pops and restores snapshots. If an active breakpoint matches the restored `PC`
    /// (and its condition/hit-count is satisfied), pauses with [`StopReason::Breakpoint`].
    /// If the history buffer becomes empty, pauses at the program entry state with [`StopReason::Entry`].
    ///
    /// # Errors
    /// Returns [`DapError::NoHistory`] if no earlier history snapshots are available.
    pub fn reverse_continue(&mut self) -> Result<Option<StopReason>, DapError> {
        if self.history.is_empty() {
            return Err(DapError::NoHistory);
        }
        while let Some(snap) = self.history.pop_back() {
            self.machine.cpu = snap.cpu;
            self.machine.ram.copy_from_slice(&snap.ram);
            self.elapsed_t_states = snap.elapsed_t_states;
            self.instruction_count = snap.instruction_count;
            self.shadow_stack = snap.shadow_stack;
            self.is_terminated = false;

            let pc = self.machine.cpu.regs.pc.0;
            if let Some(hit) = self.breakpoints.check_hit(pc, &self.machine, &self.source_map) {
                self.stop_reason = Some(StopReason::Breakpoint(hit.id));
                self.is_running = false;
                return Ok(Some(StopReason::Breakpoint(hit.id)));
            }

            if self.history.is_empty() {
                self.stop_reason = Some(StopReason::Entry);
                self.is_running = false;
                return Ok(Some(StopReason::Entry));
            }
        }
        self.stop_reason = Some(StopReason::Step);
        self.is_running = false;
        Ok(Some(StopReason::Step))
    }

    /// Drains and returns all queued output strings captured from virtual peripheral devices since the last call.
    pub fn drain_output_events(&mut self) -> Vec<String> {
        let mut q = self.output_queue.lock().unwrap();
        std::mem::take(&mut *q)
    }

    /// Feeds a line of text into the attached [`TextIODevice`](crate::device::TextIODevice) input buffer.
    pub fn feed_terminal_input(&mut self, text: &str) {
        if let Some(term) = self.machine.devices.find_device_mut::<crate::device::TextIODevice>() {
            term.feed_line(text);
        }
    }

    /// Enqueues a character into the attached [`KeyboardDevice`](crate::device::KeyboardDevice) FIFO buffer.
    pub fn press_keyboard_char(&mut self, c: char) {
        if let Some(kbd) = self.machine.devices.find_device_mut::<crate::device::KeyboardDevice>() {
            let _ = kbd.press_char(c);
        }
    }

    /// Resumes forward execution (Continue).
    ///
    /// Executes instructions in a loop until a verified breakpoint is hit, the CPU halts (`HLT`),
    /// a hardware exception occurs, or a pause request is received.
    ///
    /// # Errors
    /// Returns [`DapError`] if an execution error occurs.
    pub fn continue_exec(&mut self) -> Result<Option<StopReason>, DapError> {
        if self.is_terminated {
            return Ok(None);
        }
        self.is_running = true;
        self.record_snapshot();
        if let Some(reason) = self.execute_single_step()? {
            self.stop_reason = Some(reason.clone());
            return Ok(Some(reason));
        }

        self.run_execution_loop()
    }

    /// Asynchronously pauses a currently running execution loop.
    pub fn pause(&mut self) -> Result<Option<StopReason>, DapError> {
        self.is_running = false;
        self.stop_reason = Some(StopReason::Pause);
        Ok(Some(StopReason::Pause))
    }

    /// Builds the active DAP call stack frames.
    ///
    /// Returns Frame 0 (current `PC`, symbol, and source coordinates) followed by caller frames
    /// reconstructed from the shadow call stack.
    pub fn get_stack_trace(&self) -> Vec<StackFrame> {
        let current_pc = self.machine.cpu.regs.pc.0;
        let mut frames = Vec::new();

        // Top Frame (0)
        let top_loc = self.source_map.address_to_location(current_pc);
        let top_sym = self.source_map.address_to_symbol(current_pc).unwrap_or("main");
        let (top_file, top_line, top_col) = if let Some(loc) = top_loc {
            (
                Some(Source {
                    name: loc.file_path.file_name().map(|n| n.to_string_lossy().to_string()),
                    path: Some(loc.file_path.to_string_lossy().to_string()),
                }),
                loc.line,
                loc.col,
            )
        } else {
            (
                Some(Source {
                    name: self.program_path.file_name().map(|n| n.to_string_lossy().to_string()),
                    path: Some(self.program_path.to_string_lossy().to_string()),
                }),
                1,
                1,
            )
        };

        frames.push(StackFrame {
            id: 0,
            name: format!("{top_sym} (0x{current_pc:04X})"),
            source: top_file,
            line: top_line,
            column: top_col,
            instruction_pointer_reference: Some(format!("0x{current_pc:04X}")),
        });

        // Shadow caller frames
        for (i, frame) in self.shadow_stack.iter().rev().enumerate() {
            let caller_loc = self.source_map.address_to_location(frame.call_address);
            let (source, line, col) = if let Some(loc) = caller_loc {
                (
                    Some(Source {
                        name: loc.file_path.file_name().map(|n| n.to_string_lossy().to_string()),
                        path: Some(loc.file_path.to_string_lossy().to_string()),
                    }),
                    loc.line,
                    loc.col,
                )
            } else {
                (None, 1, 1)
            };

            frames.push(StackFrame {
                id: (i + 1) as i64,
                name: format!("{} (call @ 0x{:04X})", frame.subroutine_name, frame.call_address),
                source,
                line,
                column: col,
                instruction_pointer_reference: Some(format!("0x{:04X}", frame.call_address)),
            });
        }

        frames
    }

    /// Returns the seven specialized variable scopes for the given stack frame ID:
    /// CPU Registers, Register Pairs, Flags, Data Segment, BSS Segment, Stack Memory, Peripherals & Diagnostics.
    pub fn get_scopes(&self, frame_id: i64) -> Vec<Scope> {
        get_scopes(frame_id)
    }

    /// Resolves and formats variables for a given scope or variable reference ID.
    pub fn get_variables(&self, variables_reference: i64) -> Vec<Variable> {
        get_variables(
            variables_reference,
            &self.machine,
            Some(&self.source_map),
            self.elapsed_t_states,
            self.instruction_count,
        )
    }

    /// Mutates a register, flag, data variable, or memory location during an active debug pause.
    ///
    /// # Errors
    /// Returns [`DapError::InvalidVariable`] if the target variable cannot be modified or the value is invalid.
    pub fn set_variable(&mut self, ref_id: i64, name: &str, value: &str) -> Result<(String, String), DapError> {
        set_variable(ref_id, name, value, &mut self.machine, Some(&self.source_map))
    }

    /// Evaluates a debugger expression, comparison (`A == 0x05`), pointer dereference (`*0x2000`), or REPL command (`:in`, `:key`, `:mem`, `:regs`, `:help`).
    ///
    /// # Errors
    /// Returns [`DapError`] if evaluation syntax or identifier resolution fails.
    pub fn evaluate(&mut self, expr: &str) -> Result<EvalResult, DapError> {
        let trimmed = expr.trim();
        if let Some(cmd) = trimmed.strip_prefix(':') {
            let parts: Vec<&str> = cmd.split_whitespace().collect();
            if !parts.is_empty() {
                match parts[0] {
                    "in" | "input" | "stdin" => {
                        let text = cmd.strip_prefix(parts[0]).unwrap().trim();
                        self.feed_terminal_input(text);
                        return Ok(EvalResult {
                            display: format!("Fed input to TextIO device: \"{text}\""),
                            r#type: "input".to_string(),
                            raw_value: None,
                        });
                    }
                    "key" => {
                        let arg = cmd.strip_prefix(parts[0]).unwrap().trim();
                        let ch = arg.chars().next().unwrap_or('\n');
                        self.press_keyboard_char(ch);
                        return Ok(EvalResult {
                            display: format!("Pressed key on keyboard: '{ch}'"),
                            r#type: "key".to_string(),
                            raw_value: None,
                        });
                    }
                    _ => {}
                }
            }
        }
        evaluate_expression(expr, &self.machine, Some(&self.source_map))
    }

    /// Disassembles `count` consecutive instructions from RAM starting at `start_addr`,
    /// annotating instructions with exported symbols and source locations.
    pub fn disassemble_memory(&self, start_addr: u16, count: usize) -> Vec<DisassembledInstruction> {
        let mut results = Vec::new();
        let mut addr = start_addr;

        for _ in 0..count {
            let opcode_byte = self.machine.ram.read(Addr(addr));
            let size = opcode_size_bytes(opcode_byte);
            let mut bytes = Vec::new();
            for j in 0..size {
                bytes.push(self.machine.ram.read(Addr(addr.wrapping_add(j))));
            }

            let instr_str = if let Ok(rows) = disassemble_bytes(&bytes) {
                if let Some(r) = rows.first() {
                    r.mnemonic.clone()
                } else {
                    format!("DB 0x{opcode_byte:02X}")
                }
            } else {
                format!("DB 0x{opcode_byte:02X}")
            };

            let symbol = self.source_map.address_to_symbol(addr).map(|s| s.to_string());
            let loc = self.source_map.address_to_location(addr);
            let (source, line, col) = if let Some(l) = loc {
                (
                    Some(Source {
                        name: l.file_path.file_name().map(|n| n.to_string_lossy().to_string()),
                        path: Some(l.file_path.to_string_lossy().to_string()),
                    }),
                    Some(l.line),
                    Some(l.col),
                )
            } else {
                (None, None, None)
            };

            results.push(DisassembledInstruction {
                address: format!("0x{addr:04X}"),
                instruction_bytes: Some(bytes.iter().map(|b| format!("{b:02X}")).collect::<Vec<_>>().join(" ")),
                instruction: instr_str,
                symbol,
                location: source,
                line,
                column: col,
            });

            addr = addr.wrapping_add(size);
        }

        results
    }

    fn run_execution_loop(&mut self) -> Result<Option<StopReason>, DapError> {
        let mut step_limit = 100_000;
        while self.is_running && step_limit > 0 {
            step_limit -= 1;
            let current_pc = self.machine.cpu.regs.pc.0;

            // Check breakpoint hit
            if let Some(hit) = self.breakpoints.check_hit(current_pc, &self.machine, &self.source_map) {
                self.is_running = false;
                let reason = StopReason::Breakpoint(hit.id);
                self.stop_reason = Some(reason.clone());
                return Ok(Some(reason));
            }

            // Execute single instruction
            if let Some(stop) = self.execute_single_step()? {
                self.is_running = false;
                self.stop_reason = Some(stop.clone());
                return Ok(Some(stop));
            }
        }

        if step_limit == 0 {
            self.is_running = false;
            self.stop_reason = Some(StopReason::Pause);
            Ok(Some(StopReason::Pause))
        } else {
            Ok(self.stop_reason.clone())
        }
    }

    fn execute_single_step(&mut self) -> Result<Option<StopReason>, DapError> {
        let pc_before = self.machine.cpu.regs.pc.0;
        let opcode_byte = self.machine.ram.read(Addr(pc_before));

        if opcode_byte == 0x76 {
            // HLT instruction
            self.is_running = false;
            self.is_terminated = true;
            return Ok(Some(StopReason::Halt));
        }

        let is_call = is_call_instruction(opcode_byte);
        let is_ret = is_return_instruction(opcode_byte);

        // Step instruction: advance ticks until the start of the next Fetch cycle
        if self.machine.cpu.is_halt {
            self.is_running = false;
            self.is_terminated = true;
            return Ok(Some(StopReason::Halt));
        }

        let mut t_states = 0;
        self.machine.tick();
        t_states += 1;

        while !self.machine.cpu.is_halt
            && self.machine.cpu.fault.is_none()
            && !(self.machine.cpu.cycle == MachineCycle::Fetch && self.machine.cpu.t_state == 1)
            && t_states < 100
        {
            self.machine.tick();
            t_states += 1;
        }

        if let Some(ref fault) = self.machine.cpu.fault {
            self.is_running = false;
            return Ok(Some(StopReason::Exception(format!("{fault:?}"))));
        }

        self.elapsed_t_states += t_states as u64;
        self.instruction_count += 1;
        let pc_after = self.machine.cpu.regs.pc.0;

        // Shadow stack update
        if is_call && pc_after != pc_before.wrapping_add(3) {
            let sub_name = self.source_map.address_to_symbol(pc_after).unwrap_or("subroutine").to_string();
            self.shadow_stack.push(ShadowFrame {
                call_address: pc_before,
                return_address: pc_before.wrapping_add(3),
                subroutine_name: sub_name,
            });
        } else if is_ret {
            self.shadow_stack.pop();
        }

        if self.machine.cpu.is_halt {
            self.is_running = false;
            self.is_terminated = true;
            return Ok(Some(StopReason::Halt));
        }

        Ok(None)
    }

    fn record_snapshot(&mut self) {
        if self.history.len() >= MAX_HISTORY_SNAPSHOTS {
            self.history.pop_front();
        }
        let ram_copy = self.machine.ram.as_slice().to_vec().into_boxed_slice();
        self.history.push_back(MachineSnapshot {
            cpu: self.machine.cpu.clone(),
            ram: ram_copy,
            elapsed_t_states: self.elapsed_t_states,
            instruction_count: self.instruction_count,
            shadow_stack: self.shadow_stack.clone(),
        });
    }
}

fn is_call_instruction(op: u8) -> bool {
    matches!(
        op,
        0xCD | 0xC4 | 0xCC | 0xD4 | 0xDC | 0xE4 | 0xEC | 0xF4 | 0xFC
            | 0xC7 | 0xCF | 0xD7 | 0xDF | 0xE7 | 0xEF | 0xF7 | 0xFF
    )
}

fn is_return_instruction(op: u8) -> bool {
    matches!(
        op,
        0xC9 | 0xC0 | 0xC8 | 0xD0 | 0xD8 | 0xE0 | 0xE8 | 0xF0 | 0xF8
    )
}

fn opcode_size_bytes(op: u8) -> u16 {
    match op {
        0x01 | 0x11 | 0x21 | 0x31 | 0x22 | 0x2A | 0x32 | 0x3A
        | 0xC2 | 0xC3 | 0xCA | 0xD2 | 0xDA | 0xE2 | 0xEA | 0xF2 | 0xFA
        | 0xC4 | 0xCC | 0xCD | 0xD4 | 0xDC | 0xE4 | 0xEC | 0xF4 | 0xFC => 3,

        0x06 | 0x0E | 0x16 | 0x1E | 0x26 | 0x2E | 0x36 | 0x3E
        | 0xC6 | 0xCE | 0xD3 | 0xD6 | 0xDB | 0xDE | 0xE6 | 0xEE | 0xF6 | 0xFE => 2,

        _ => 1,
    }
}
