# Intel 8085 Canvas Device Specification

The **Canvas Device** is an interactive 2D graphics, text rendering, and keyboard input peripheral for the `emu8085` emulator. It allows 8085 assembly programs to create windows, run real-time game loops, draw 2D primitives, render text, and receive keyboard events.

---

## 1. System Architecture & Memory Model

### 1.1 Host-Side Framebuffer & Fixed 4x4 Pixel Scaling
- The Intel 8085 microprocessor has an 8-bit data bus and a 16-bit address bus (64 KB total memory limit).
- A physical high-resolution display buffer would exceed the 8085's entire address space.
- **Design Principle**:
  - The actual framebuffer and window management **reside entirely on the host side** inside the `CanvasDevice`.
  - The 8085 program operates in a pure **8-bit coordinate space** ($0 \le X \le 255$, $0 \le Y \le 255$).
  - **Initialization & Scaling Rule**:
    - During `InitCanvas(width, height, title)`, the `CanvasDevice` creates the physical display window with dimensions `(width * 4) x (height * 4)`.
    - Internally, each logical pixel is automatically scaled by **4x4 physical pixels** on the physical display window.
    - **Immutability**: This 4x4 pixel scaling and the canvas dimensions are established **strictly during canvas initialization and cannot be changed later**. The window size remains fixed for the entire lifetime of the canvas.
    - This 4x4 scaling is completely transparent to the assembly program.
  - **Default Resolution**:
    - **Logical Resolution (Program View)**: `160 x 90` (8-bit coordinates).
    - **Physical Display Window**: `640 x 360` pixels ($160 \times 4 = 640$, $90 \times 4 = 360$).

### 1.2 Color Model: 16-bit RGBA4444
- Colors are represented as **16-bit values** (2 bytes / 1 word), providing 4-bit depth per channel:
  - **Red**: 4 bits (`0x0` to `0xF`, 16 levels)
  - **Green**: 4 bits (`0x0` to `0xF`, 16 levels)
  - **Blue**: 4 bits (`0x0` to `0xF`, 16 levels)
  - **Alpha**: 4 bits (`0x0` to `0xF`, 16 levels, `0x0` = transparent, `0xF` = fully opaque)
- **Binary Layout**: `0xRGBA` (High Byte = `(R << 4) | G`, Low Byte = `(B << 4) | A`).
- **Standard 16-bit Color Constants**:
  - `COLOR_BLACK`       = `0x000F`
  - `COLOR_WHITE`       = `0xFFFF`
  - `COLOR_RED`         = `0xF00F`
  - `COLOR_GREEN`       = `0x0F0F`
  - `COLOR_BLUE`        = `0x00FF`
  - `COLOR_YELLOW`      = `0xFF0F`
  - `COLOR_CYAN`        = `0x0FFF`
  - `COLOR_MAGENTA`     = `0xF0FF`
  - `COLOR_GRAY`        = `0x888F`
  - `COLOR_TRANSPARENT` = `0x0000`
- **Functional Directive Helpers**:
  - `Rgb(r, g, b)`: Constructs 16-bit RGBA4444 with full alpha (`0xF`).
  - `Rgba(r, g, b, a)`: Constructs 16-bit RGBA4444 with custom alpha channel.
- **Backward Compatibility 8-bit Component Aliases**:
  - Split constants (`COLOR_*_HI` and `COLOR_*_LO`) are preserved for existing codebase compatibility.

### 1.3 Recommended 1-Byte Game Resolutions & Presets

To maximize graphical fidelity within the 8085's 8-bit coordinate space ($0 \le X, Y \le 255$), the `CanvasDevice` supports several industry-standard retro aspect ratios. Each logical pixel scales 4x4 into physical window pixels:

