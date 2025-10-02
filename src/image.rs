// Image and video file encoding for fractal mind training
// Converts static images and video files into string representations

use image::{DynamicImage, GenericImageView, imageops};
use std::path::Path;

/// Encoding strategy for images/video frames
#[derive(Clone, Copy)]
pub enum ImageEncoding {
    /// Downsampled pixel grid (similar to camera encoding)
    PixelGrid { width: usize, height: usize },

    /// Dominant colors extracted (e.g., "rgb:120,45,200|rgb:180,90,50|...")
    DominantColors { color_count: usize },

    /// Edge detection ASCII art
    EdgeAscii { width: usize, height: usize },

    /// Histogram-based encoding (frequency distribution)
    ColorHistogram { bins: usize },

    /// Spatial frequency encoding (texture patterns)
    TexturePattern { grid_size: usize },
}

/// Load and encode an image file
pub fn encode_image<P: AsRef<Path>>(
    path: P,
    encoding: ImageEncoding,
) -> Result<String, Box<dyn std::error::Error>> {
    let img = image::open(path)?;
    Ok(encode_image_data(&img, encoding))
}

/// Encode image data using specified strategy
pub fn encode_image_data(img: &DynamicImage, encoding: ImageEncoding) -> String {
    match encoding {
        ImageEncoding::PixelGrid { width, height } => {
            encode_pixel_grid_image(img, width, height)
        }
        ImageEncoding::DominantColors { color_count } => {
            encode_dominant_colors(img, color_count)
        }
        ImageEncoding::EdgeAscii { width, height } => {
            encode_edge_ascii(img, width, height)
        }
        ImageEncoding::ColorHistogram { bins } => {
            encode_color_histogram(img, bins)
        }
        ImageEncoding::TexturePattern { grid_size } => {
            encode_texture_pattern(img, grid_size)
        }
    }
}

/// Downsample image to grid and encode as comma-separated RGB values
fn encode_pixel_grid_image(img: &DynamicImage, target_w: usize, target_h: usize) -> String {
    let resized = img.resize_exact(target_w as u32, target_h as u32, imageops::FilterType::Nearest);
    let rgb = resized.to_rgb8();

    let mut output = String::with_capacity(target_w * target_h * 12);

    for pixel in rgb.pixels() {
        output.push_str(&format!("{},{},{},", pixel[0], pixel[1], pixel[2]));
    }

    output
}

/// Extract dominant colors using simple k-means-like clustering
fn encode_dominant_colors(img: &DynamicImage, color_count: usize) -> String {
    let rgb = img.to_rgb8();
    let (width, height) = img.dimensions();

    // Sample pixels uniformly
    let sample_step = ((width * height) / 1000).max(1);
    let mut samples = Vec::new();

    for y in (0..height).step_by(sample_step as usize) {
        for x in (0..width).step_by(sample_step as usize) {
            let pixel = rgb.get_pixel(x, y);
            samples.push([pixel[0] as u32, pixel[1] as u32, pixel[2] as u32]);
        }
    }

    if samples.is_empty() {
        return "colors:none".to_string();
    }

    // Simple clustering: divide color space into regions
    let mut clusters = Vec::new();
    let step = 256 / ((color_count as f32).cbrt().ceil() as u32);

    for r in (0..256).step_by(step as usize) {
        for g in (0..256).step_by(step as usize) {
            for b in (0..256).step_by(step as usize) {
                clusters.push(([r as u32, g as u32, b as u32], 0usize));
            }
        }
    }

    // Assign samples to nearest cluster
    for sample in &samples {
        let mut best_dist = u32::MAX;
        let mut best_idx = 0;

        for (i, (cluster, _)) in clusters.iter().enumerate() {
            let dist = color_distance(*sample, *cluster);
            if dist < best_dist {
                best_dist = dist;
                best_idx = i;
            }
        }

        clusters[best_idx].1 += 1;
    }

    // Sort by frequency and take top N
    clusters.sort_by(|a, b| b.1.cmp(&a.1));

    let mut output = String::from("colors:");
    for i in 0..color_count.min(clusters.len()) {
        let (color, _count) = clusters[i];
        output.push_str(&format!("rgb:{},{},{}|", color[0], color[1], color[2]));
    }

    output
}

