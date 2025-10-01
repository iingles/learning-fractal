use std::fs;
use bincode::{Encode, Decode};
use rand::Rng;

use crate::fractal::{
    MandelbrotCoord, hash_to_mandelbrot_coord, contextual_mandelbrot_coord,
    julia_fingerprint_from_mandelbrot, mandelbrot_stability, hamming_distance,
    find_nearby_interesting_points
};
use crate::memory::{FractalSymbol, ConceptTrajectory, AssociativeField};

#[derive(Encode, Decode)]
pub struct FractalMind {
    pub current_coord: MandelbrotCoord,
    pub symbols: Vec<FractalSymbol>,
    pub trajectories: Vec<ConceptTrajectory>,
    pub associative_fields: Vec<AssociativeField>,
    pub context_history: Vec<String>,
    pub symbol_history: Vec<usize>,
    pub scale: u32,
    pub exploration_radius: f64
}

impl FractalMind {
    pub fn new() -> Self {
        println!("initializing fractal mind - mandelbrot indexes julia patterns...");
        FractalMind {
            current_coord: MandelbrotCoord::new(-0.5, 0.0),
            symbols: Vec::new(),
            trajectories: Vec::new(),
            associative_fields: Vec::new(),
            context_history: Vec::new(),
            symbol_history: Vec::new(),
            scale: 4096,  // Lower scale = faster, more diversity
            exploration_radius: 0.20,
        }
    }

    pub fn process_input(&mut self, input: &str) -> String {
        self.process_with_intensity(input, 1.0)
    }

    pub fn process_with_intensity(&mut self, input: &str, intensity: f64) -> String {
        // Only update context for high-intensity inputs
        if intensity > 0.3 {
            self.context_history.push(input.to_string());
            if self.context_history.len() > 10 {
                self.context_history.drain(0..1);
            }
        }

        let base_coord = hash_to_mandelbrot_coord(input);
        let context = self.context_history.join(" ");
        self.current_coord = contextual_mandelbrot_coord(base_coord, &context, 0.7 * intensity);

        self.learn_concept_with_intensity(input, intensity);

        // Decay unused symbols (synaptic pruning)
        self.decay_symbols();

        self.generate_response(input)
    }

    fn learn_concept(&mut self, concept: &str) {
        self.learn_concept_with_intensity(concept, 1.0)
    }

    fn learn_concept_with_intensity(&mut self, concept: &str, intensity: f64) {
        let mut path = Vec::new();
        let mut symbol_indices = Vec::new();

        for (i, ch) in concept.chars().enumerate() {
            // Each character gets coordinate based on char+position+entire word
            // Use the full hash to maximize diversity
            let char_seed = format!("{}:{}:{}", ch, i, concept);
            let char_coord = hash_to_mandelbrot_coord(&char_seed);

            // Use the hashed coordinate directly for maximum pattern diversity
            path.push(char_coord);

            let symbol_idx = self.store_symbol_at_coord(char_coord, ch);
            symbol_indices.push(symbol_idx);

            // Update current position
            self.current_coord = char_coord;
        }

        if !path.is_empty() {
            let trajectory = ConceptTrajectory::new(path.clone(), concept.to_string(), symbol_indices.clone());
            self.trajectories.push(trajectory);


            // Add to symbol history for generation context
            self.symbol_history.extend(symbol_indices.iter());
            if self.symbol_history.len() > 20 {
                self.symbol_history.drain(..self.symbol_history.len() - 20);
            }

            // Create or strengthen associative field (scaled by intensity)
            let mut found_field = false;
            for field in &mut self.associative_fields {
                if field.contains(self.current_coord) {
                    field.strength += 0.1 * intensity;
                    found_field = true;
                    break;
                }
            }

            if !found_field && intensity > 0.3 {
                let field = AssociativeField::new(self.current_coord, self.exploration_radius);
                self.associative_fields.push(field);
            }

            // Strengthen connections between overlapping trajectories
            let traj_count = self.trajectories.len() - 1;
            for traj_idx in 0..traj_count {
                let mut strengthen = false;
                for &new_coord in &path {
                    for &old_coord in &self.trajectories[traj_idx].path {
                        let dist = ConceptTrajectory::coord_distance(new_coord, old_coord);
                        if dist < self.exploration_radius * 0.5 {
                            strengthen = true;
                            break;
                        }
                    }
                    if strengthen { break; }
                }
                if strengthen {
                    self.trajectories[traj_idx].strength += 0.05;
                }
            }
        }
    }

