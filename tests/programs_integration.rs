//! Integration tests for files in `programs/` directory.
//!
//! Driven by declarative manifests:
//! - `programs/programs.json` for standalone/terminal/printer executables in `programs/`
//! - `programs/libraries.json` for reusable subroutine libraries in `devices/` and `lib/`

use std::fs;
use std::path::Path;
use std::sync::mpsc::channel;
use std::sync::{Arc, Mutex};

use emu8085::asm::container::BinaryContainer;
use emu8085::asm::{assemble, assemble_and_link, load};
use emu8085::{Machine, PrinterDevice, TerminalDevice};
use serde::Deserialize;

#[derive(Debug, Deserialize)]
struct ProgramsManifest {
    programs: Vec<ProgramTestCase>,
}

#[derive(Debug, Deserialize)]
struct ProgramTestCase {
    name: String,
    file: String,
    #[serde(rename = "type")]
    test_type: String,
    #[serde(default)]
    link_libs: Vec<String>,
    #[serde(default)]
    stdin: Option<String>,
    #[serde(default)]
    expect_output: Option<String>,
    #[serde(default)]
    expect_reg_a: Option<u8>,
    #[serde(default)]
    expect_reg_h: Option<u8>,
    #[serde(default)]
    expect_reg_l: Option<u8>,
    #[serde(default)]
    expect_reg_sp: Option<u16>,
}

#[derive(Debug, Deserialize)]
struct LibrariesManifest {
    libraries: Vec<LibraryTestCase>,
}

#[derive(Debug, Deserialize)]
struct LibraryTestCase {
    name: String,
    dir: String,
    file: String,
    #[serde(default)]
    link_libs: Vec<String>,
    expected_symbols: Vec<String>,
}

fn compile_lib_file(rel_path: &str, base_dir: &Path) -> BinaryContainer {
    let full_path = base_dir.join(rel_path);
    let src = fs::read_to_string(&full_path)
        .unwrap_or_else(|e| panic!("failed to read library '{}': {e}", full_path.display()));
    let img = assemble(&src)
        .unwrap_or_else(|e| panic!("failed to assemble library '{}': {e}", full_path.display()));
    img.to_container()
}