| Logical Resolution | Aspect Ratio | Max 0-Indexed Coords | Physical Window Size (4x4) | Best Use Case & Industry Examples |
| :--- | :--- | :--- | :--- | :--- |
| **`160 x 90`** | 16:9 | `(159, 89)` | **`640 x 360`** | **Default Widescreen**: Clean modern 16:9 aspect ratio. |
| **`128 x 128`** | 1:1 | `(127, 127)` | **`512 x 512`** | **PICO-8 Fantasy Console**: Top-down RPGs, arcade shooters, puzzle games. |
| **`160 x 120`** | 4:3 | `(159, 119)` | **`640 x 480`** | **Retro CRT / QQVGA**: Classic standard TV resolution; leaves headroom for off-screen spawning. |
| **`160 x 144`** | 10:9 | `(159, 143)` | **`640 x 576`** | **Nintendo Game Boy / GBC**: Classic handheld action, platformers, and RPGs. |
| **`240 x 136`** | ~16:9 | `(239, 135)` | **`960 x 544`** | **TIC-80 Fantasy Console**: Widescreen retro; scales cleanly onto 16:9 monitors without thick sidebars. |
| **`256 x 192`** | 4:3 | `(255, 191)` | **`1024 x 768`** | **ZX Spectrum / MSX / Nintendo DS**: Maximizes screen detail across full 1-byte X width (`0` denotes 256). |
| **`256 x 224`** | 8:7 | `(255, 223)` | **`1024 x 896`** | **NES / SNES**: Authentic 16-bit console proportions utilizing full 1-byte coordinate range. |

#### Predefined Resolution Constants in `canvas.e8085`:
```e8085
; Default Widescreen (16:9) -> 640 x 360 physical
%define RES_DEFAULT_W        160
%define RES_DEFAULT_H        90

; PICO-8 Fantasy Console (1:1) -> 512 x 512 physical
%define RES_PICO8_W          128
%define RES_PICO8_H          128

; Retro CRT / QQVGA (4:3) -> 640 x 480 physical
%define RES_CRT_W            160
%define RES_CRT_H            120

; Nintendo Game Boy (10:9) -> 640 x 576 physical
%define RES_GAMEBOY_W        160
%define RES_GAMEBOY_H        144

; TIC-80 Widescreen (~16:9) -> 960 x 544 physical
%define RES_WIDESCREEN_W     240
%define RES_WIDESCREEN_H     136

; ZX Spectrum / MSX (4:3) -> 1024 x 768 physical (0 encodes 256 in 8-bit)
%define RES_SPECTRUM_W       0
%define RES_SPECTRUM_H       192

; NES / SNES (8:7) -> 1024 x 896 physical (0 encodes 256 in 8-bit)
%define RES_NES_W            0
%define RES_NES_H            224
```

---

## 2. Hardware I/O Port Protocol

The 8085 communicates with the Canvas Device via 3 dedicated hardware I/O ports:

| Port | Symbol | Direction | Purpose |
| :--- | :--- | :--- | :--- |
| `0x10` | `CANVAS_CMD_PORT` | OUT | Write command identifier |
| `0x11` | `CANVAS_DATA_PORT` | IN / OUT | Stream parameter bytes and read return values |
| `0x12` | `CANVAS_STATUS_PORT` | IN | Read hardware and window status flags |

### 2.1 Status Register (`0x12`) Bit Flags
- **Bit 0 (`0x01`) - `STATUS_WINDOW_OPEN`**: `1` if window is open and active, `0` if user closed window.
- **Bit 1 (`0x02`) - `STATUS_FRAME_READY`**: `1` when frame time elapsed, `0` otherwise.
- **Bit 2 (`0x04`) - `STATUS_KEY_AVAILABLE`**: `1` if keyboard event queue is non-empty.
- **Bit 3 (`0x08`) - `STATUS_MOUSE_DOWN`**: `1` if any mouse button is currently held down, `0` otherwise.

### 2.2 Streaming Protocol (Approach 1)
Every macro or subroutine invocation communicates directly with the hardware:
1. Writes the command byte to `CANVAS_CMD_PORT` (`0x10`).
2. Streams parameter bytes sequentially to `CANVAS_DATA_PORT` (`0x11`).
3. For query operations, reads result bytes from `CANVAS_DATA_PORT` (`0x11`).
No intermediary parameter buffers or memory blocks in 8085 RAM are required.

---

## 3. Command Codes Table