    fn store_symbol_at_coord(&mut self, coord: MandelbrotCoord, ch: char) -> usize {
        const MERGE_THRESHOLD: u32 = 8;

        let pattern = julia_fingerprint_from_mandelbrot(coord, self.scale);
        let stability = mandelbrot_stability(coord, self.scale);

        let mut min_distance = u32::MAX;
        let mut closest_symbol = None;

        for (i, symbol) in self.symbols.iter_mut().enumerate() {
            let distance = hamming_distance(&pattern, &symbol.pattern);
            if distance < min_distance {
                min_distance = distance;
                closest_symbol = Some(i);
            }

            // Merge similar patterns - character label is just metadata
            if distance < MERGE_THRESHOLD {
                symbol.count += 1;
                symbol.confidence = (symbol.confidence + 0.15).min(1.0);
                // Keep first label or update if unlabeled
                if symbol.label.is_none() {
                    symbol.label = Some(ch);
                }
                return i;
            }
        }

        let new_symbol = FractalSymbol::new(coord, pattern, stability);
        self.symbols.push(new_symbol);
        let idx = self.symbols.len() - 1;
        self.symbols[idx].label = Some(ch);
        idx
    }

    fn generate_response(&mut self, _input: &str) -> String {
        if self.symbols.is_empty() {
            return "?".to_string();
        }

        // Start at current position - what julia pattern are we at?
        let current_pattern = julia_fingerprint_from_mandelbrot(self.current_coord, self.scale);

        // Find symbols with similar patterns - these are associatively activated
        let mut activated: Vec<(usize, f64)> = Vec::new();

        for (sym_idx, symbol) in self.symbols.iter().enumerate() {
            let pattern = &symbol.pattern;
            let distance = hamming_distance(&current_pattern, pattern);
            let similarity = 1.0 - (distance as f64 / 2048.0);

            if similarity > 0.5 {  // Activation threshold
                activated.push((sym_idx, similarity * symbol.confidence as f64));
            }
        }

        if activated.is_empty() {
            return "?".to_string();
        }

        // Sort by activation
        activated.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());

        let mut response = String::new();
        let mut current_coord = self.current_coord;
        let max_length = 40;

        for _ in 0..max_length {
            // Find trajectories that pass through or near current position
            let mut next_coords: Vec<(MandelbrotCoord, usize, f64)> = Vec::new();

            for trajectory in &self.trajectories {
                let (closest_idx, dist) = trajectory.closest_point(current_coord);

                // If we're on this trajectory, follow it
                if dist < self.exploration_radius && closest_idx + 1 < trajectory.symbols.len() {
                    let next_coord = trajectory.path[closest_idx + 1];
                    let next_sym = trajectory.symbols[closest_idx + 1];
                    let influence = trajectory.influence_at(current_coord);

                    next_coords.push((next_coord, next_sym, influence));
                }
            }

            // Also consider symbols with similar julia patterns
            let current_pattern = julia_fingerprint_from_mandelbrot(current_coord, self.scale);
            for (sym_idx, symbol) in self.symbols.iter().enumerate() {
                let distance = hamming_distance(&current_pattern, &symbol.pattern);
                let similarity = 1.0 - (distance as f64 / 2048.0);

                if similarity > 0.5 {
                    next_coords.push((symbol.coord, sym_idx, similarity * 0.5));
                }
            }

            if next_coords.is_empty() {
                break;
            }

            // Probabilistically sample next position
            let total: f64 = next_coords.iter().map(|(_, _, w)| w).sum();
            let mut rng = rand::thread_rng();
            let mut r = rng.gen_range(0.0..total);

            let mut chosen = None;
            for &(coord, sym_idx, weight) in &next_coords {
                if r < weight {
                    chosen = Some((coord, sym_idx));
                    break;
                }
                r -= weight;
            }

            if let Some((next_coord, sym_idx)) = chosen {
                if sym_idx < self.symbols.len() {
                    if let Some(ch) = self.symbols[sym_idx].label {
                        response.push(ch);
                        current_coord = next_coord;

                        if matches!(ch, '.' | '!' | '?') && response.len() > 5 {
                            break;
                        }
                    } else {
                        break;
                    }
                } else {
                    break;
                }
            } else {
                break;
            }
        }

        self.current_coord = current_coord;

