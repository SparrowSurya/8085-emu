use std::path::Path;
use emu8085::asm::{assemble_and_link_with_file, load};
use emu8085::device::canvas::*;
use emu8085::{CanvasDevice, Machine};

fn setup_test_machine(src: &str) -> (Machine, CanvasDevice) {
    let base_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("examples");
    let fake_file = base_dir.join("test_canvas.e8085");

    let image = assemble_and_link_with_file(Some(&fake_file), src, Some(&base_dir), None, &[])
        .expect("assembly must succeed");

    let mut machine = Machine::default();
    let canvas = CanvasDevice::headless(0x10, 0x11, 0x12);
    load(&mut machine, &image).expect("image loading must succeed");

    (machine, canvas)
}

#[test]
fn test_canvas_assembly_lifecycle_headless() {
    let src = r#"
%include <device/canvas>

segment .data
title_str byte "Retro Game\0"
title_buf byte 32

segment .text
main:
    ; 1. Initialize canvas (160x90) with title
    InitCanvas 160, 90, title_str

    ; 2. Read canvas size into B (w) and C (h)
    GetCanvasSize

    ; 3. Read window title into memory
    GetCanvasTitle title_buf

    ; 4. Check if canvas is open (A=1)
    IsCanvasOpen

    ; 5. Dispose canvas
    DisposeCanvas

    hlt
"#;

    let (mut machine, canvas) = setup_test_machine(src);
    machine.attach_device(Box::new(canvas), &[0x10, 0x11, 0x12]);

    machine.run();
    assert!(machine.cpu.is_halt);

    let dev = machine.devices.find_device::<CanvasDevice>().unwrap();
    assert_eq!(dev.logical_width(), 160);
    assert_eq!(dev.logical_height(), 90);
    assert_eq!(dev.title(), "Retro Game");
    assert!(!dev.is_open()); // Disposed
}

#[test]
fn test_canvas_gameloop_timing() {
    let src = r#"
%include <device/canvas>

segment .data
title byte "FPS Test\0"

segment .text
main:
    InitCanvas 128, 128, title

    ; Set FPS to 60
    SetFPS 60

    ; Read FPS back into A
    GetFPS
    mov B, A

    ; Update canvas frame
    UpdateCanvas

    ; Read frame delta into A
    GetDelta
    mov C, A

    hlt
"#;

    let (mut machine, canvas) = setup_test_machine(src);
    machine.attach_device(Box::new(canvas), &[0x10, 0x11, 0x12]);

    machine.run();
    assert!(machine.cpu.is_halt);

    assert_eq!(machine.cpu.regs.b, 60);
    let dev = machine.devices.find_device::<CanvasDevice>().unwrap();
    assert_eq!(dev.fps(), 60);
}

#[test]
fn test_canvas_drawing_primitives() {
    let src = r#"
%include <device/canvas>

segment .data
title byte "Draw Test\0"

segment .text
main:
    InitCanvas 160, 90, title

    ; 1. Clear background to Red (0xF00F)
    Clear COLOR_RED_HI, COLOR_RED_LO

    ; 2. Set Pixel at (10, 10) to Blue (0x00FF)
    SetPixel 10, 10, COLOR_BLUE_HI, COLOR_BLUE_LO

    ; 3. Draw Green line from (20, 20) to (25, 20)
    Line 20, 20, 25, 20, 1, COLOR_GREEN_HI, COLOR_GREEN_LO

    ; 4. Draw White filled rectangle at (30, 30) of size (5, 5)
    FillRectangle 30, 30, 5, 5, COLOR_WHITE_HI, COLOR_WHITE_LO

    ; 5. Draw Yellow stroked rectangle at (40, 40) of size (6, 6)
    StrokeRectangle 40, 40, 6, 6, 1, COLOR_YELLOW_HI, COLOR_YELLOW_LO

    ; 6. Draw Magenta filled ellipse centered at (60, 60), rx=4, ry=4
    FillEllipse 60, 60, 4, 4, COLOR_MAGENTA_HI, COLOR_MAGENTA_LO

    ; 7. Draw Cyan stroked ellipse centered at (75, 75), rx=5, ry=5
    StrokeEllipse 75, 75, 5, 5, 1, COLOR_CYAN_HI, COLOR_CYAN_LO

    ; 8. Read back pixel at (10, 10) into B:C
    GetPixel 10, 10

    hlt
"#;

    let (mut machine, canvas) = setup_test_machine(src);
    machine.attach_device(Box::new(canvas), &[0x10, 0x11, 0x12]);

    machine.run();
    assert!(machine.cpu.is_halt);

    // B = 0x00, C = 0xFF (COLOR_BLUE)
    assert_eq!(machine.cpu.regs.b, 0x00);
    assert_eq!(machine.cpu.regs.c, 0xFF);

    let dev = machine.devices.find_device::<CanvasDevice>().unwrap();
    // Verify background is red (0xF00F)
    assert_eq!(dev.get_pixel_value(0, 0), COLOR_RED);
    // Verify blue pixel at (10, 10)
    assert_eq!(dev.get_pixel_value(10, 10), COLOR_BLUE);
    // Verify green line at (22, 20)
    assert_eq!(dev.get_pixel_value(22, 20), COLOR_GREEN);
    // Verify white rectangle at (32, 32)
    assert_eq!(dev.get_pixel_value(32, 32), COLOR_WHITE);
    // Verify yellow stroke rectangle border at (40, 40)
    assert_eq!(dev.get_pixel_value(40, 40), COLOR_YELLOW);
    // Verify magenta filled ellipse at (60, 60)
    assert_eq!(dev.get_pixel_value(60, 60), COLOR_MAGENTA);
    // Verify cyan stroked ellipse at (75, 70)
    assert_eq!(dev.get_pixel_value(75, 70), COLOR_CYAN);
}

