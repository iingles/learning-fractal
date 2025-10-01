use crate::math::C;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

pub type Fingerprint = Vec<u64>; // 32 u64s = 2048 bits

#[derive(Clone, Copy, Debug, bincode::Encode, bincode::Decode)]
pub struct MandelbrotCoord {
    pub re: f64,
    pub im: f64,
}

impl MandelbrotCoord {
    pub fn new(re: f64, im: f64) -> Self {
        MandelbrotCoord { re, im }
    }

    pub fn to_julia_param(&self) -> C {
        C::new(self.re, self.im)
    }
}

pub fn mandelbrot_escapes(c: C, max_iter: u32) -> u32 {
    let mut z = C::new(0.0, 0.0);
    for i in 0..max_iter {
        if z.abs_sq() > 4.0 { return i; }
        z = z.mul(z).add(c);
    }
    max_iter
}

pub fn julia_escapes(z0: C, c: C, max_iter: u32) -> u32 {
    let mut z = z0;
    for i in 0..max_iter {
        if z.abs_sq() > 4.0 { return i; }
        z = z.mul(z).add(c);
    }
    max_iter
}

pub fn hash_to_mandelbrot_coord(input: &str) -> MandelbrotCoord {
    let mut hasher = DefaultHasher::new();
    input.hash(&mut hasher);
    let hash = hasher.finish();

    let re_bits = (hash >> 32) as u32;
    let im_bits = hash as u32;

    // Map to interesting region of mandelbrot set
    // Re: -0.75 to 0.25 (1.0 range)
    // Im: -1.0 to 1.0 (2.0 range)
    let re = -0.75 + (re_bits as f64 / u32::MAX as f64) * 1.0;
    let im = -1.0 + (im_bits as f64 / u32::MAX as f64) * 2.0;

    MandelbrotCoord::new(re, im)
}

pub fn contextual_mandelbrot_coord(base: MandelbrotCoord, context: &str, influence: f64) -> MandelbrotCoord {
    let context_coord = hash_to_mandelbrot_coord(context);

    let re = base.re + (context_coord.re - base.re) * influence * 0.1;
    let im = base.im + (context_coord.im - base.im) * influence * 0.1;

    let re = re.clamp(-0.75, 0.25);
    let im = im.clamp(-1.0, 1.0);

    MandelbrotCoord::new(re, im)
}

pub fn julia_fingerprint_from_mandelbrot(coord: MandelbrotCoord, scale: u32) -> Fingerprint {
    let c = coord.to_julia_param();
    let grid_size = 32;
    let range = 2.0;
    let mut bits = vec![0u64; 32];
    let third = scale / 3;

    for y in 0..grid_size {
        for x in 0..grid_size {
            let zx = -range + (x as f64 / grid_size as f64) * 2.0 * range;
            let zy = -range + (y as f64 / grid_size as f64) * 2.0 * range;
            let z0 = C::new(zx, zy);
            let escape_time = julia_escapes(z0, c, scale);

            let bin = if escape_time >= scale { 0 }
                     else if escape_time <= third { 1 }
                     else if escape_time <= 2 * third { 2 }
                     else { 3 };

            let idx = y * grid_size + x;
            let word_idx = (idx * 2) / 64;
            let bit_offset = (idx * 2) % 64;

            bits[word_idx] |= (bin as u64) << bit_offset;
        }
    }
    bits
}

pub fn mandelbrot_stability(coord: MandelbrotCoord, scale: u32) -> f64 {
    let c = coord.to_julia_param();
    let escape_time = mandelbrot_escapes(c, scale);
    (escape_time as f64) / (scale as f64)
}

pub fn find_nearby_interesting_points(coord: MandelbrotCoord, radius: f64, samples: usize) -> Vec<MandelbrotCoord> {
    let mut points = Vec::new();

    for i in 0..samples {
        let angle = (i as f64 / samples as f64) * 2.0 * std::f64::consts::PI;
        let r = radius * (i as f64 / samples as f64).sqrt();

        let re = coord.re + r * angle.cos();
        let im = coord.im + r * angle.sin();

        let re = re.clamp(-0.75, 0.25);
        let im = im.clamp(-1.0, 1.0);

        let test_coord = MandelbrotCoord::new(re, im);
        let stability = mandelbrot_stability(test_coord, 256);

        if stability > 0.3 && stability < 0.95 {
            points.push(test_coord);
        }
    }

    points
}

pub fn hamming_distance(a: &Fingerprint, b: &Fingerprint) -> u32 {
    a.iter().zip(b.iter())
        .map(|(x, y)| (x ^ y).count_ones())
        .sum()
}