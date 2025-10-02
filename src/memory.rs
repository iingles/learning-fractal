use bincode::{Encode, Decode};
use crate::fractal::{Fingerprint, MandelbrotCoord};

#[derive(Clone, Encode, Decode)]
pub struct FractalSymbol {
    pub coord: MandelbrotCoord,
    pub pattern: Fingerprint,
    pub count: u32,
    pub label: Option<char>,
    pub confidence: f32,
    pub stability: f64,
}

impl FractalSymbol {
    pub fn new(coord: MandelbrotCoord, pattern: Fingerprint, stability: f64) -> Self {
        FractalSymbol {
            coord,
            pattern,
            count: 1,
            label: None,
            confidence: 0.0,
            stability,
        }
    }
}

#[derive(Clone, Encode, Decode)]
pub struct ConceptTrajectory {
    pub path: Vec<MandelbrotCoord>,
    pub strength: f64,
    pub symbols: Vec<usize>,  // Indices into fractal symbol space (geometric patterns, not strings)
    pub image_path: Option<String>,  // Only for visual recall - path to source image
    // NO STRING STORAGE - emergence from geometry alone
}

impl ConceptTrajectory {
    pub fn new(path: Vec<MandelbrotCoord>, _concept: String, symbols: Vec<usize>) -> Self {
        // Concept string is discarded - only geometric patterns matter
        ConceptTrajectory {
            path,
            strength: 1.0,
            symbols,
            image_path: None,
        }
    }

    pub fn new_with_image(path: Vec<MandelbrotCoord>, _concept: String, symbols: Vec<usize>, image_path: String) -> Self {
        // Concept string is discarded - only geometric patterns + image reference matter
        ConceptTrajectory {
            path,
            strength: 1.0,
            symbols,
            image_path: Some(image_path),
        }
    }

    pub fn coord_distance(a: MandelbrotCoord, b: MandelbrotCoord) -> f64 {
        let dr = a.re - b.re;
        let di = a.im - b.im;
        (dr * dr + di * di).sqrt()
    }

    pub fn closest_point(&self, coord: MandelbrotCoord) -> (usize, f64) {
        let mut best_idx = 0;
        let mut best_dist = Self::coord_distance(coord, self.path[0]);

        for (i, &point) in self.path.iter().enumerate().skip(1) {
            let dist = Self::coord_distance(coord, point);
            if dist < best_dist {
                best_dist = dist;
                best_idx = i;
            }
        }
        (best_idx, best_dist)
    }

    pub fn influence_at(&self, coord: MandelbrotCoord) -> f64 {
        let (_, dist) = self.closest_point(coord);
        self.strength * (-dist * dist / 0.05).exp()
    }

    pub fn suggest_next_coord(&self, current: MandelbrotCoord) -> Option<(MandelbrotCoord, usize)> {
        let (idx, _) = self.closest_point(current);
        if idx + 1 < self.path.len() {
            let next_coord = self.path[idx + 1];
            let symbol_idx = if idx + 1 < self.symbols.len() {
                Some(self.symbols[idx + 1])
            } else {
                None
            };
            Some((next_coord, symbol_idx?))
        } else {
            None
        }
    }
}

#[derive(Clone, Encode, Decode)]
pub struct AssociativeField {
    pub center: MandelbrotCoord,
    pub radius: f64,
    pub strength: f64,
}

impl AssociativeField {
    pub fn new(center: MandelbrotCoord, radius: f64) -> Self {
        AssociativeField {
            center,
            radius,
            strength: 1.0,
        }
    }

    pub fn contains(&self, coord: MandelbrotCoord) -> bool {
        ConceptTrajectory::coord_distance(self.center, coord) <= self.radius
    }

    pub fn influence_at(&self, coord: MandelbrotCoord) -> f64 {
        let dist = ConceptTrajectory::coord_distance(self.center, coord);
        if dist <= self.radius {
            self.strength * (1.0 - dist / self.radius)
        } else {
            0.0
        }
    }

    pub fn add_association(&mut self, _concept: String, _weight: f64) {
        // Concept is encoded in the field's position/strength, not stored as text
    }
}