| Command Code | Identifier | Parameters Streamed (OUT 0x11) | Return Value (IN 0x11) | Description |
| :--- | :--- | :--- | :--- | :--- |
| `0x01` | `CMD_INIT_CANVAS` | `width` (u8), `height` (u8), `title_len` (u8), `title_bytes...` | None | Initialize window with logical dimensions and title; fixes 4x4 scaling and window geometry |
| `0x02` | `CMD_DISPOSE_CANVAS` | None | None | Closes canvas window and frees resources |
| `0x03` | `CMD_GET_CANVAS_SIZE`| None | `width` (u8), `height` (u8) | Read active logical canvas dimensions |
| `0x05` | `CMD_GET_CANVAS_TITLE`| None | `title_len` (u8), `title_bytes...` | Read current canvas title into CPU |
| `0x06` | `CMD_SET_CANVAS_TITLE`| `title_len` (u8), `title_bytes...` | None | Update canvas window title |
| `0x07` | `CMD_UPDATE_CANVAS` | None | None | Blit backbuffer to window, poll events, sync FPS |
| `0x08` | `CMD_IS_CANVAS_OPEN` | None | `is_open` (u8: 1 or 0) | Query whether window is open |
| `0x09` | `CMD_GET_FPS` | None | `fps` (u8) | Read target FPS |
| `0x0A` | `CMD_SET_FPS` | `fps` (u8) | None | Set target FPS (e.g. 30, 60) |
| `0x0B` | `CMD_GET_DELTA` | None | `delta_ms` (u8) | Read elapsed milliseconds since previous frame |
| `0x10` | `CMD_CLEAR` | `color_hi` (u8), `color_lo` (u8) | None | Clear entire canvas with RGBA4444 color |
| `0x11` | `CMD_SET_PIXEL` | `x` (u8), `y` (u8), `color_hi` (u8), `color_lo` (u8) | None | Set single pixel at $(x, y)$ |
| `0x12` | `CMD_GET_PIXEL` | `x` (u8), `y` (u8) | `color_hi` (u8), `color_lo` (u8) | Read RGBA4444 color at $(x, y)$ |
| `0x13` | `CMD_LINE` | `x1` (u8), `y1` (u8), `x2` (u8), `y2` (u8), `thickness` (u8), `color_hi` (u8), `color_lo` (u8) | None | Draw straight line |
| `0x14` | `CMD_FILL_RECT` | `x` (u8), `y` (u8), `w` (u8), `h` (u8), `color_hi` (u8), `color_lo` (u8) | None | Draw filled rectangle |
| `0x15` | `CMD_STROKE_RECT` | `x` (u8), `y` (u8), `w` (u8), `h` (u8), `thickness` (u8), `color_hi` (u8), `color_lo` (u8) | None | Draw outlined rectangle |
| `0x16` | `CMD_FILL_ELLIPSE` | `cx` (u8), `cy` (u8), `rx` (u8), `ry` (u8), `color_hi` (u8), `color_lo` (u8) | None | Draw filled ellipse / circle |
| `0x17` | `CMD_STROKE_ELLIPSE`| `cx` (u8), `cy` (u8), `rx` (u8), `ry` (u8), `thickness` (u8), `color_hi` (u8), `color_lo` (u8) | None | Draw outlined ellipse / circle |
| `0x18` | `CMD_PAINT_MASK` | `x` (u8), `y` (u8), `w` (u8), `h` (u8), `mask_val` (u8), `invert` (u8), `color_hi` (u8), `color_lo` (u8), `mask_bytes...` | None | Draw multi-valued sprite mask with transparent 0x00 and optional invert |
| `0x20` | `CMD_FILL_TEXT` | `x` (u8), `y` (u8), `size` (u8), `color_hi` (u8), `color_lo` (u8), `len` (u8), `text_bytes...` | None | Render filled text string using built-in font |
| `0x21` | `CMD_TEXT_WIDTH` | `size` (u8), `len` (u8), `text_bytes...` | `width` (u8) | Calculate rendered pixel width of string |
| `0x22` | `CMD_STROKE_TEXT` | `x` (u8), `y` (u8), `size` (u8), `thickness` (u8), `color_hi` (u8), `color_lo` (u8), `len` (u8), `text_bytes...` | None | Render outlined/stroke text string |
| `0x30` | `CMD_KEY_PRESS` | `key_code` (u8) | `is_pressed` (u8: 1 or 0) | 1 if key transitioned to pressed this frame |
| `0x31` | `CMD_KEY_DOWN` | `key_code` (u8) | `is_down` (u8: 1 or 0) | 1 if key is currently held down |
| `0x32` | `CMD_KEY_RELEASE` | `key_code` (u8) | `is_released` (u8: 1 or 0) | 1 if key was released this frame |
| `0x33` | `CMD_KEY_INPUT` | None | `key_code` (u8, 0 if none) | Pop next key event from input queue |
| `0x40` | `CMD_MOUSE_PRESS` | `mousebutton` (u8) | `is_pressed` (u8: 1 or 0) | 1 if button transitioned to pressed this frame |
| `0x41` | `CMD_MOUSE_DOWN` | `mousebutton` (u8) | `is_down` (u8: 1 or 0) | 1 if button is currently held down |
| `0x42` | `CMD_MOUSE_RELEASE` | `mousebutton` (u8) | `is_released` (u8: 1 or 0) | 1 if button was released this frame |
| `0x43` | `CMD_MOUSE_POS` | None | `x` (u8), `y` (u8) | Read logical mouse cursor coordinate (B=X, C=Y) |