#[test]
fn test_canvas_text_rendering() {
    let src = r#"
%include <device/canvas>

segment .data
title byte "Text Test\0"
msg byte "HI\0"

segment .text
main:
    InitCanvas 160, 90, title

    ; Calculate width of "HI" at scale 1
    TextWidth msg, 1
    mov B, A

    ; Render filled text
    FillText msg, 10, 10, 1, COLOR_WHITE_HI, COLOR_WHITE_LO

    ; Render stroke text
    StrokeText msg, 10, 25, 1, 1, COLOR_YELLOW_HI, COLOR_YELLOW_LO

    hlt
"#;

    let (mut machine, canvas) = setup_test_machine(src);
    machine.attach_device(Box::new(canvas), &[0x10, 0x11, 0x12]);

    machine.run();
    assert!(machine.cpu.is_halt);

    // "HI" is 2 characters * 8 pixels = 16 pixels
    assert_eq!(machine.cpu.regs.b, 16);

    let dev = machine.devices.find_device::<CanvasDevice>().unwrap();
    // Check that some pixels in the text region (10..26, 10..18) are rendered with COLOR_WHITE
    let mut white_count = 0;
    for y in 10..18 {
        for x in 10..26 {
            if dev.get_pixel_value(x, y) == COLOR_WHITE {
                white_count += 1;
            }
        }
    }
    assert!(white_count > 0, "Text 'HI' should render white pixels");
}

#[test]
fn test_canvas_keyboard_simulation() {
    let src = r#"
%include <device/canvas>

segment .data
title byte "Key Test\0"

segment .text
main:
    InitCanvas 160, 90, title

    ; 1. Check KeyDown for K_A
    KeyDown K_A
    mov B, A

    ; 2. Check KeyPress for K_A
    KeyPress K_A
    mov C, A

    ; 3. Pop KeyInput
    KeyInput
    mov D, A

    hlt
"#;

    let (mut machine, mut canvas) = setup_test_machine(src);
    // Simulate key K_A pressed
    canvas.simulate_key_down(K_A);

    machine.attach_device(Box::new(canvas), &[0x10, 0x11, 0x12]);
    machine.run();
    assert!(machine.cpu.is_halt);

    // KeyDown(K_A) should return 1
    assert_eq!(machine.cpu.regs.b, 1);
    // KeyPress(K_A) should return 1
    assert_eq!(machine.cpu.regs.c, 1);
    // KeyInput should return K_A (0x41)
    assert_eq!(machine.cpu.regs.d, K_A);
}

#[test]
fn test_canvas_complete_pong_gameloop() {
    let src = r#"
%include <device/canvas>

segment .data
title_str byte "8085 Pong Game\0"

segment .bss
paddle_y byte 1
loop_counter byte 1

segment .text
main:
    ; 1. Initialize canvas
    InitCanvas RES_DEFAULT_W, RES_DEFAULT_H, title_str
    SetFPS 60

    mvi A, 40
    sta paddle_y
    mvi A, 5
    sta loop_counter

game_loop:
    ; Check if window open
    IsCanvasOpen
    jz game_exit

    ; Clear canvas (Dark Navy Blue: 0x00, 0x2F)
    Clear 0x00, 0x2F

    ; Draw paddle
    lda paddle_y
    FillRectangle 10, 40, 4, 16, COLOR_WHITE_HI, COLOR_WHITE_LO

    ; Draw ball
    FillEllipse 80, 45, 3, 3, COLOR_YELLOW_HI, COLOR_YELLOW_LO

    ; Update canvas
    UpdateCanvas

    ; Decrement loop counter to exit after 5 frames
    lda loop_counter
    dcr A
    sta loop_counter
    jnz game_loop

game_exit:
    DisposeCanvas
    hlt
"#;

    let (mut machine, canvas) = setup_test_machine(src);
    machine.attach_device(Box::new(canvas), &[0x10, 0x11, 0x12]);

    machine.run();
    assert!(machine.cpu.is_halt);

    let dev = machine.devices.find_device::<CanvasDevice>().unwrap();
    assert!(!dev.is_open());
}