#[test]
fn test_programs_manifest_suite() {
    let base_dir = Path::new("programs");
    let manifest_path = base_dir.join("programs.json");
    let manifest_str = fs::read_to_string(&manifest_path)
        .unwrap_or_else(|e| panic!("failed to read {}: {e}", manifest_path.display()));
    let manifest: ProgramsManifest =
        serde_json::from_str(&manifest_str).expect("valid programs.json schema");

    for prog in manifest.programs {
        let prog_path = base_dir.join(&prog.file);
        let src = fs::read_to_string(&prog_path)
            .unwrap_or_else(|e| panic!("failed to read program '{}': {e}", prog_path.display()));

        let linked_containers: Vec<BinaryContainer> = prog
            .link_libs
            .iter()
            .map(|lib_file| compile_lib_file(lib_file, base_dir))
            .collect();

        let image = assemble_and_link(&src, Some(base_dir), &linked_containers)
            .unwrap_or_else(|e| panic!("failed to assemble & link '{}': {e}", prog.name));

        let mut machine = Machine::default();

        match prog.test_type.as_str() {
            "register" => {
                load(&mut machine, &image)
                    .unwrap_or_else(|e| panic!("failed to load '{}': {e}", prog.name));
                machine.run();
                assert!(machine.cpu.is_halt, "program '{}' did not halt", prog.name);

                if let Some(expected_a) = prog.expect_reg_a {
                    assert_eq!(
                        machine.cpu.regs.a, expected_a,
                        "program '{}' reg A mismatch",
                        prog.name
                    );
                }
                if let Some(expected_h) = prog.expect_reg_h {
                    assert_eq!(
                        machine.cpu.regs.h, expected_h,
                        "program '{}' reg H mismatch",
                        prog.name
                    );
                }
                if let Some(expected_l) = prog.expect_reg_l {
                    assert_eq!(
                        machine.cpu.regs.l, expected_l,
                        "program '{}' reg L mismatch",
                        prog.name
                    );
                }
                if let Some(expected_sp) = prog.expect_reg_sp {
                    assert_eq!(
                        machine.cpu.regs.sp.0, expected_sp,
                        "program '{}' reg SP mismatch",
                        prog.name
                    );
                }
            }
            "printer" => {
                let printed = Arc::new(Mutex::new(String::new()));
                let sink = printed.clone();
                machine.attach_device(
                    Box::new(PrinterDevice::with_callback(move |c| {
                        sink.lock().unwrap().push(c);
                    })),
                    &[0x02],
                );
                load(&mut machine, &image)
                    .unwrap_or_else(|e| panic!("failed to load '{}': {e}", prog.name));
                machine.run();
                assert!(machine.cpu.is_halt, "program '{}' did not halt", prog.name);

                if let Some(expected) = &prog.expect_output {
                    assert_eq!(
                        &*printed.lock().unwrap(),
                        expected,
                        "program '{}' printer output mismatch",
                        prog.name
                    );
                }
            }
            "terminal" => {
                let output = Arc::new(Mutex::new(Vec::new()));
                let output_clone = output.clone();
                let (_tx, rx) = channel();
                let mut terminal = TerminalDevice::with_io(0x01, 0x02, rx, move |b| {
                    output_clone.lock().unwrap().push(b);
                });

                if let Some(input_text) = &prog.stdin {
                    terminal.feed_line(input_text);
                }

                machine.attach_device(Box::new(terminal), &[0x01, 0x02]);
                load(&mut machine, &image)
                    .unwrap_or_else(|e| panic!("failed to load '{}': {e}", prog.name));
                machine.run();
                assert!(machine.cpu.is_halt, "program '{}' did not halt", prog.name);

                if let Some(expected) = &prog.expect_output {
                    let out_str = String::from_utf8_lossy(&output.lock().unwrap()).to_string();
                    assert_eq!(
                        out_str, *expected,
                        "program '{}' terminal output mismatch",
                        prog.name
                    );
                }
            }
            other => panic!("unknown test type '{other}' for program '{}'", prog.name),
        }
    }
}

#[test]
fn test_libraries_manifest_suite() {
    let base_dir = Path::new("programs");
    let manifest_path = base_dir.join("libraries.json");
    let manifest_str = fs::read_to_string(&manifest_path)
        .unwrap_or_else(|e| panic!("failed to read {}: {e}", manifest_path.display()));
    let manifest: LibrariesManifest =
        serde_json::from_str(&manifest_str).expect("valid libraries.json schema");

    for lib in manifest.libraries {
        let lib_path = base_dir.join(&lib.file);
        let src = fs::read_to_string(&lib_path)
            .unwrap_or_else(|e| panic!("failed to read library '{}': {e}", lib_path.display()));

        let linked_containers: Vec<BinaryContainer> = lib
            .link_libs
            .iter()
            .map(|f| compile_lib_file(f, base_dir))
            .collect();

        let image = assemble_and_link(&src, Some(base_dir), &linked_containers)
            .unwrap_or_else(|e| panic!("failed to assemble & link library '{}': {e}", lib.name));

        assert_eq!(
            image.entry, 0,
            "library '{}' in '{}' should have entry point 0x0000 (pure library)",
            lib.name, lib.dir
        );

        let container = image.to_container();
        assert_eq!(
            container.header.entry_pc, 0,
            "container entry_pc must be 0 for library '{}'",
            lib.name
        );

        for expected_sym in &lib.expected_symbols {
            assert!(
                container.lookup_symbol(expected_sym).is_some(),
                "library '{}' must export symbol '{}'",
                lib.name,
                expected_sym
            );
        }
    }
}
