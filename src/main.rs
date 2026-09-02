use std::sync::mpsc::channel;

use emu8085::{
    Addr, Instruction, Machine, Opcode, Operand, Program, TerminalDevice,
    device::terminal::{CMD_DISPLAY, CMD_READ, CMD_WRITE},
};

fn main() {
    let (tx, rx) = channel::<u8>();

    std::thread::spawn(move || {
        use std::io::{self, Read};
        let mut buf = [0u8; 1];
        while io::stdin().read_exact(&mut buf).is_ok() {
            if tx.send(buf[0]).is_err() {
                break;
            }
        }
    });

    let mut machine = Machine::default();

    let term_data_port: u8 = 0x01;
    let term_cmd_port: u8 = 0x02;
    let terminal = TerminalDevice::with_io(term_data_port, term_cmd_port, rx, |b| {
        print!("{}", b as char);
    });

    machine.attach_device(Box::new(terminal), &[term_data_port, term_cmd_port]);

    let prompt = "What is your name? \n";
    let prompt_addr = Addr::from_le(0x00, 0x00);
    for (i, ch) in prompt.chars().enumerate() {
        machine
            .ram
            .write(prompt_addr.wrapping_add(i as u16), ch as u8);
    }

    let hello = "Hi ";
    let hello_addr = prompt_addr.wrapping_add(prompt.len() as u16);
    for (i, ch) in hello.chars().enumerate() {
        machine
            .ram
            .write(hello_addr.wrapping_add(i as u16), ch as u8);
    }
    let name_addr = hello_addr.wrapping_add(hello.len() as u16);
    let excl = "!\n";
    let excl_addr = Addr::from_le(0x00, 0x70);
    for (i, ch) in excl.chars().enumerate() {
        machine
            .ram
            .write(excl_addr.wrapping_add(i as u16), ch as u8);
    }

    let program = Program::new(vec![
        // Enter write mode on the terminal command port
        Instruction::with(Opcode::MVI_A, Operand::Byte(CMD_WRITE)),
        Instruction::with(Opcode::OUT, Operand::Byte(term_cmd_port)),
        // Write the length of the prompt to the terminal data port
        Instruction::with(Opcode::MVI_A, Operand::Byte(prompt.len() as u8)),
        Instruction::with(Opcode::OUT, Operand::Byte(term_data_port)),
        // Point HL to the prompt in RAM
        Instruction::with(Opcode::MVI_HL, Operand::Word(prompt_addr.0)),
        // Setup loop counter in B register
        Instruction::with(Opcode::MVI_B, Operand::Byte(prompt.len() as u8)),
        // Loop: write prompt characters
        Instruction::new(Opcode::MOV_AM).labeled("send_prompt"),
        Instruction::with(Opcode::OUT, Operand::Byte(term_data_port)),
        Instruction::new(Opcode::INX_HL),
        Instruction::new(Opcode::DCR_B),
        Instruction::with(Opcode::JNZ, Operand::Label(String::from("send_prompt"))),
        // Display buffer (print prompt)
        Instruction::with(Opcode::MVI_A, Operand::Byte(CMD_DISPLAY)),
        Instruction::with(Opcode::OUT, Operand::Byte(term_cmd_port)),
        // Trigger capture from the terminal to read name
        Instruction::with(Opcode::MVI_A, Operand::Byte(CMD_READ)),
        Instruction::with(Opcode::OUT, Operand::Byte(term_cmd_port)),
        // Read the length byte of the captured input
        Instruction::with(Opcode::IN, Operand::Byte(term_data_port)),
        // Store length in B (loop counter) and C (preserved length)
        Instruction::new(Opcode::MOV_BA),
        Instruction::new(Opcode::MOV_CA),
        // Point HL to name_addr where we want to write the data
        Instruction::with(Opcode::MVI_HL, Operand::Word(name_addr.0)),
        // Check if length is 0; if it is, skip read loop
        Instruction::new(Opcode::MOV_AB),
        Instruction::with(Opcode::CPI, Operand::Byte(0x00)),
        Instruction::with(Opcode::JZ, Operand::Label(String::from("capture_done"))),
        // Loop: read input characters and store them in name_addr
        Instruction::with(Opcode::IN, Operand::Byte(term_data_port)).labeled("read_char"),
        Instruction::new(Opcode::MOV_MA),
        Instruction::new(Opcode::INX_HL),
        Instruction::new(Opcode::DCR_B),
        Instruction::with(Opcode::JNZ, Operand::Label(String::from("read_char"))),
        // Append trailing newline to name_addr
        Instruction::with(Opcode::MVI_A, Operand::Byte(0x0A)).labeled("capture_done"),
        Instruction::new(Opcode::MOV_MA),
        // Now print "Hi {name}!\n"
        // Enter write mode on the terminal command port
        Instruction::with(Opcode::MVI_A, Operand::Byte(CMD_WRITE)),
        Instruction::with(Opcode::OUT, Operand::Byte(term_cmd_port)),
        // Calculate total length (C + 5) and write it to terminal data port
        Instruction::new(Opcode::MOV_AC),
        Instruction::with(Opcode::ADI, Operand::Byte(5)),
        Instruction::with(Opcode::OUT, Operand::Byte(term_data_port)),
        // Write "Hi " prefix
        Instruction::with(Opcode::MVI_HL, Operand::Word(hello_addr.0)),
        Instruction::with(Opcode::MVI_B, Operand::Byte(3)),
        Instruction::new(Opcode::MOV_AM).labeled("send_hello"),
        Instruction::with(Opcode::OUT, Operand::Byte(term_data_port)),
        Instruction::new(Opcode::INX_HL),
        Instruction::new(Opcode::DCR_B),
        Instruction::with(Opcode::JNZ, Operand::Label(String::from("send_hello"))),
        // Write {name} payload
        Instruction::with(Opcode::MVI_HL, Operand::Word(name_addr.0)),
        Instruction::new(Opcode::MOV_BC), // load name length into B
        Instruction::new(Opcode::MOV_AB),
        Instruction::with(Opcode::CPI, Operand::Byte(0x00)),
        Instruction::with(Opcode::JZ, Operand::Label(String::from("send_name_done"))),
        Instruction::new(Opcode::MOV_AM).labeled("send_name"),
        Instruction::with(Opcode::OUT, Operand::Byte(term_data_port)),
        Instruction::new(Opcode::INX_HL),
        Instruction::new(Opcode::DCR_B),
        Instruction::with(Opcode::JNZ, Operand::Label(String::from("send_name"))),
        // Write suffix "!\n"
        Instruction::with(Opcode::MVI_HL, Operand::Word(excl_addr.0)).labeled("send_name_done"),
        Instruction::with(Opcode::MVI_B, Operand::Byte(2)),
        Instruction::new(Opcode::MOV_AM).labeled("send_excl"),
        Instruction::with(Opcode::OUT, Operand::Byte(term_data_port)),
        Instruction::new(Opcode::INX_HL),
        Instruction::new(Opcode::DCR_B),
        Instruction::with(Opcode::JNZ, Operand::Label(String::from("send_excl"))),
        // Display buffer
        Instruction::with(Opcode::MVI_A, Operand::Byte(CMD_DISPLAY)),
        Instruction::with(Opcode::OUT, Operand::Byte(term_cmd_port)),
        Instruction::new(Opcode::HLT),
    ]);

    machine.load(&program, Addr::from_le(0x00, 0xA0)).unwrap();
    machine.run();
}
