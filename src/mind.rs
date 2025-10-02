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
    pub background_thought_coord: Option<MandelbrotCoord>,  // Where background thoughts are wandering
    pub contextual_coord: MandelbrotCoord,  // Context-driven position (not overwritten by learning)
    pub last_output: String,  // Last generated response or thought
    pub inhibited_symbols: Vec<(usize, u32)>,  // (symbol_idx, steps_remaining) - refractory period
    pub inhibited_trajectories: Vec<(usize, u32)>,  // (traj_idx, steps_remaining)
    pub scale: u32,
    pub exploration_radius: f64
}

impl FractalMind {
    pub fn new() -> Self {
        println!("initializing fractal mind - mandelbrot indexes julia patterns...");
        FractalMind {
            current_coord: MandelbrotCoord::new(-0.5, 0.0),
            contextual_coord: MandelbrotCoord::new(-0.5, 0.0),
            symbols: Vec::new(),
            last_output: String::new(),
            inhibited_symbols: Vec::new(),
            inhibited_trajectories: Vec::new(),
            trajectories: Vec::new(),
            associative_fields: Vec::new(),
            context_history: Vec::new(),
            symbol_history: Vec::new(),
            background_thought_coord: None,
            scale: 4096,  // Lower scale = faster, more diversity
            exploration_radius: 0.20,
        }
    }

    pub fn process_input(&mut self, input: &str) -> String {
        self.process_with_intensity(input, 1.0)
    }

    pub fn process_with_intensity(&mut self, input: &str, intensity: f64) -> String {
        // Multi-scale hierarchical processing for emergence
        self.process_hierarchical(input, intensity);

        // Update background thought position (low intensity wandering)
        if intensity < 0.3 {
            // This is a background thought - track where it's wandering
            self.background_thought_coord = Some(self.current_coord);
        }

        // Decay unused symbols (synaptic pruning)
        self.decay_symbols();

        self.generate_response(input)
    }

    fn process_hierarchical(&mut self, input: &str, base_intensity: f64) {
        // Only update context for high-intensity inputs
        if base_intensity > 0.3 {
            self.context_history.push(input.to_string());
            if self.context_history.len() > 10 {
                self.context_history.drain(0..1);
            }
        }

        let context = self.context_history.join(" ");

        // Level 3: Full semantic chunks (sentences/paragraphs) - HIGHEST intensity
        // This is the primary conceptual understanding
        let chunk_coord = hash_to_mandelbrot_coord(input);
        let contextual_position = contextual_mandelbrot_coord(chunk_coord, &context, 0.7 * base_intensity);

        // CRITICAL: Save contextual position BEFORE learning overwrites current_coord
        self.contextual_coord = contextual_position;

        self.learn_concept_with_intensity(input, base_intensity * 0.8);

        // Level 2: Word-level processing - MEDIUM intensity
        // Break into words for compositional understanding
        let words: Vec<&str> = input.split_whitespace()
            .filter(|w| !w.is_empty())
            .collect();

        if words.len() > 1 {
            for word in &words {
                self.learn_concept_with_intensity(word, base_intensity * 0.4);
            }
        }

        // Level 1: Character-level patterns - LOW intensity
        // Only for building basic pattern recognition, not primary storage
        // Process as continuous stream for pattern detection
        if base_intensity > 0.5 {
            // Only do character-level for high-intensity learning
            let chars: String = input.chars()
                .filter(|c| !c.is_whitespace())
                .take(50) // Limit to prevent explosion
                .collect();

            if !chars.is_empty() {
                self.learn_concept_with_intensity(&chars, base_intensity * 0.1);
            }
        }
    }

    fn learn_concept(&mut self, concept: &str) {
        self.learn_concept_with_intensity(concept, 1.0)
    }

    pub fn learn_concept_with_image(&mut self, concept: &str, intensity: f64, image_path: String) {
        self.learn_concept_with_intensity_and_image(concept, intensity, Some(image_path));
    }

    fn learn_concept_with_intensity(&mut self, concept: &str, intensity: f64) {
        self.learn_concept_with_intensity_and_image(concept, intensity, None);
    }

