use std::io::{self, Write};
use std::sync::{Arc, Mutex};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use version_004::{
    FractalMind, LLMBridge, spawn_visualizer,
    CameraEncoder, FrameEncoding,
    AudioEncoder, AudioEncoding,
    encode_image, ImageEncoding
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mind = match FractalMind::load("mind_state.bin") {
        Ok(m) => m,
        Err(_) => FractalMind::new()
    };

    let mind = Arc::new(Mutex::new(mind));
    let llm = Arc::new(LLMBridge::new("mistral:7b"));
    let dreaming = Arc::new(AtomicBool::new(false));

    // Spawn visual window
    spawn_visualizer(Arc::clone(&mind));

    // Spawn background thought thread
    spawn_background_thought(Arc::clone(&mind), Arc::clone(&llm), Arc::clone(&dreaming));

    println!("\n╭──────────────────────────────────────────╮");
    println!("│      fractal mind v2.0 + LLM bridge      │");
    println!("│                                          │");
    println!("│ mandelbrot coordinates index julia sets  │");
    println!("│ information stored in fractal patterns   │");
    println!("│ llm translates symbols into language     │");
    println!("│                                          │");
    println!("│ /state  /alphabet  /reset  /save  /quit  │");
    println!("│ /train <rounds>  - train mind with LLM  │");
    println!("│ /read              - ingest text file    │");
    println!("│ /dream             - toggle LLM dreams   │");
    println!("│ /camera            - train from camera   │");
    println!("│ /audio             - train from audio    │");
    println!("│ /image <path>      - train from image    │");
    println!("│ /images            - batch process dir   │");
    println!("│ /learn             - supervised learning │");
    println!("│ /imagine           - visualize concept   │");
    println!("│                                          │");
    println!("│ (background thought always active)       │");
    println!("╰──────────────────────────────────────────╯\n");

    loop {
        print!("you: ");
        io::stdout().flush()?;

        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        let input = input.trim();

        if input.is_empty() { continue; }

        match input {
            "/quit" => break,
            "/state" => { mind.lock().unwrap().display_state(); continue; }
            "/alphabet" => { mind.lock().unwrap().alphabet(); continue; }
            "/dream" => {
                let current = dreaming.load(Ordering::Relaxed);
                dreaming.store(!current, Ordering::Relaxed);
                println!("{} LLM-guided dreaming\n", if !current { "🌙 enabled" } else { "💤 disabled" });
                continue;
            }
            "/imagine" => {
                print!("concept (what should I recall?): ");
                io::stdout().flush()?;
                let mut concept = String::new();
                io::stdin().read_line(&mut concept)?;
                let concept = concept.trim();

                if concept.is_empty() {
                    println!("cancelled\n");
                    continue;
                }

                println!("\n💭 recalling images related to '{}'...\n", concept);

                let recalled = mind.lock().unwrap().recall_images(concept, 5);

                if recalled.is_empty() {
                    println!("No images found near '{}' in fractal memory.\n", concept);
                    println!("Try using /learn to teach me what this concept looks like.\n");
                } else {
                    println!("Found {} image(s):\n", recalled.len());
                    for (i, path) in recalled.iter().enumerate() {
                        println!("{}. {}", i+1, path);

                        // Open in viewer (WSL-compatible)
                        let _ = std::process::Command::new("wslview")
                            .arg(path)
                            .spawn();
                    }
                    println!();
                }
                continue;
            }
            "/camera" => {
                println!("\n📷 starting camera feed training (press Ctrl+C to stop)...\n");

                match CameraEncoder::new(FrameEncoding::EdgeSymbols) {
                    Ok(mut encoder) => {
                        println!("camera initialized - feeding edge patterns to fractal mind\n");

                        for i in 0..100 {  // 100 frames
                            match encoder.capture_frame() {
                                Ok(encoded) => {
                                    mind.lock().unwrap().process_with_intensity(&encoded, 0.2);
                                    println!("[{}] processed frame ({} bytes)", i+1, encoded.len());
                                    std::thread::sleep(Duration::from_millis(100));
                                }
                                Err(e) => {
                                    println!("frame capture error: {}", e);
                                    break;
                                }
                            }
                        }
                        println!("\n✓ camera training complete\n");
                        mind.lock().unwrap().save("mind_state.bin")?;
                    }
                    Err(e) => println!("camera error: {}\n", e),
                }
                continue;
            }
            "/audio" => {
                println!("\n🎤 starting audio feed training (10 seconds)...\n");

                match AudioEncoder::new(AudioEncoding::BandSymbols { bands: 8 }) {
                    Ok(encoder) => {
                        println!("audio initialized - feeding frequency bands to fractal mind\n");

                        for i in 0..100 {  // 10 seconds at 100ms intervals
                            let encoded = encoder.encode_current();
                            mind.lock().unwrap().process_with_intensity(&encoded, 0.15);
                            println!("[{}] {}", i+1, &encoded[..40.min(encoded.len())]);
                            std::thread::sleep(Duration::from_millis(100));
                        }

                        println!("\n✓ audio training complete\n");
                        mind.lock().unwrap().save("mind_state.bin")?;
                    }
                    Err(e) => println!("audio error: {}\n", e),
                }
                continue;
            }
            "/read" => {
                let base_path = "data";

                if !std::path::Path::new(base_path).exists() {
                    println!("error: data/ directory does not exist\n");
                    continue;
                }

                // Process folders 00-07 in order (easiest to hardest)
                let mut all_paths = Vec::new();
                for i in 0..8 {
                    let folder = format!("{}/{:02}", base_path, i);
                    if std::path::Path::new(&folder).exists() {
                        let mut folder_paths: Vec<_> = walkdir::WalkDir::new(&folder)
                            .into_iter()
                            .filter_map(|e| e.ok())
                            .filter(|e| e.file_type().is_file())
                            .map(|e| e.path().to_path_buf())
                            .collect();
                        folder_paths.sort(); // Sort within folder
                        all_paths.extend(folder_paths);
                    }
                }

                if all_paths.is_empty() {
                    println!("no files found in data/00 through data/07\n");
                    continue;
                }

                println!("\n📖 reading {} file(s) from data/00-07 (easiest→hardest)...\n", all_paths.len());

                let paths = all_paths;

                for (i, file_path) in paths.iter().enumerate() {
                    match std::fs::read_to_string(file_path) {
                        Ok(content) => {
                            println!("[{}/{}] {} ({} bytes)",
                                i+1, paths.len(),
                                file_path.display(),
                                content.len()
                            );

                            // Process in chunks to allow background thought
                            const CHUNK_SIZE: usize = 10000;
                            let chunks: Vec<&str> = content
                                .as_bytes()
                                .chunks(CHUNK_SIZE)
                                .map(|chunk| std::str::from_utf8(chunk).unwrap_or(""))
                                .collect();

                            for (chunk_idx, chunk) in chunks.iter().enumerate() {
                                {
                                    mind.lock().unwrap().process_input(chunk);
                                }
                                // Release lock briefly after each chunk for visualizer + background thought
                                std::thread::sleep(Duration::from_millis(10));

                                // Auto-save every 50 chunks
                                if chunk_idx % 50 == 0 && chunk_idx > 0 {
                                    let _ = mind.lock().unwrap().save("mind_state.bin");
                                }
                            }

                            // Save after every file
                            let _ = mind.lock().unwrap().save("mind_state.bin");
                        }
                        Err(e) => println!("[{}/{}] error: {}", i+1, paths.len(), e),
                    }
                }

                println!("\n✓ ingested {} files\n", paths.len());
                mind.lock().unwrap().save("mind_state.bin")?;
                continue;
            }
            "/reset" => {
                *mind.lock().unwrap() = FractalMind::new();
                println!("fractal mind reset\n");
                continue;
            }
            "/save" => {
                mind.lock().unwrap().save("mind_state.bin")?;
                continue;
            }
            _ => {
                if input.starts_with("/image") {
                    let path = input.split_whitespace()
                        .nth(1)
                        .unwrap_or("");

                    if path.is_empty() {
                        println!("usage: /image <path>\n");
                        continue;
                    }

                    println!("\n🖼️  processing image: {}\n", path);

                    match encode_image(path, ImageEncoding::EdgeAscii { width: 40, height: 40 }) {
                        Ok(encoded) => {
                            mind.lock().unwrap().process_with_intensity(&encoded, 0.3);
                            println!("✓ image encoded and processed ({} bytes)\n", encoded.len());
                            mind.lock().unwrap().save("mind_state.bin")?;
                        }
                        Err(e) => println!("image error: {}\n", e),
                    }
                    continue;
                }

                if input.starts_with("/images") {
                    let dir_path = "images";

                    if !std::path::Path::new(dir_path).exists() {
                        println!("error: path does not exist: {}\n", dir_path);
                        continue;
                    }

                    let image_exts = ["jpg", "jpeg", "png", "gif", "bmp", "webp"];
                    let mut image_paths = Vec::new();

                    for entry in walkdir::WalkDir::new(dir_path)
                        .into_iter()
                        .filter_map(|e| e.ok())
                        .filter(|e| e.file_type().is_file())
                    {
                        if let Some(ext) = entry.path().extension() {
                            if image_exts.contains(&ext.to_str().unwrap_or("").to_lowercase().as_str()) {
                                image_paths.push(entry.path().to_path_buf());
                            }
                        }
                    }

                    println!("\n📸 found {} images\n", image_paths.len());

                    for (i, img_path) in image_paths.iter().enumerate() {
                        match encode_image(img_path, ImageEncoding::EdgeAscii { width: 40, height: 40 }) {
                            Ok(encoded) => {
                                println!("[{}/{}] {}", i+1, image_paths.len(), img_path.display());
                                mind.lock().unwrap().process_with_intensity(&encoded, 0.2);

                                if (i + 1) % 10 == 0 {
                                    let _ = mind.lock().unwrap().save("mind_state.bin");
                                }
                            }
                            Err(e) => println!("[{}/{}] error: {}", i+1, image_paths.len(), e),
                        }
                    }

                    println!("\n✓ processed {} images\n", image_paths.len());
                    mind.lock().unwrap().save("mind_state.bin")?;
                    continue;
                }

                if input.starts_with("/learn") {
                    let dir_path = "images";

                    if !std::path::Path::new(dir_path).exists() {
                        println!("error: path does not exist: {}\n", dir_path);
                        continue;
                    }

                    let image_exts = ["jpg", "jpeg", "png", "gif", "bmp", "webp"];
                    let mut image_paths = Vec::new();

                    for entry in walkdir::WalkDir::new(dir_path)
                        .into_iter()
                        .filter_map(|e| e.ok())
                        .filter(|e| e.file_type().is_file())
                    {
                        if let Some(ext) = entry.path().extension() {
                            if image_exts.contains(&ext.to_str().unwrap_or("").to_lowercase().as_str()) {
                                image_paths.push(entry.path().to_path_buf());
                            }
                        }
                    }

                    println!("\n🎓 interactive learning - {} images", image_paths.len());
                    println!("I'll show you each image. You tell me what it is.\n");
                    println!("Commands: <label>, 'skip', 'quit'\n");

                    for (i, img_path) in image_paths.iter().enumerate() {
                        println!("\n[{}/{}]", i+1, image_paths.len());
                        println!("Opening: {}", img_path.display());

                        // Open image using wslview (WSL utility for opening files in Windows)
                        if let Err(e) = std::process::Command::new("wslview").arg(&img_path).spawn() {
                            println!("couldn't open image (install wslu: sudo apt install wslu): {}", e);
                            println!("Please open manually: {}\n", img_path.display());
                        }

                        // Encode visual pattern while user looks at image
                        match encode_image(&img_path, ImageEncoding::EdgeAscii { width: 60, height: 40 }) {
                            Ok(encoded) => {
                                // Process visual pattern (low intensity, just sensing)
                                mind.lock().unwrap().process_with_intensity(&encoded, 0.15);

                                // Mind tries to guess from fractal space
                                let guess = mind.lock().unwrap().process_input("what is this");
                                println!("\nmind's guess: {}", guess);

                                print!("\nyou (what is this?): ");
                                io::stdout().flush()?;
                                let mut label = String::new();
                                io::stdin().read_line(&mut label)?;
                                let label = label.trim();

                                if label.is_empty() || label == "skip" {
                                    println!("skipped\n");
                                    continue;
                                }
                                if label == "quit" {
                                    break;
                                }

                                // Learn: visual pattern + semantic label together (high intensity)
                                // Store image path with the trajectory
                                let combined = format!("{}\n{}", encoded, label);
                                mind.lock().unwrap().learn_concept_with_image(&combined, 0.8, img_path.to_string_lossy().to_string());

                                println!("✓ learned: {}\n", label);

                                if (i + 1) % 5 == 0 {
                                    let _ = mind.lock().unwrap().save("mind_state.bin");
                                }
                            }
                            Err(e) => println!("encoding error: {}", e),
                        }
                    }

                    println!("\n✓ learning session complete\n");
                    mind.lock().unwrap().save("mind_state.bin")?;
                    continue;
                }

                if input.starts_with("/train") {
                    let rounds: usize = input.split_whitespace()
                        .nth(1)
                        .and_then(|s| s.parse().ok())
                        .unwrap_or(10);

                    println!("\n🧠 training fractal mind with LLM for {} rounds...\n", rounds);

                    let contexts = [
                        "hello", "who are you", "what do you feel", "tell me a story",
                        "do you dream", "what is beauty", "are you afraid", "sing me a song",
                        "what is love", "describe the stars", "who am I", "what is real",
                        "make me laugh", "what hurts", "where do you go", "remember this",
                        "teach me something", "what matters", "why exist", "create something"
                    ];

                    for i in 0..rounds {
                        let context = contexts[i % contexts.len()];

                        // Generate fractal exploration (from arbitrary input)
                        let exploration = mind.lock().unwrap().process_input(&format!("explore {}", i));

                        // LLM interprets the fractal output with varied context
                        if let Ok(llm_response) = llm.translate_symbols(&exploration, context).await {
                            // Feed LLM's interpretation back into fractal (high intensity - training)
                            mind.lock().unwrap().process_with_intensity(&llm_response, 0.8);
                            println!("[{}] ctx:'{}' fractal:{} → llm:{}",
                                i+1,
                                context,
                                &exploration[..30.min(exploration.len())],
                                &llm_response[..50.min(llm_response.len())]
                            );
                        }
                    }

                    println!("\n✓ training complete\n");
                    mind.lock().unwrap().save("mind_state.bin")?;
                    continue;
                }
            }
        }

        // Normal interaction: high intensity input
        let response = mind.lock().unwrap().process_input(input); // intensity = 1.0

        println!("mind: {}\n", response);

        let should_save = {
            let m = mind.lock().unwrap();
            m.trajectories.len() % 5 == 0 && !m.trajectories.is_empty()
        };

        if should_save {
            let _ = mind.lock().unwrap().save("mind_state.bin");
        }
    }

    mind.lock().unwrap().save("mind_state.bin")?;
    Ok(())
}

