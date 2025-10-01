use std::io::{self, Write};
use std::sync::{Arc, Mutex};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use version_004::{FractalMind, LLMBridge, spawn_visualizer};

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
            "/read" => {
                print!("path (file or directory): ");
                io::stdout().flush()?;
                let mut path = String::new();
                io::stdin().read_line(&mut path)?;
                let path = path.trim();

                if !std::path::Path::new(path).exists() {
                    println!("error: path does not exist: {}\n", path);
                    continue;
                }

                let paths = if std::path::Path::new(path).is_dir() {
                    // Walk directory recursively
                    walkdir::WalkDir::new(path)
                        .into_iter()
                        .filter_map(|e| e.ok())
                        .filter(|e| e.file_type().is_file())
                        .map(|e| e.path().to_path_buf())
                        .collect::<Vec<_>>()
                } else {
                    vec![std::path::PathBuf::from(path)]
                };

                println!("\n📖 reading {} file(s)...\n", paths.len());

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

                            // Auto-save every 10 files
                            if (i + 1) % 10 == 0 {
                                let _ = mind.lock().unwrap().save("mind_state.bin");
                            }
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

        // Response has medium intensity for self-learning
        mind.lock().unwrap().process_with_intensity(&response, 0.5);

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
        let mut last_thought = String::from("...");

        loop {
            tokio::time::sleep(Duration::from_secs(2)).await;

            iteration += 1;

            // Always processing: output becomes input (low intensity background thought)
            let thought = {
                if let Ok(mut m) = mind.try_lock() {
                    m.process_with_intensity(&last_thought, 0.1)
                } else {
                    // Mind busy, skip this cycle
                    continue;
                }
            };

            // If dreaming enabled: LLM guides every 20 iterations
            if dreaming.load(Ordering::Relaxed) && iteration % 20 == 0 {
                if let Ok(guidance) = llm.translate_symbols(&thought, "dreaming").await {
                    println!("\n💭 dream: {}\n", &guidance[..60.min(guidance.len())]);
                    last_thought = guidance;
                } else {
                    last_thought = thought;
                }
            } else {
                last_thought = thought;
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