/// Simple Euclidean distance between RGB colors
fn color_distance(a: [u32; 3], b: [u32; 3]) -> u32 {
    let dr = (a[0] as i32 - b[0] as i32).pow(2);
    let dg = (a[1] as i32 - b[1] as i32).pow(2);
    let db = (a[2] as i32 - b[2] as i32).pow(2);
    (dr + dg + db) as u32
}

/// Edge detection using gradient, output as ASCII art
fn encode_edge_ascii(img: &DynamicImage, target_w: usize, target_h: usize) -> String {
    let resized = img.resize_exact(target_w as u32, target_h as u32, imageops::FilterType::Triangle);
    let gray = resized.to_luma8();

    let mut output = String::with_capacity(target_w * target_h + target_h);

    for y in 0..target_h {
        for x in 0..target_w {
            let center = gray.get_pixel(x as u32, y as u32)[0] as i32;

            let right = if x + 1 < target_w {
                gray.get_pixel((x + 1) as u32, y as u32)[0] as i32
            } else {
                center
            };

            let down = if y + 1 < target_h {
                gray.get_pixel(x as u32, (y + 1) as u32)[0] as i32
            } else {
                center
            };

            let gradient = (right - center).abs() + (down - center).abs();

            let symbol = match gradient {
                0..=20 => ' ',
                21..=50 => '.',
                51..=100 => '-',
                101..=150 => '|',
                _ => '#',
            };
            output.push(symbol);
        }
        output.push('\n');
    }

    output
}

/// Encode color histogram (distribution of RGB values)
fn encode_color_histogram(img: &DynamicImage, bins: usize) -> String {
    let rgb = img.to_rgb8();
    let mut histogram = vec![0u32; bins];

    let bin_size = 256 / bins;

    for pixel in rgb.pixels() {
        let luminance = (pixel[0] as u32 + pixel[1] as u32 + pixel[2] as u32) / 3;
        let bin_idx = (luminance / bin_size as u32).min(bins as u32 - 1) as usize;
        histogram[bin_idx] += 1;
    }

    let mut output = String::from("hist:");
    for count in histogram {
        output.push_str(&format!("{},", count));
    }

    output
}

/// Encode texture patterns by analyzing spatial frequency
fn encode_texture_pattern(img: &DynamicImage, grid_size: usize) -> String {
    let gray = img.to_luma8();
    let (width, height) = gray.dimensions();

    let cell_w = width / grid_size as u32;
    let cell_h = height / grid_size as u32;

    let mut output = String::from("texture:");

    for gy in 0..grid_size {
        for gx in 0..grid_size {
            let x0 = gx as u32 * cell_w;
            let y0 = gy as u32 * cell_h;
            let x1 = ((gx + 1) as u32 * cell_w).min(width);
            let y1 = ((gy + 1) as u32 * cell_h).min(height);

            // Compute variance within cell (texture measure)
            let mut sum = 0u32;
            let mut sum_sq = 0u32;
            let mut count = 0u32;

            for y in y0..y1 {
                for x in x0..x1 {
                    let val = gray.get_pixel(x, y)[0] as u32;
                    sum += val;
                    sum_sq += val * val;
                    count += 1;
                }
            }

            let mean = sum / count;
            let variance = (sum_sq / count) - (mean * mean);

            let symbol = match variance {
                0..=100 => '.',
                101..=500 => '-',
                501..=1500 => '|',
                _ => '#',
            };
            output.push(symbol);
        }
    }

    output
}

/// Video file processing (reads frames sequentially)
pub struct VideoEncoder {
    // For now, this is a placeholder - full video decoding would require ffmpeg/gstreamer
    // You can implement frame extraction using external tools or libraries
}

impl VideoEncoder {
    /// Extract frames from video file and encode each
    /// Note: Requires external video decoding library (ffmpeg-next, gstreamer, etc.)
    pub fn encode_video_frames<P: AsRef<Path>>(
        _path: P,
        _encoding: ImageEncoding,
        _frame_skip: usize,
    ) -> Result<Vec<String>, Box<dyn std::error::Error>> {
        // Placeholder for video frame extraction
        // Implementation would use ffmpeg-next or similar to decode video frames
        // and encode each frame using encode_image_data()
        Err("video decoding not yet implemented - use ffmpeg to extract frames first".into())
    }
}