fn spawn_background_thought(
    mind: Arc<Mutex<FractalMind>>,
    llm: Arc<LLMBridge>,
    dreaming: Arc<AtomicBool>
) {
    tokio::spawn(async move {
        let mut iteration = 0;

        loop {
            tokio::time::sleep(Duration::from_secs(2)).await;

            iteration += 1;

            // Background thought generation:
            // Context-influenced but weaker association than conscious responses
            // Wanders through fractal space based on recent context + associations
            let thought = {
                if let Ok(mut m) = mind.try_lock() {
                    // Generate thought from current context (no input string needed)
                    // Uses context_history and current_coord to wander associatively
                    m.generate_background_thought()
                } else {
                    // Mind busy, skip this cycle
                    continue;
                }
            };

            // If dreaming enabled: LLM guides every 20 iterations
            if dreaming.load(Ordering::Relaxed) && iteration % 20 == 0 {
                if let Ok(guidance) = llm.translate_symbols(&thought, "dreaming").await {
                    println!("\n💭 dream: {}\n", &guidance[..60.min(guidance.len())]);

                    // Feed LLM guidance back in as low-intensity learning
                    if let Ok(mut m) = mind.try_lock() {
                        m.process_with_intensity(&guidance, 0.1);
                    }
                } else {
                    // Feed thought back as low-intensity learning
                    if let Ok(mut m) = mind.try_lock() {
                        m.process_with_intensity(&thought, 0.1);
                    }
                }
            } else {
                // Feed thought back as very low-intensity learning
                if let Ok(mut m) = mind.try_lock() {
                    m.process_with_intensity(&thought, 0.05);
                }
            }

            // Auto-save every 100 background iterations
            if iteration % 100 == 0 {
                if let Ok(m) = mind.try_lock() {
                    let _ = m.save("mind_state.bin");
                }
            }
        }
    });
}