---

## 4. API Catalog: Macros & Subroutines

Canvas macros provide a clean, high-level interface that loads parameters into 8085 register pairs and calls shared library subroutines. This eliminates inline I/O streaming duplication, shrinking call-site binary size by 50% to 64%.

### 4.1 Canvas Lifecycle
- **`InitCanvas(w: imm8, h: imm8, title: addr)`**:
  - Initializes canvas window with logical dimensions (`w`, `h`) and title string.
  - Setup: `B = w`, `C = h`, `HL = title`. Calls `canvas_init`.
- **`DisposeCanvas()`**:
  - Closes the active window and frees all host resources.
- **`GetCanvasSize()`**:
  - Reads active logical dimensions into registers (`B = width`, `C = height`). Calls `canvas_get_size`.
- **`GetCanvasTitle(buf: addr)`**:
  - Copies window title into memory buffer pointed to by `HL`. Calls `canvas_get_title`.
- **`SetCanvasTitle(str: addr)`**:
  - Updates window title from null-terminated string at `HL`. Calls `canvas_set_title`.

### 4.2 GameLoop & Timing
- **`UpdateCanvas()`**:
  - Flushes backbuffer to the physical display window, polls OS events, and throttles frame rate.
- **`IsCanvasOpen()`**:
  - Returns `1` in register `A` if window is open; `0` if user closed window. Sets `Z` flag (`jz exit`).
- **`GetFPS()`**:
  - Returns current target FPS in register `A`.
- **`SetFPS(fps: imm8)`**:
  - Sets target frame rate (default: `30` FPS).
- **`GetDelta()`**:
  - Returns elapsed milliseconds since previous frame in register `A`.

### 4.3 2D Drawing Primitives
All coordinates are 8-bit (`0` to `255`). Colors are 16-bit RGBA4444 values loaded via `LXI`.

- **`Clear(color: Color)`**:
  - Fills entire logical canvas with 16-bit color.
  - Setup: `HL = color`. Calls `canvas_clear`.
- **`SetPixel(x: imm8, y: imm8, color: Color)`**:
  - Sets pixel at $(x, y)$ to 16-bit color.
  - Setup: `B = x`, `C = y`, `HL = color`. Calls `canvas_set_pixel`.
- **`GetPixel(x: imm8, y: imm8)`**:
  - Reads 16-bit color of pixel $(x, y)$ into register pair `BC` (`B = color_hi`, `C = color_lo`).
  - Setup: `B = x`, `C = y`. Calls `canvas_get_pixel`.
- **`FillRect(x: imm8, y: imm8, w: imm8, h: imm8, color: Color)`**:
  - Draws a solid filled rectangle with top-left corner at $(x, y)$ of dimensions $(w, h)$.
  - Setup: `B = x`, `C = y`, `D = w`, `E = h`, `HL = color`. Calls `canvas_draw_rect`.
- **`StrokeRect(x: imm8, y: imm8, w: imm8, h: imm8, color: Color)`**:
  - Draws an outlined rectangle of dimensions $(w, h)$ with 1px border.
  - Setup: `B = x`, `C = y`, `D = w`, `E = h`, `HL = color`. Calls `canvas_stroke_rect_1`.
- **`FillCircle(cx: imm8, cy: imm8, r: imm8, color: Color)`**:
  - Draws a solid filled circle centered at $(cx, cy)$ with radius $r$.
  - Setup: `B = cx`, `C = cy`, `D = r`, `E = r`, `HL = color`. Calls `canvas_draw_ellipse`.
- **`StrokeCircle(cx: imm8, cy: imm8, r: imm8, color: Color)`**:
  - Draws an outlined circle centered at $(cx, cy)$ with radius $r$ and 1px border.
  - Setup: `B = cx`, `C = cy`, `D = r`, `E = r`, `HL = color`. Calls `canvas_stroke_circle_1`.
- **`Line(x1: imm8, y1: imm8, x2: imm8, y2: imm8, color: Color)`**:
  - Draws a 1px straight line between $(x1, y1)$ and $(x2, y2)$.
  - Setup: `B = x1`, `C = y1`, `D = x2`, `E = y2`, `HL = color`. Calls `canvas_draw_line_1`.
