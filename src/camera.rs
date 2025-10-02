// Camera frame encoding for fractal mind training
// Converts video frames into string representations that hash to Mandelbrot coordinates
//
// Note: Camera capture requires system access to /dev/video*.
// On WSL2, ensure USB passthrough is configured for webcams.

/// Encoding strategy for camera frames
#[derive(Clone, Copy, Debug)]
pub enum FrameEncoding {
    /// Downsampled pixel grid (e.g., "128,45,200,130,46,198,...")
    PixelGrid { width: usize, height: usize },

    /// Edge detection symbols (e.g., "|||--|||---||")
    EdgeSymbols,

    /// Average color blocks (e.g., "rgb:128,45,200|rgb:130,46,198|...")
    ColorBlocks { blocks_x: usize, blocks_y: usize },

    /// Motion vectors between frames (e.g., "mv:+2,-3,+1,+5")
    MotionVectors,
}

/// Captures frames from camera and encodes them as strings
/// Currently a stub - requires v4l2/libclang dependencies for real camera access
pub struct CameraEncoder {
    encoding: FrameEncoding,
}

impl CameraEncoder {
    /// Initialize camera encoder with specified encoding strategy
    pub fn new(encoding: FrameEncoding) -> Result<Self, Box<dyn std::error::Error>> {
        // TODO: Implement camera capture using v4l2 bindings or ffmpeg
        // For now, return error with helpful message
        Err(format!(
            "Camera capture not yet implemented.\n\
             To use camera:\n\
             1. Extract frames with: ffmpeg -i /dev/video0 -r 10 frame_%04d.png\n\
             2. Use /image command to process frames\n\
             \n\
             Encoding strategy: {:?}",
            encoding
        ).into())
    }

    /// Capture and encode a single frame
    pub fn capture_frame(&mut self) -> Result<String, Box<dyn std::error::Error>> {
        Err("Camera capture not implemented - use ffmpeg to extract frames".into())
    }
}

/// Downsample frame to grid and encode as comma-separated RGB values
fn encode_pixel_grid(buffer: &[u8], width: usize, height: usize, target_w: usize, target_h: usize) -> String {
    let mut output = String::with_capacity(target_w * target_h * 12);

    let step_x = width / target_w;
    let step_y = height / target_h;

    for y in 0..target_h {
        for x in 0..target_w {
            let src_x = (x * step_x).min(width - 1);
            let src_y = (y * step_y).min(height - 1);
            let idx = (src_y * width + src_x) * 3;

            if idx + 2 < buffer.len() {
                let r = buffer[idx];
                let g = buffer[idx + 1];
                let b = buffer[idx + 2];
                output.push_str(&format!("{},{},{},", r, g, b));
            }
        }
    }

    output
}

/// Detect edges using simple gradient and encode as symbols
fn encode_edges(buffer: &[u8], width: usize, height: usize) -> String {
    let mut output = String::with_capacity(width * height / 100);

    let step = 8; // Sample every 8 pixels

    for y in (step..height - step).step_by(step) {
        for x in (step..width - step).step_by(step) {
            let idx = (y * width + x) * 3;

            if idx + 3 < buffer.len() {
                let center = buffer[idx] as i32;
                let right_idx = (y * width + x + step) * 3;
                let down_idx = ((y + step) * width + x) * 3;

                if right_idx + 2 < buffer.len() && down_idx + 2 < buffer.len() {
                    let right = buffer[right_idx] as i32;
                    let down = buffer[down_idx] as i32;

                    let gradient_x = (right - center).abs();
                    let gradient_y = (down - center).abs();
                    let gradient = gradient_x + gradient_y;

                    let symbol = match gradient {
                        0..=20 => ' ',
                        21..=50 => '.',
                        51..=100 => '-',
                        101..=150 => '|',
                        _ => '#',
                    };
                    output.push(symbol);
                }
            }
        }
        output.push('\n');
    }

    output
}

/// Divide frame into blocks and encode average color
fn encode_color_blocks(buffer: &[u8], width: usize, height: usize, blocks_x: usize, blocks_y: usize) -> String {
    let mut output = String::with_capacity(blocks_x * blocks_y * 20);

    let block_w = width / blocks_x;
    let block_h = height / blocks_y;

    for by in 0..blocks_y {
        for bx in 0..blocks_x {
            let mut r_sum = 0u32;
            let mut g_sum = 0u32;
            let mut b_sum = 0u32;
            let mut count = 0u32;

            for y in (by * block_h)..((by + 1) * block_h).min(height) {
                for x in (bx * block_w)..((bx + 1) * block_w).min(width) {
                    let idx = (y * width + x) * 3;
                    if idx + 2 < buffer.len() {
                        r_sum += buffer[idx] as u32;
                        g_sum += buffer[idx + 1] as u32;
                        b_sum += buffer[idx + 2] as u32;
                        count += 1;
                    }
                }
            }

            if count > 0 {
                let r_avg = r_sum / count;
                let g_avg = g_sum / count;
                let b_avg = b_sum / count;
                output.push_str(&format!("rgb:{},{},{}|", r_avg, g_avg, b_avg));
            }
        }
    }

    output
}

/// Encode motion between frames as directional vectors
fn encode_motion_vectors(buffer: &[u8], last_frame: &Option<Vec<u8>>, width: usize, height: usize) -> String {
    let Some(prev) = last_frame else {
        return "mv:init".to_string();
    };

    if prev.len() != buffer.len() {
        return "mv:reset".to_string();
    }

    let mut output = String::from("mv:");
    let step = 16; // Compare 16x16 blocks

    for y in (0..height - step).step_by(step) {
        for x in (0..width - step).step_by(step) {
            let idx = (y * width + x) * 3;

            if idx + 2 < buffer.len() && idx + 2 < prev.len() {
                let diff_r = buffer[idx] as i32 - prev[idx] as i32;
                let diff_g = buffer[idx + 1] as i32 - prev[idx + 1] as i32;
                let diff_b = buffer[idx + 2] as i32 - prev[idx + 2] as i32;

                let motion = (diff_r.abs() + diff_g.abs() + diff_b.abs()) / 3;

                let symbol = match motion {
                    0..=10 => '.',
                    11..=30 => '+',
                    31..=60 => '*',
                    _ => '#',
                };
                output.push(symbol);
            }
        }
    }

    output
}