        if response.trim().is_empty() {
            "?".to_string()
        } else {
            response
        }
    }

    fn explore_response(&mut self) -> String {
        // When no patterns match, explore nearby space
        let nearby_points = find_nearby_interesting_points(self.current_coord, 0.1, 30);

        for point in nearby_points {
            if let Some(symbol_idx) = self.find_nearest_symbol(point) {
                if let Some(ch) = self.symbols[symbol_idx].label {
                    self.current_coord = point;
                    return ch.to_string();
                }
            }
        }

        "?".to_string()
    }

    fn get_contextual_suggestions(&self, coord: MandelbrotCoord) -> Vec<(MandelbrotCoord, usize)> {
        let mut suggestions = Vec::new();

        for trajectory in &self.trajectories {
            if let Some((next_coord, symbol_idx)) = trajectory.suggest_next_coord(coord) {
                let influence = trajectory.influence_at(coord);
                if influence > 0.1 {
                    suggestions.push((next_coord, symbol_idx));
                }
            }
        }

        suggestions.sort_by(|a, b| {
            let dist_a = ConceptTrajectory::coord_distance(coord, a.0);
            let dist_b = ConceptTrajectory::coord_distance(coord, b.0);
            dist_a.partial_cmp(&dist_b).unwrap()
        });

        suggestions
    }

    fn find_nearest_symbol(&self, coord: MandelbrotCoord) -> Option<usize> {
        let mut best_idx = None;
        let mut best_dist = f64::INFINITY;

        for (i, symbol) in self.symbols.iter().enumerate() {
            let dist = ConceptTrajectory::coord_distance(coord, symbol.coord);
            if dist < best_dist {
                best_dist = dist;
                best_idx = Some(i);
            }
        }

        best_idx
    }

    fn decay_symbols(&mut self) {
        // Gradually decay confidence of all symbols (synaptic pruning)
        const DECAY_RATE: f32 = 0.001;
        const MIN_CONFIDENCE: f32 = 0.05;

        for symbol in &mut self.symbols {
            symbol.confidence = (symbol.confidence - DECAY_RATE).max(0.0);
        }

        // Mark symbols that are referenced by trajectories (can't be pruned)
        let mut referenced = vec![false; self.symbols.len()];
        for traj in &self.trajectories {
            for &sym_idx in &traj.symbols {
                if sym_idx < referenced.len() {
                    referenced[sym_idx] = true;
                }
            }
        }

        // Remove symbols that have decayed below threshold, aren't referenced, and haven't been used much
        let mut new_symbols = Vec::new();
        let mut index_map = vec![0; self.symbols.len()];

        for (old_idx, symbol) in self.symbols.iter().enumerate() {
            if symbol.confidence > MIN_CONFIDENCE || symbol.count > 10 || referenced[old_idx] {
                index_map[old_idx] = new_symbols.len();
                new_symbols.push(symbol.clone());
            }
        }

        // Update trajectory indices to match new symbol positions
        for traj in &mut self.trajectories {
            traj.symbols = traj.symbols.iter()
                .filter_map(|&old_idx| {
                    if old_idx < index_map.len() && old_idx < self.symbols.len() {
                        let new_idx = index_map[old_idx];
                        if new_idx < new_symbols.len() {
                            Some(new_idx)
                        } else {
                            None
                        }
                    } else {
                        None
                    }
                })
                .collect();
        }

        self.symbols = new_symbols;
    }

    pub fn display_state(&self) {
        println!("\n╭─── Fractal Mind State ───╮");
        println!("│ Position: ({:.3},{:.3})", self.current_coord.re, self.current_coord.im);
        println!("│ Symbols: {} (labeled: {})",
                 self.symbols.len(),
                 self.symbols.iter().filter(|s| s.label.is_some()).count());
        println!("│ Trajectories: {}", self.trajectories.len());
        println!("│ Fields: {}", self.associative_fields.len());
        println!("│ Context: {}", self.context_history.len());
        let mem_mb = (self.symbols.len() * 400 + self.trajectories.len() * 300) / 1024;
        println!("│ Memory: ~{}KB", mem_mb);
        println!("╰───────────────────────────╯");
    }

    pub fn alphabet(&self) {
        print!("learned patterns: ");
        for s in &self.symbols {
            if let Some(ch) = s.label {
                print!("{} ", ch);
            }
        }   
    }

    pub fn save(&self, filename: &str) -> Result<(), Box<dyn std::error::Error>> {
        println!("saving fractal mind...");
        let cfg = bincode::config::standard();
        let encoded = bincode::encode_to_vec(self, cfg)?;
        fs::write(filename, encoded)?;
        println!("saved {} symbols, {} trajectories, {} fields",
                 self.symbols.len(), self.trajectories.len(), self.associative_fields.len());
        Ok(())
    }

    pub fn load(filename: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let data = fs::read(filename)?;
        let cfg = bincode::config::standard();
        let (mut mind, _len): (FractalMind, usize) = bincode::decode_from_slice(&data, cfg)?;
        mind.context_history.clear();
        mind.symbol_history.clear();

        println!("loaded {} symbols, {} trajectories, {} fields",
                 mind.symbols.len(), mind.trajectories.len(), mind.associative_fields.len());
        Ok(mind)
    }
}