- **`DrawHLine(x: imm8, y: imm8, len: imm8, color: Color)`**:
  - Fast horizontal line from $(x, y)$ with length `len`.
  - Setup: `B = x`, `C = y`, `D = len`, `HL = color`. Calls `canvas_draw_hline`.
- **`DrawVLine(x: imm8, y: imm8, len: imm8, color: Color)`**:
  - Fast vertical line from $(x, y)$ with length `len`.
  - Setup: `B = x`, `C = y`, `E = len`, `HL = color`. Calls `canvas_draw_vline`.
- **`DrawSprite(x: imm8, y: imm8, w: imm8, h: imm8, mask: addr, color: Color)`**:
  - Rasterizes a 2D masked sprite from memory at `mask`.
  - Setup: `B = x`, `C = y`, `D = w`, `E = h`, `HL = mask`, color in static storage. Calls `canvas_draw_sprite`.

### 4.4 Text Rendering
- **`DrawText(str: addr, x: imm8, y: imm8, color: Color)`**:
  - Renders solid text string at $(x, y)$ with scale 1.
  - Setup: `HL = str`, `B = x`, `C = y`, `DE = color`. Calls `canvas_draw_text_1`.
- **`TextWidth(str: addr, scale: imm8)`**:
  - Computes rendered pixel width of string; returns width in register `A`.
  - Setup: `HL = str`, `A = scale`. Calls `canvas_text_width`.

### 4.5 Keyboard Input
- **`KeyPress(key: imm8)`**:
  - Checks if key transitioned to pressed this frame. Returns `1` in `A` if true, `0` if false (`jnz pressed`).
- **`KeyDown(key: imm8)`**:
  - Checks if key is currently held down. Returns `1` in `A` if true, `0` if false.
- **`KeyRelease(key: imm8)`**:
  - Checks if key was released this frame. Returns `1` in `A` if true, `0` if false.
- **`KeyInput()`**:
  - Returns next key code from event queue in `A`, or `0` if empty.

### 4.6 Mouse Input
- **`MousePress(button: imm8)`**:
  - Checks if mouse button transitioned to pressed this frame. Returns `1` in `A` if true, `0` if false.
- **`MouseDown(button: imm8)`**:
  - Checks if mouse button is currently held down. Returns `1` in `A` if true, `0` if false.
- **`MouseRelease(button: imm8)`**:
  - Checks if mouse button was released this frame. Returns `1` in `A` if true, `0` if false.
- **`MousePosition()`**:
  - Loads logical cursor $X$ into register `B`, and logical $Y$ into register `C`.

---

## 5. Register Calling Conventions

Subroutines use Intel 8085 register pairs for zero-overhead parameter passing:

| Register / Pair | Common Role | Preserved by Subroutines |
| :--- | :--- | :--- |
| **`BC`** | `B = x` (or `x1`), `C = y` (or `y1`) | Yes (except `get_pixel`, `get_size`, `mouse_pos`) |
| **`DE`** | `D = w`, `E = h` (or `D = x2`, `E = y2` or `color` in `draw_text`) | Yes |
| **`HL`** | `H = color_hi`, `L = color_lo` (or pointer to string / mask) | Yes |
| **`A`** | Command byte, thickness, scale, query results | Return value / alters |

#### Code Size Comparison:
- **Legacy Inline Streaming**: 28 bytes per rectangle call site.
- **Subroutine-backed Macro**: 14 bytes per rectangle call site (50% reduction).

---

## 6. Key Codes Reference Table

All key codes are 8-bit constants (`%define K_...`):

### Letters & Numbers
| Constant | Hex Code | Description | Constant | Hex Code | Description |
| :--- | :--- | :--- | :--- | :--- | :--- |
| `K_A` .. `K_Z` | `0x41` .. `0x5A` | Standard ASCII 'A'–'Z' | `K_0` .. `K_9` | `0x30` .. `0x39` | Top row digits '0'–'9' |

### Function Keys
| Constant | Hex Code | Description | Constant | Hex Code | Description |
| :--- | :--- | :--- | :--- | :--- | :--- |
| `K_F1` .. `K_F6` | `0x81` .. `0x86` | Function Keys F1–F6 | `K_F7` .. `K_F12` | `0x87` .. `0x8C` | Function Keys F7–F12 |