    fn learn_concept_with_intensity_and_image(&mut self, concept: &str, intensity: f64, image_path: Option<String>) {
        let mut path = Vec::new();
        let mut symbol_indices = Vec::new();

        for (i, ch) in concept.chars().enumerate() {
            // Hash based on character + context, but allow merging of similar patterns
            // For low-intensity (character-level), use simpler hash for consolidation
            // For high-intensity (word/concept-level), include position for diversity
            let char_coord = if intensity < 0.2 {
                // Character-level: just the character itself for maximum merging
                hash_to_mandelbrot_coord(&ch.to_string())
            } else {
                // Word/concept level: include position and context for diversity
                let char_seed = format!("{}:{}:{}", ch, i, concept);
                hash_to_mandelbrot_coord(&char_seed)
            };

            path.push(char_coord);

            let symbol_idx = self.store_symbol_at_coord(char_coord, ch);
            symbol_indices.push(symbol_idx);

            // Update current position
            self.current_coord = char_coord;
        }

        if !path.is_empty() {
            let trajectory = if let Some(img_path) = image_path {
                ConceptTrajectory::new_with_image(path.clone(), concept.to_string(), symbol_indices.clone(), img_path)
            } else {
                ConceptTrajectory::new(path.clone(), concept.to_string(), symbol_indices.clone())
            };
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

    fn generate_response(&mut self, input: &str) -> String {
        if self.trajectories.is_empty() || self.symbols.is_empty() {
            return "?".to_string();
        }

        // Response generation driven by:
        // 1. Input context (PRIMARY) - where did processing put us?
        // 2. Background thought (SECONDARY) - unconscious influence
        // 3. Slight randomness (TERTIARY) - creative variation

        let mut response = String::new();

        // Start from CONTEXTUAL position (not character-level wandering position)
        // This is the PRIMARY driver - response emerges from input context
        let mut current_coord = self.contextual_coord;

        let max_symbols = 100; // Generate up to 100 characters

        let mut rng = rand::thread_rng();

        let mut used_trajectories: Vec<usize> = Vec::new(); // Track which trajectories we use (Hebbian)
        let mut last_traj_idx: Option<usize> = None; // Track trajectory to enforce forward movement

        for _step in 0..max_symbols {
            // Decay inhibition counters
            for (_, steps) in &mut self.inhibited_symbols {
                *steps = steps.saturating_sub(1);
            }
            for (_, steps) in &mut self.inhibited_trajectories {
                *steps = steps.saturating_sub(1);
            }
            self.inhibited_symbols.retain(|(_, steps)| *steps > 0);
            self.inhibited_trajectories.retain(|(_, steps)| *steps > 0);
            let mut candidates: Vec<(usize, &ConceptTrajectory, usize, f64)> = Vec::new();

            // Search coordinate calculation:
            // 85% - stay at current position (follow input context)
            // 10% - integrate background thought
            // 5%  - slight random exploration
            let rand_val = rng.r#gen::<f64>();

            let search_coord = if rand_val < 0.85 {
                // PRIMARY: Follow the input context trajectory
                current_coord
            } else if rand_val < 0.95 {
                // SECONDARY: Background thought influence
                if let Some(bg_coord) = self.background_thought_coord {
                    // Blend background thought with current position
                    MandelbrotCoord {
                        re: current_coord.re * 0.7 + bg_coord.re * 0.3,
                        im: current_coord.im * 0.7 + bg_coord.im * 0.3,
                    }
                } else {
                    current_coord
                }
            } else {
                // TERTIARY: Tiny bit of exploration
                let nearby = find_nearby_interesting_points(current_coord, 0.1, 3);
                nearby.get(0).copied().unwrap_or(current_coord)
            };

            for (traj_idx, trajectory) in self.trajectories.iter().enumerate() {
                // INHIBITION: Skip recently-used trajectories (refractory period)
                if self.inhibited_trajectories.iter().any(|(idx, _)| *idx == traj_idx) {
                    continue;
                }

                let (closest_idx, dist) = trajectory.closest_point(search_coord);
                let influence = trajectory.influence_at(search_coord);

                // FORWARD MOVEMENT: If continuing same trajectory, must move forward
                let next_idx = if Some(traj_idx) == last_traj_idx {
                    // Same trajectory - must progress forward
                    closest_idx + 1
                } else {
                    // New trajectory - start at closest point
                    closest_idx
                };

                // Wide search - exploration_radius * 10
                if dist < self.exploration_radius * 10.0 && next_idx < trajectory.symbols.len() {
                    let symbol_idx = trajectory.symbols[next_idx];

                    // INHIBITION: Skip recently-used symbols
                    if self.inhibited_symbols.iter().any(|(idx, _)| *idx == symbol_idx) {
                        continue;
                    }

                    // HEBBIAN LEARNING: Stronger trajectories are easier to activate
                    let familiarity = trajectory.strength;

                    // Pattern richness: prefer multi-character trajectories
                    let pattern_bonus = if trajectory.symbols.len() > 5 { 2.0 } else { 0.5 };

                    let weight = influence * familiarity * pattern_bonus;
                    candidates.push((traj_idx, trajectory, symbol_idx, weight));
                }
            }

            // Also check associative fields for distant but semantically related patterns
            for field in &self.associative_fields {
                let field_dist = ConceptTrajectory::coord_distance(search_coord, field.center);
                if field_dist < field.radius * 3.0 {
                    // Find trajectories in this field
                    for (traj_idx, trajectory) in self.trajectories.iter().enumerate() {
                        if field.contains(trajectory.path[0]) && !trajectory.symbols.is_empty() {
                            let field_influence = field.strength * 0.5;
                            let symbol_idx = trajectory.symbols[0];
                            candidates.push((traj_idx, trajectory, symbol_idx, field_influence));
                        }
                    }
                }
            }

            if candidates.is_empty() {
                break;
            }

            // Sort by weight
            candidates.sort_by(|a, b| b.3.partial_cmp(&a.3).unwrap());

            // Probabilistically select from top candidates
            let total: f64 = candidates.iter().map(|(_, _, _, w)| w).sum();
            if total == 0.0 {
                break;
            }

            let mut r = rng.gen_range(0.0..total);

            let mut selected = None;
            for (traj_idx, traj, symbol_idx, weight) in &candidates {
                if r < *weight {
                    selected = Some((*traj_idx, *traj, *symbol_idx));
                    break;
                }
                r -= *weight;
            }

            if let Some((traj_idx, traj, symbol_idx)) = selected {
                // HEBBIAN STRENGTHENING: Reinforce used trajectory
                used_trajectories.push(traj_idx);

                // INHIBITION: Add to refractory period (3-5 steps)
                self.inhibited_symbols.push((symbol_idx, 3));
                self.inhibited_trajectories.push((traj_idx, 2));

                // Track which trajectory we're following
                last_traj_idx = Some(traj_idx);

                // Output character from symbol's label
                if symbol_idx < self.symbols.len() {
                    if let Some(label) = self.symbols[symbol_idx].label {
                        response.push(label);

                        // Stop at sentence boundaries
                        if label == '.' || label == '!' || label == '?' {
                            break;
                        }
                    }
                }

                // Move to next coordinate along this trajectory
                if let Some((next_coord, _)) = traj.suggest_next_coord(current_coord) {
                    current_coord = next_coord;
                }
            } else {
                break;
            }
        }

        self.current_coord = current_coord;

        // HEBBIAN STRENGTHENING: Reinforce trajectories that were used
        for traj_idx in used_trajectories {
            if traj_idx < self.trajectories.len() {
                self.trajectories[traj_idx].strength += 0.1;
            }
        }

        // TRAJECTORY DECAY: Weaken all trajectories slightly over time
        for traj in &mut self.trajectories {
            traj.strength = (traj.strength * 0.995).max(0.1); // Decay to minimum 0.1
        }

        let output = if response.is_empty() {
            "?".to_string()
        } else {
            response
        };

        self.last_output = output.clone();
        output
    }

    pub fn generate_background_thought(&mut self) -> String {
        if self.trajectories.is_empty() || self.symbols.is_empty() {
            return String::new();
        }

        // Background thoughts:
        // - Influenced by context (where we are in fractal space)
        // - Weaker associations than conscious responses
        // - More random exploration (unconscious wandering)
        // - NO vocabulary lookup, only geometric patterns

        let mut thought = String::new();
        let mut rng = rand::thread_rng();

        // Start from current position but with some drift
        let drift_amount = 0.3;
        let mut current_coord = MandelbrotCoord {
            re: self.current_coord.re + (rng.r#gen::<f64>() - 0.5) * drift_amount,
            im: self.current_coord.im + (rng.r#gen::<f64>() - 0.5) * drift_amount,
        };

        let max_symbols = 30; // Shorter background thoughts

        for _step in 0..max_symbols {
            let mut candidates: Vec<(&ConceptTrajectory, usize, f64)> = Vec::new();

            // Background thought search weighting:
            // 50% - current wandering position
            // 30% - nearby associations
            // 20% - random exploration
            let rand_val = rng.r#gen::<f64>();

            let search_coord = if rand_val < 0.5 {
                current_coord
            } else if rand_val < 0.8 {
                // Explore nearby associations
                let nearby = find_nearby_interesting_points(current_coord, 0.3, 5);
                nearby.get(0).copied().unwrap_or(current_coord)
            } else {
                // Random exploration
                let nearby = find_nearby_interesting_points(current_coord, 0.5, 10);
                if nearby.is_empty() {
                    current_coord
                } else {
                    nearby.get(rng.gen_range(0..nearby.len())).copied().unwrap_or(current_coord)
                }
            };

            // Find candidate patterns
            for trajectory in &self.trajectories {
                let (closest_idx, dist) = trajectory.closest_point(search_coord);

                // Wider search for background thoughts (more exploratory)
                if dist < self.exploration_radius * 15.0 && closest_idx < trajectory.symbols.len() {
                    let influence = trajectory.influence_at(search_coord);
                    let symbol_idx = trajectory.symbols[closest_idx];
                    candidates.push((trajectory, symbol_idx, influence * trajectory.strength));
                }
            }

            if candidates.is_empty() {
                break;
            }

            // Probabilistic selection
            let total: f64 = candidates.iter().map(|(_, _, w)| w).sum();
            if total == 0.0 {
                break;
            }

            let mut r = rng.gen_range(0.0..total);
            let mut selected = None;

            for (traj, symbol_idx, weight) in &candidates {
                if r < *weight {
                    selected = Some((*traj, *symbol_idx));
                    break;
                }
                r -= *weight;
            }

            if let Some((traj, symbol_idx)) = selected {
                if symbol_idx < self.symbols.len() {
                    if let Some(label) = self.symbols[symbol_idx].label {
                        thought.push(label);

                        if label == '.' || label == '!' || label == '?' {
                            break;
                        }
                    }
                }

                // Wander along trajectory with some drift
                if let Some((next_coord, _)) = traj.suggest_next_coord(current_coord) {
                    current_coord = MandelbrotCoord {
                        re: next_coord.re + (rng.r#gen::<f64>() - 0.5) * 0.1,
                        im: next_coord.im + (rng.r#gen::<f64>() - 0.5) * 0.1,
                    };
                }
            } else {
                break;
            }
        }

        // Update background thought coordinate
        self.background_thought_coord = Some(current_coord);

        thought
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

    /// Recall images associated with a concept
    pub fn recall_images(&mut self, concept: &str, limit: usize) -> Vec<String> {
        // Navigate to concept in fractal space
        self.process_input(concept);

        let current = self.current_coord;

        // Find trajectories with image paths near this concept
        let mut candidates: Vec<(String, f64)> = Vec::new();

        for traj in &self.trajectories {
            if let Some(ref img_path) = traj.image_path {
                let (_, dist) = traj.closest_point(current);
                let relevance = traj.strength * (-dist * dist / 0.1).exp();

                if relevance > 0.01 {
                    candidates.push((img_path.clone(), relevance));
                }
            }
        }

        // Sort by relevance
        candidates.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());

        // Return top N image paths
        candidates.into_iter()
            .take(limit)
            .map(|(path, _)| path)
            .collect()
    }

    /// Imagine/recall visual pattern from concept (generative)
    pub fn imagine_visual(&mut self, concept: &str, width: usize, height: usize) -> String {
        // Process concept to get fractal position
        self.process_input(concept);

        let mut visual = String::with_capacity(width * height + height);
        let mut current_coord = self.current_coord;

        // Generate grid of symbols from fractal trajectories
        for _y in 0..height {
            for _x in 0..width {
                // Find trajectories near current position
                let mut next_coords: Vec<(MandelbrotCoord, usize, f64)> = Vec::new();

                for trajectory in &self.trajectories {
                    let (closest_idx, dist) = trajectory.closest_point(current_coord);

                    if dist < self.exploration_radius && closest_idx + 1 < trajectory.symbols.len() {
                        let next_coord = trajectory.path[closest_idx + 1];
                        let next_sym = trajectory.symbols[closest_idx + 1];
                        let influence = trajectory.influence_at(current_coord);

                        next_coords.push((next_coord, next_sym, influence));
                    }
                }

                // Sample symbol from fractal space
                if !next_coords.is_empty() {
                    let total: f64 = next_coords.iter().map(|(_, _, w)| w).sum();
                    let mut rng = rand::thread_rng();
                    let mut r = rng.gen_range(0.0..total.max(0.01));

                    for &(coord, sym_idx, weight) in &next_coords {
                        if r < weight {
                            if sym_idx < self.symbols.len() {
                                if let Some(ch) = self.symbols[sym_idx].label {
                                    visual.push(ch);
                                } else {
                                    visual.push(' ');
                                }
                            } else {
                                visual.push(' ');
                            }
                            current_coord = coord;
                            break;
                        }
                        r -= weight;
                    }
                } else {
                    visual.push(' ');
                }
            }
            visual.push('\n');
        }

        visual
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