use minifb::{Window, WindowOptions, Key};
use crate::{FractalMind, julia_escapes, C};
use std::sync::{Arc, Mutex};
use std::thread;

const WIDTH: usize = 600;
const HEIGHT: usize = 600;

// Simple 3x5 pixel font
fn draw_char(buffer: &mut [u32], x: usize, y: usize, ch: char, color: u32) {
    let pattern: &[u8] = match ch {
        '0' => &[0b111, 0b101, 0b101, 0b101, 0b111],
        '1' => &[0b010, 0b110, 0b010, 0b010, 0b111],
        '2' => &[0b111, 0b001, 0b111, 0b100, 0b111],
        '3' => &[0b111, 0b001, 0b111, 0b001, 0b111],
        '4' => &[0b101, 0b101, 0b111, 0b001, 0b001],
        '5' => &[0b111, 0b100, 0b111, 0b001, 0b111],
        '6' => &[0b111, 0b100, 0b111, 0b101, 0b111],
        '7' => &[0b111, 0b001, 0b001, 0b001, 0b001],
        '8' => &[0b111, 0b101, 0b111, 0b101, 0b111],
        '9' => &[0b111, 0b101, 0b111, 0b001, 0b111],
        '.' => &[0b000, 0b000, 0b000, 0b000, 0b010],
        '-' => &[0b000, 0b000, 0b111, 0b000, 0b000],
        ':' => &[0b000, 0b010, 0b000, 0b010, 0b000],
        ' ' => &[0b000, 0b000, 0b000, 0b000, 0b000],
        'a' => &[0b111, 0b101, 0b111, 0b101, 0b101],
        'b' => &[0b110, 0b101, 0b110, 0b101, 0b110],
        'c' => &[0b111, 0b100, 0b100, 0b100, 0b111],
        'd' => &[0b110, 0b101, 0b101, 0b101, 0b110],
        'e' => &[0b111, 0b100, 0b111, 0b100, 0b111],
        'f' => &[0b111, 0b100, 0b111, 0b100, 0b100],
        'g' => &[0b111, 0b100, 0b101, 0b101, 0b111],
        'h' => &[0b101, 0b101, 0b111, 0b101, 0b101],
        'i' => &[0b111, 0b010, 0b010, 0b010, 0b111],
        'j' => &[0b001, 0b001, 0b001, 0b101, 0b111],
        'l' => &[0b100, 0b100, 0b100, 0b100, 0b111],
        'm' => &[0b101, 0b111, 0b111, 0b101, 0b101],
        'n' => &[0b101, 0b111, 0b111, 0b111, 0b101],
        'o' => &[0b111, 0b101, 0b101, 0b101, 0b111],
        'p' => &[0b111, 0b101, 0b111, 0b100, 0b100],
        'r' => &[0b110, 0b101, 0b110, 0b101, 0b101],
        's' => &[0b111, 0b100, 0b111, 0b001, 0b111],
        't' => &[0b111, 0b010, 0b010, 0b010, 0b010],
        'y' => &[0b101, 0b101, 0b111, 0b010, 0b010],
        _ => &[0b000, 0b000, 0b000, 0b000, 0b000],
    };

    for (dy, &row) in pattern.iter().enumerate() {
        if y + dy >= HEIGHT { break; }
        for dx in 0..3 {
            if x + dx >= WIDTH { break; }
            if row & (1 << (2 - dx)) != 0 {
                let idx = (y + dy) * WIDTH + (x + dx);
                buffer[idx] = color;
            }
        }
    }
}

fn draw_text(buffer: &mut [u32], x: usize, y: usize, text: &str, color: u32) {
    let mut offset_x = x;
    for ch in text.chars() {
        if offset_x + 4 >= WIDTH { break; }
        draw_char(buffer, offset_x, y, ch, color);
        offset_x += 4;
    }
}

pub fn spawn_visualizer(mind: Arc<Mutex<FractalMind>>) {
    thread::spawn(move || {
        let mut window = Window::new(
            "Fractal Mind — Live Julia Set",
            WIDTH,
            HEIGHT,
            WindowOptions::default(),
        ).unwrap();

        window.set_target_fps(30);

        let mut buffer: Vec<u32> = vec![0; WIDTH * HEIGHT];

        while window.is_open() && !window.is_key_down(Key::Escape) {
            let (cx, cy, symbols, trajectories, fields) = {
                let mind = mind.lock().unwrap();
                (
                    mind.current_coord.re,
                    mind.current_coord.im,
                    mind.symbols.len(),
                    mind.trajectories.len(),
                    mind.associative_fields.len(),
                )
            };

            // Render Julia set at current mind position
            render_julia(&mut buffer, cx, cy);

            // Draw metrics overlay
            let text_color = 0xFFFFFF;
            draw_text(&mut buffer, 10, 10, &format!("c: {:.3} {:.3}i", cx, cy), text_color);
            draw_text(&mut buffer, 10, 25, &format!("symbols: {}", symbols), text_color);
            draw_text(&mut buffer, 10, 40, &format!("trajectories: {}", trajectories), text_color);
            draw_text(&mut buffer, 10, 55, &format!("fields: {}", fields), text_color);

            window.update_with_buffer(&buffer, WIDTH, HEIGHT).unwrap();
        }
    });
}

fn render_julia(buffer: &mut [u32], cx: f64, cy: f64) {
    let zoom = 1.5;
    let c = C { re: cx, im: cy };

    for y in 0..HEIGHT {
        for x in 0..WIDTH {
            let zx = (x as f64 / WIDTH as f64 - 0.5) * zoom * 2.0;
            let zy = (y as f64 / HEIGHT as f64 - 0.5) * zoom * 2.0;
            let z0 = C { re: zx, im: zy };

            let iter = julia_escapes(z0, c, 256);

            let color = if iter < 256 {
                // Escaped - color by iteration count
                let t = iter as f64 / 256.0;
                let r = ((t * 255.0).sin().abs() * 255.0) as u32;
                let g = ((t * 180.0).cos().abs() * 200.0) as u32;
                let b = ((1.0 - t) * 150.0) as u32;
                (r << 16) | (g << 8) | b
            } else {
                // Inside set - deep blue/black
                0x001020
            };

            buffer[y * WIDTH + x] = color;
        }
    }
}
