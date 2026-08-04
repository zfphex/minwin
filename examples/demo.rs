use minwin::*;

fn main() {
    println!("Initializing Window...");
    let mut window = create_window("Demo", None, 800, 600, false, WindowStyle::Standard);

    println!("Demo Controls:");
    println!("  [C] - Cycle cursor icon (Arrow -> IBeam -> PointingHand -> Crosshair)");
    println!("  [V] - Toggle cursor visibility");
    println!("  [G] - Toggle cursor grab/lock");
    println!("  [P] - Copy 'Hello from minwin!' to clipboard");
    println!("  [O] - Read and print clipboard contents");
    println!("  [T] - Change window title");
    println!("  [Drop Files] - Drop files onto the window to print their paths");

    let mut frame_count = 0;
    let mut cursor_index = 0;
    let mut title_index = 0;
    let titles = ["Demo - Title A", "Demo - Title B", "Demo - Title C"];
    let cursor_icons = [
        CursorIcon::Arrow,
        CursorIcon::IBeam,
        CursorIcon::PointingHand,
        CursorIcon::Crosshair,
        CursorIcon::ResizeLeftRight,
        CursorIcon::ResizeUpDown,
    ];

    let mut cursor_visible = true;
    let mut cursor_grabbed = false;

    while window.open() {
        window.draw(|win| {
            let (w, h) = win.scaled_size();
            let scale = win.scale_factor() as f32;
            let w = (w as f32 * scale).round() as usize;
            let h = (h as f32 * scale).round() as usize;
            let frame = &mut frame_count;
            let pixels = win.framebuffer();

            for y in 0..h {
                for x in 0..w {
                    let r = ((x + *frame) & 0xFF) as u32;
                    let g = ((y + *frame) & 0xFF) as u32;
                    let b = (*frame & 0xFF) as u32;
                    pixels[y * w + x] = 0xFF000000 | (r << 16) | (g << 8) | b;
                }
            }
            *frame += 2;

            win.present();
        });

        if window.pressed(Key::Escape) {
            window.close();
        }

        for &c in window.text_input() {
            println!("Character Received: {:?}", c);
        }

        for &key in window.pressed_keys() {
            match key {
                Key::Char('C') => {
                    cursor_index = (cursor_index + 1) % cursor_icons.len();
                    let icon = cursor_icons[cursor_index];
                    window.set_cursor_icon(icon);
                    println!("Set cursor shape to {:?}", icon);
                }
                Key::Char('V') => {
                    cursor_visible = !cursor_visible;
                    window.set_cursor_visible(cursor_visible);
                    println!("Set cursor visibility: {}", cursor_visible);
                }
                Key::Char('G') => {
                    cursor_grabbed = !cursor_grabbed;
                    window.set_cursor_grab(cursor_grabbed);
                    println!("Set cursor grab: {}", cursor_grabbed);
                }
                Key::Char('P') => {
                    let text = "Hello from minwin Clipboard!";
                    window.set_clipboard_text(text);
                    println!("Copied text to clipboard: {:?}", text);
                }
                Key::Char('O') => {
                    if let Some(text) = window.get_clipboard_text() {
                        println!("Read text from clipboard: {:?}", text);
                    }
                }
                Key::Char('T') => {
                    title_index = (title_index + 1) % titles.len();
                    let new_title = titles[title_index];
                    window.set_title(new_title);
                    println!("Set title to {:?}", new_title);
                }
                _ => {}
            }
        }

        let raw_delta = window.raw_mouse_delta();
        if raw_delta != (0.0, 0.0) {
            println!("Raw mouse delta: {:?}", raw_delta);
        }

        if !window.dropped_files().is_empty() {
            println!("Dropped Files:");
            for file in window.dropped_files() {
                println!("  - {:?}", file);
            }
        }

        window.wait_for_vsync();
    }
}
