//! Drives the buffered TextIODevice through the full CPU + RAM + DeviceManager loop
//! with monitor-style programs, exercising WRITE/DISPLAY and READ over the real bus.

use emu8085::device::text_io::{CMD_DISPLAY, CMD_READ, CMD_WRITE};
use emu8085::instruction::Opcode::*;
use emu8085::{
    Addr, Cpu, DeviceManager, Instruction as I, Memory, Operand, Program, SystemBus, TextIODevice,
};

const PORT_CMD: u8 = 0x08;
const PORT_DATA: u8 = 0x09;

fn out(port: u8, val: u8) -> Vec<I> {
    vec![
        I::with(MVI_A, Operand::byte(val)),
        I::with(OUT, Operand::byte(port)),
    ]
}

fn run(prog: Program, dm: &mut DeviceManager) -> Cpu {
    let mut cpu = Cpu::new();
    let mut ram = Memory::from_lines(16);
    let mut bus = SystemBus::default();
    ram.load_bytes(&prog.compile(Addr(0)).unwrap(), Addr(0))
        .unwrap();
    cpu.start_at(Addr(0));
    let mut t = 0;
    while !cpu.is_halt && cpu.fault.is_none() && t < 100_000 {
        cpu.process(&mut bus);
        ram.step(&mut bus);
        dm.step(&mut bus);
        t += 1;
    }
    cpu
}

#[test]
fn program_writes_and_displays_hi() {
    // OUT cmd,WRITE ; OUT data,2 ; OUT data,'H' ; OUT data,'I' ; OUT cmd,DISPLAY ; HLT
    let mut insts = Vec::new();
    insts.extend(out(PORT_CMD, CMD_WRITE));
    insts.extend(out(PORT_DATA, 2));
    insts.extend(out(PORT_DATA, b'H'));
    insts.extend(out(PORT_DATA, b'I'));
    insts.extend(out(PORT_CMD, CMD_DISPLAY));
    insts.push(I::new(HLT));

    let mut dm = DeviceManager::new();
    dm.attach(
        Box::new(TextIODevice::new(PORT_CMD, PORT_DATA)),
        &[PORT_CMD, PORT_DATA],
    );
    run(Program::new(insts), &mut dm);

    let text_io = dm.device_ref::<TextIODevice>(0).unwrap();
    assert_eq!(text_io.output_string(), "HI");
}

#[test]
fn program_reads_a_line_length_then_bytes() {
    // OUT cmd,READ ; IN data->B(len) ; IN data->C ; IN data->D ; HLT
    let mut insts = Vec::new();
    insts.extend(out(PORT_CMD, CMD_READ));
    insts.push(I::with(IN, Operand::byte(PORT_DATA)));
    insts.push(I::new(MOV_BA)); // B = length
    insts.push(I::with(IN, Operand::byte(PORT_DATA)));
    insts.push(I::new(MOV_CA)); // C = first byte
    insts.push(I::with(IN, Operand::byte(PORT_DATA)));
    insts.push(I::new(MOV_DA)); // D = second byte
    insts.push(I::new(HLT));

    let mut text_io = TextIODevice::new(PORT_CMD, PORT_DATA);
    text_io.feed_line("AB"); // host supplies a line of input
    let mut dm = DeviceManager::new();
    dm.attach(Box::new(text_io), &[PORT_CMD, PORT_DATA]);

    let cpu = run(Program::new(insts), &mut dm);
    assert_eq!(cpu.regs.b, 2); // length byte
    assert_eq!(cpu.regs.c, b'A');
    assert_eq!(cpu.regs.d, b'B');
}

#[test]
fn read_then_display_echoes_a_line() {
    // OUT cmd,READ ; OUT cmd,DISPLAY ; HLT  -- the line survives in the buffer.
    let mut insts = Vec::new();
    insts.extend(out(PORT_CMD, CMD_READ));
    insts.extend(out(PORT_CMD, CMD_DISPLAY));
    insts.push(I::new(HLT));

    let mut text_io = TextIODevice::new(PORT_CMD, PORT_DATA);
    text_io.feed_line("hello 8085");
    let mut dm = DeviceManager::new();
    dm.attach(Box::new(text_io), &[PORT_CMD, PORT_DATA]);
    run(Program::new(insts), &mut dm);

    assert_eq!(
        dm.device_ref::<TextIODevice>(0).unwrap().output_string(),
        "hello 8085"
    );
}

#[test]
fn repl_evaluates_arithmetic_expressions_and_exits() {
    let repl_path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("demo/repl.e8085");
    let base_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("demo");
    let src = std::fs::read_to_string(&repl_path).unwrap();

    let image = emu8085::asm::assemble_and_link(&src, Some(&base_dir), &[])
        .expect("assembles repl.e8085");

    let mut machine = emu8085::Machine::default();
    let mut text_io = TextIODevice::new(0x02, 0x01);
    text_io.feed_line("10 + 20");
    text_io.feed_line("100 - 45");
    text_io.feed_line("6 * 7");
    text_io.feed_line("50 / 5");
    text_io.feed_line("2 + 3 * 4"); // (2 + 3) * 4 = 20 (left-to-right evaluation)
    text_io.feed_line("10 / 0");     // Division by zero error
    text_io.feed_line("5 + invalid"); // Invalid syntax error
    text_io.feed_line("q");

    machine.attach_device(Box::new(text_io), &[0x01, 0x02]);
    emu8085::asm::load(&mut machine, &image).expect("loads repl image");
    
    machine.cpu.is_halt = false;
    machine.run();
    let dev = machine.devices.find_device::<TextIODevice>().unwrap();
    let out = dev.output_string();

    assert!(machine.cpu.is_halt);

    assert!(out.contains("Welcome to 8085 calculator REPL"));
    assert!(out.contains("30"));
    assert!(out.contains("55"));
    assert!(out.contains("42"));
    assert!(out.contains("10"));
    assert!(out.contains("20"));
    assert!(out.contains("Error: Division by zero"));
    assert!(out.contains("Error: Invalid Expression"));
    assert!(out.contains("Exited successfully"));
}