#[test]
fn test_canvas_mouse_simulation() {
    let src = r#"
%include <device/canvas>

segment .data
title byte "Mouse Test\0"

segment .text
main:
    InitCanvas 160, 90, title

    ; 1. Read Mouse Position (loads X into B, Y into C)
    MousePosition
    mov D, B
    mov E, C

    ; 2. Check MouseDown for Left button (held down)
    MouseDown MOUSE_BUTTON_LEFT
    mov H, A

    ; 3. Check MousePress for Left button (just pressed)
    MousePress MOUSE_BUTTON_LEFT
    mov L, A

    ; 4. Check MousePress for Right button (not pressed)
    MousePress MOUSE_BUTTON_RIGHT
    mov B, A

    ; 5. Check MouseRelease for Left button (not released yet)
    MouseRelease MOUSE_BUTTON_LEFT
    mov C, A

    hlt
"#;

    let (mut machine, mut canvas) = setup_test_machine(src);
    // Simulate mouse position at (55, 33)
    canvas.simulate_mouse_move(55, 33);
    // Simulate mouse down on Left button
    canvas.simulate_mouse_down(MOUSE_BUTTON_LEFT);

    machine.attach_device(Box::new(canvas), &[0x10, 0x11, 0x12]);
    machine.run();
    assert!(machine.cpu.is_halt);

    // MousePosition X in D = 55, Y in E = 33
    assert_eq!(machine.cpu.regs.d, 55);
    assert_eq!(machine.cpu.regs.e, 33);
    // MouseDown(Left) in H = 1
    assert_eq!(machine.cpu.regs.h, 1);
    // MousePress(Left) in L = 1
    assert_eq!(machine.cpu.regs.l, 1);
    // MousePress(Right) in B = 0
    assert_eq!(machine.cpu.regs.b, 0);
    // MouseRelease(Left) in C = 0
    assert_eq!(machine.cpu.regs.c, 0);

    // Status register bit 3 (STATUS_MOUSE_DOWN) should be set
    let dev = machine.devices.find_device::<CanvasDevice>().unwrap();
    assert_eq!(dev.mouse_x, 55);
    assert_eq!(dev.mouse_y, 33);
}

#[test]
fn test_canvas_mouse_transitions() {
    let mut canvas = CanvasDevice::headless(0x10, 0x11, 0x12);

    // Initially all up
    assert!(!canvas.mouse_down[0]);
    assert!(!canvas.mouse_prev[0]);

    // Frame 1: Press Left button
    canvas.simulate_mouse_down(MOUSE_BUTTON_LEFT);
    assert!(canvas.mouse_down[0] && !canvas.mouse_prev[0]); // Pressed = true

    // End of Frame 1: UpdateCanvas
    canvas.update_canvas_internal();
    assert!(canvas.mouse_down[0]);
    assert!(canvas.mouse_prev[0]); // Held = true, Pressed = false

    // Frame 2: Release Left button
    canvas.simulate_mouse_up(MOUSE_BUTTON_LEFT);
    assert!(!canvas.mouse_down[0] && canvas.mouse_prev[0]); // Released = true

    // End of Frame 2: UpdateCanvas
    canvas.update_canvas_internal();
    assert!(!canvas.mouse_down[0]);
    assert!(!canvas.mouse_prev[0]); // Released = false
}

#[test]
fn test_canvas_paint_mask_primitive() {
    let src = r#"
%include <device/canvas>

segment .data
title byte "Mask Test\0"
; 4x4 test mask:
; 0 1 1 0
; 1 2 2 1
; 1 2 2 1
; 0 1 1 0
test_mask byte 0, 1, 1, 0, 1, 2, 2, 1, 1, 2, 2, 1, 0, 1, 1, 0

segment .text
main:
    InitCanvas 160, 90, title

    ; Clear canvas with black (0x00, 0x0F)
    Clear 0x00, 0x0F

    ; 1. Paint mask value 1 in Red (0xF0, 0x0F) at (10, 10)
    PaintMask 10, 10, test_mask, 4, 4, 1, 0xF0, 0x0F, 0

    ; 2. Paint mask value 2 in Green (0x0F, 0x0F) at (10, 10)
    PaintMask 10, 10, test_mask, 4, 4, 2, 0x0F, 0x0F, 0

    hlt
"#;

    let (mut machine, canvas) = setup_test_machine(src);
    machine.attach_device(Box::new(canvas), &[0x10, 0x11, 0x12]);
    machine.run();
    assert!(machine.cpu.is_halt);

    let dev = machine.devices.find_device::<CanvasDevice>().unwrap();
    // (10, 10) was 0 -> transparent, still black (0x000F)
    assert_eq!(dev.get_pixel_value(10, 10), 0x000F);
    // (11, 10) was 1 -> painted Red (0xF00F)
    assert_eq!(dev.get_pixel_value(11, 10), 0xF00F);
    // (11, 11) was 2 -> painted Green (0x0F0F)
    assert_eq!(dev.get_pixel_value(11, 11), 0x0F0F);
}