### Navigation & Arrow Keys
| Constant | Hex Code | Description | Constant | Hex Code | Description |
| :--- | :--- | :--- | :--- | :--- | :--- |
| `K_ARROW_UP` | `0x90` | Up Arrow | `K_HOME` | `0x94` | Home Key |
| `K_ARROW_DOWN` | `0x91` | Down Arrow | `K_END` | `0x95` | End Key |
| `K_ARROW_LEFT` | `0x92` | Left Arrow | `K_PAGE_UP` | `0x96` | Page Up |
| `K_ARROW_RIGHT`| `0x93` | Right Arrow | `K_PAGE_DOWN` | `0x97` | Page Down |

### Controls & Modifiers
| Constant | Hex Code | Description | Constant | Hex Code | Description |
| :--- | :--- | :--- | :--- | :--- | :--- |
| `K_ESCAPE` | `0x1B` | Escape Key | `K_ENTER` | `0x0D` | Enter / Return |
| `K_SPACE` | `0x20` | Spacebar | `K_BACKSPACE`| `0x08` | Backspace |
| `K_TAB` | `0x09` | Tab Key | `K_DELETE` | `0x7F` | Delete |
| `K_SHIFT` | `0xA0` | Shift (Left/Right) | `K_CTRL` | `0xA1` | Control Key |
| `K_ALT` | `0xA2` | Alt / Option Key | `K_CAPS_LOCK`| `0xA3` | Caps Lock |

### Punctuation & Symbols
| Constant | Hex Code | Character | Constant | Hex Code | Character |
| :--- | :--- | :--- | :--- | :--- | :--- |
| `K_MINUS` | `0x2D` | `-` | `K_EQUALS` | `0x3D` | `=` |
| `K_LEFT_BRACKET` | `0x5B` | `[` | `K_RIGHT_BRACKET` | `0x5D` | `]` |
| `K_BACKSLASH` | `0x5C` | `\` | `K_SEMICOLON` | `0x3B` | `;` |
| `K_QUOTE` | `0x27` | `'` | `K_GRAVE` | `0x60` | `` ` `` |
| `K_COMMA` | `0x2C` | `,` | `K_PERIOD` | `0x2E` | `.` |
| `K_SLASH` | `0x2F` | `/` | | | |

---

## 7. Mouse Buttons Reference Table

Mouse button identifiers are 1-byte constants passed to `MousePress`, `MouseDown`, `MouseRelease`, and their subroutine equivalents:

| Constant | Value | Description |
| :--- | :--- | :--- |
| `MOUSE_BUTTON_LEFT` | `0` | Primary (Left) mouse button |
| `MOUSE_BUTTON_RIGHT` | `1` | Secondary (Right) mouse button |
| `MOUSE_BUTTON_MIDDLE` | `2` | Middle mouse button / scroll wheel click |

---

## 8. Example Game Loop Program

Below is a complete, idiomatic 8085 assembly game loop demonstrating the Canvas API:

```e8085
%include "devices/canvas.e8085"

segment .data
title_str byte "8085 Pong Game\0"

segment .bss
paddle_y byte 1

segment .text
main:
    ; 1. Initialize canvas using preset constants (160x90 scaled 4x4 to 640x360 window)
    InitCanvas RES_DEFAULT_W, RES_DEFAULT_H, title_str
    SetFPS 30

    ; Initial paddle position
    mvi A, 40
    sta paddle_y

game_loop:
    ; 2. Check if window is still open
    IsCanvasOpen
    jz game_exit

    ; 3. Handle Keyboard Input (Move Paddle)
    KeyDown K_ARROW_UP
    jz .check_down
    lda paddle_y
    dcr A
    sta paddle_y

.check_down:
    KeyDown K_ARROW_DOWN
    jz .render
    lda paddle_y
    inr A
    sta paddle_y

.render:
    ; 4. Clear Canvas (Dark Navy Blue)
    Clear 0x00, 0x2F

    ; 5. Draw Paddle (White Rectangle: x=10, y=paddle_y, w=4, h=16)
    lda paddle_y
    mov B, A
    FillRectangle 10, 40, 4, 16, COLOR_WHITE_HI, COLOR_WHITE_LO

    ; 6. Draw Ball (Yellow Ellipse: cx=80, cy=45, rx=3, ry=3)
    FillEllipse 80, 45, 3, 3, COLOR_YELLOW_HI, COLOR_YELLOW_LO

    ; 7. Swap Buffer, Poll Events, and Throttle to 30 FPS
    UpdateCanvas
    jmp game_loop

game_exit:
    DisposeCanvas
    hlt
```
