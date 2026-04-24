mod app;
mod builder;
mod hardware;
mod ui;
mod huggingface;
mod orchestrator;
mod service;

use anyhow::Result;
use app::{App, InputMode};
use clap::{Parser, Subcommand};
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};
use std::{io, time::Duration};
use std::path::PathBuf;
use std::sync::mpsc;
use std::thread;

#[derive(Parser)]
#[command(name = "oflama")]
#[command(about = "A TUI to manage llama.cpp, models, and hardware optimizations", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// Build llama.cpp from source
    Build {
        #[arg(short, long, default_value = "cuda", help = "Backend: cuda|sycl|vulkan|cpu")]
        backend: String,
        #[arg(long)]
        turboquant: bool,
    },
    /// Search Hugging Face for GGUF models
    Search {
        query: String,
    },
    /// Download a GGUF model from Hugging Face
    Download {
        model_id: String,
        file: String,
        #[arg(short, long, default_value = ".")]
        dest: String,
    },
    /// Start the llama.cpp server
    StartServer {
        #[arg(short, long)]
        model_path: String,
        #[arg(short, long, default_value_t = 4096)]
        ctx_size: u32,
        #[arg(short, long, default_value_t = 4)]
        threads: usize,
        #[arg(short, long, default_value_t = 0)]
        ngl: u32,
        #[arg(short, long)]
        flash_attention: bool,
    },
    /// Show system hardware information
    Hardware,
}

// ── Background events from async tasks ──────────────────────────────────────
enum BgEvent {
    SearchResults(Vec<huggingface::ModelMeta>),
    ModelFiles(Vec<String>),
    SearchError(String),
    BuildLog(String),
    BuildComplete,
    BuildError(String),
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    match &cli.command {
        Some(cmd) => handle_cli_command(cmd).await?,
        None      => run_tui().await?,
    }
    Ok(())
}

// ─────────────────────────────────────────────────────────────────────────────
//  CLI MODE
// ─────────────────────────────────────────────────────────────────────────────
async fn handle_cli_command(cmd: &Commands) -> Result<()> {
    match cmd {
        Commands::Build { backend, turboquant } => {
            println!("Building llama.cpp with backend: {}", backend);
            let b = match backend.as_str() {
                "cpu"    => builder::Backend::Cpu,
                "sycl"   => builder::Backend::Sycl,
                "vulkan" => builder::Backend::Vulkan,
                _        => builder::Backend::Cuda,
            };
            let opts = builder::BuildOptions { backend: b, use_turboquant: *turboquant, ..Default::default() };
            let source_dir = std::env::current_dir()?.join("llama.cpp");
            println!("Cloning repository...");
            builder::clone_repo(&opts, &source_dir).await?;
            println!("Building...");
            builder::build_llama_cpp(&opts, &source_dir).await?;
            println!("Build complete!");
        }
        Commands::Search { query } => {
            println!("Searching for GGUF models: {}...", query);
            let hf = huggingface::HuggingFaceClient::new();
            let models = hf.search_gguf_models(query).await.map_err(|e| anyhow::anyhow!("{}", e))?;
            for m in &models { println!("  - {}", m.model_id); }
            println!("{} results.", models.len());
        }
        Commands::Download { model_id, file, dest } => {
            println!("Downloading {} from {} → {}", file, model_id, dest);
            let hf = huggingface::HuggingFaceClient::new();
            let mut dest_path = PathBuf::from(dest);
            if dest_path.is_dir() { dest_path.push(file); }
            hf.download_file(model_id, file, dest_path).await.map_err(|e| anyhow::anyhow!("{}", e))?;
            println!("Download complete!");
        }
        Commands::StartServer { model_path, ctx_size, threads, ngl, flash_attention } => {
            println!("Starting server with model: {}", model_path);
            let mut orch = orchestrator::Orchestrator::new();
            orch.start_server(model_path, *ctx_size, *threads, *ngl, *flash_attention)
                .map_err(|e| anyhow::anyhow!("{}", e))?;
            if let Some(mut child) = orch.server_process.take() {
                let status = child.wait().await?;
                println!("Server exited with: {}", status);
            }
        }
        Commands::Hardware => {
            let info = hardware::get_system_info();
            println!("System Hardware:");
            println!("  CPU Cores:   {}", info.cpu_cores);
            println!("  Total RAM:   {} MB", info.total_memory / 1024 / 1024);
            println!("  Nvidia GPU:  {}", info.has_nvidia);
            if info.has_nvidia {
                println!("    VRAM:      {} MB total, {} MB free",
                    info.nvidia_vram_total / 1024 / 1024,
                    info.nvidia_vram_free  / 1024 / 1024);
            }
            println!("  Intel GPU:   {}", info.has_intel_gpu);
        }
    }
    Ok(())
}

// ─────────────────────────────────────────────────────────────────────────────
//  TUI MODE
// ─────────────────────────────────────────────────────────────────────────────
async fn run_tui() -> Result<()> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut app = App::new();
    let res = run_app(&mut terminal, &mut app).await;

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen, DisableMouseCapture)?;
    terminal.show_cursor()?;

    if let Err(err) = res { eprintln!("{:?}", err); }
    Ok(())
}

async fn run_app(
    terminal: &mut Terminal<CrosstermBackend<std::io::Stdout>>,
    app: &mut App,
) -> Result<()> {
    // Channel for background task results (HF search etc.)
    let (tx, rx) = mpsc::channel::<BgEvent>();

    loop {
        terminal.draw(|f| ui::draw(f, app))?;

        // Process any background results
        while let Ok(ev) = rx.try_recv() {
            match ev {
                BgEvent::SearchResults(results) => {
                    app.search_loading = false;
                    let count = results.len();
                    app.search_results = results;
                    if count > 0 {
                        app.search_results_state.select(Some(0));
                    }
                    app.push_toast(format!("✅ {} modelos encontrados", count));
                }
                BgEvent::ModelFiles(files) => {
                    app.model_files = files;
                    app.model_files_state.select(if app.model_files.is_empty() { None } else { Some(0) });
                    app.explorer_focus = app::ExplorerFocus::FileList;
                }
                BgEvent::SearchError(e) => {
                    app.search_loading = false;
                    app.push_toast(format!("❌ Error: {}", e));
                }
                BgEvent::BuildLog(msg) => {
                    app.push_log(msg);
                }
                BgEvent::BuildComplete => {
                    app.push_toast("✅ Compilación exitosa".to_string());
                    app.push_log("[INFO] Compilación exitosa de llama.cpp".to_string());
                }
                BgEvent::BuildError(e) => {
                    app.push_toast(format!("❌ Error de compilación: {}", e));
                    app.push_log(format!("[ERROR] {}", e));
                }
            }
        }

        // Trigger search if requested
        if app.search_loading && !app.search_input.value().is_empty() {
            let query = app.search_input.value().to_string();
            let tx2 = tx.clone();
            thread::spawn(move || {
                let rt = tokio::runtime::Runtime::new().unwrap();
                rt.block_on(async move {
                    let hf = huggingface::HuggingFaceClient::new();
                    match hf.search_gguf_models(&query).await {
                        Ok(r) => { let _ = tx2.send(BgEvent::SearchResults(r)); }
                        Err(e) => { let _ = tx2.send(BgEvent::SearchError(e.to_string())); }
                    }
                });
            });
            // Reset so we don't re-trigger
            app.search_loading = false;
        }

        if !event::poll(Duration::from_millis(100))? {
            continue;
        }
        let ev = event::read()?;
        let Event::Key(key) = ev else { continue };

        // ── When a text input has focus, most keys go straight to it ─────────
        if app.focus_is_text_input() {
            match key.code {
                KeyCode::Esc => {
                    // Esc: leave editing mode (for Wizard fields) or just unfocus
                    app.input_mode = InputMode::Normal;
                }
                KeyCode::Enter => {
                    // In search input: trigger search
                    if app.explorer_focus == app::ExplorerFocus::SearchInput {
                        app.push_toast(format!("🔍 Buscando: {}", app.search_input.value()));
                        app.search_loading = true;
                    } else {
                        app.input_mode = InputMode::Normal;
                    }
                }
                KeyCode::Tab => {
                    // Tab moves to next widget even from search input
                    app.input_mode = InputMode::Normal;
                    app.handle_tab_focus();
                }
                _ => {
                    app.handle_char_key(key);
                }
            }
            continue;
        }

        // ── Normal navigation mode ────────────────────────────────────────────
        match (key.code, key.modifiers) {
            // Quit only when NOT editing text
            (KeyCode::Char('q'), _) | (KeyCode::Esc, _) => return Ok(()),

            // Tab / Shift+Tab: cycle through sections within the current tab
            (KeyCode::Tab, KeyModifiers::NONE) => app.handle_tab_focus(),
            (KeyCode::BackTab, _)              => {
                // Shift+Tab: switch to previous top-level tab
                app.previous_tab();
            }

            // Left / Right: switch tabs
            (KeyCode::Left,  _) => app.previous_tab(),
            (KeyCode::Right, _) => app.next_tab(),
            (KeyCode::Char('1'), _) => app.current_tab = app::Tab::Builder,
            (KeyCode::Char('2'), _) => app.current_tab = app::Tab::Explorer,
            (KeyCode::Char('3'), _) => app.current_tab = app::Tab::Wizard,
            (KeyCode::Char('4'), _) => app.current_tab = app::Tab::Dashboard,

            // Up / Down: navigate within list widgets of the current section
            (KeyCode::Down, _) => app.handle_down(),
            (KeyCode::Up,   _) => app.handle_up(),

            // Enter: confirm / activate
            (KeyCode::Enter, _) => {
                let is_build_btn = app.current_tab == app::Tab::Builder && app.builder_focus == app::BuilderFocus::BuildBtn;

                app.handle_enter();

                if is_build_btn {
                    let tx2 = tx.clone();
                    let backend_idx = app.builder_backend_state.selected().unwrap_or(0);
                    let backend_str = app.builder_backends[backend_idx];
                    let backend = if backend_str.contains("CUDA") {
                        builder::Backend::Cuda
                    } else if backend_str.contains("SYCL") {
                        builder::Backend::Sycl
                    } else if backend_str.contains("Vulkan") {
                        builder::Backend::Vulkan
                    } else {
                        builder::Backend::Cpu
                    };
                    
                    let use_tq = app.builder_turboquant;
                    
                    thread::spawn(move || {
                        let rt = tokio::runtime::Runtime::new().unwrap();
                        rt.block_on(async move {
                            let _ = tx2.send(BgEvent::BuildLog("Clonando repositorio...".to_string()));
                            let opts = builder::BuildOptions { backend, use_turboquant: use_tq, ..Default::default() };
                            let source_dir = std::env::current_dir().unwrap().join("llama.cpp");
                            
                            match builder::clone_repo(&opts, &source_dir).await {
                                Ok(_) => {
                                    let _ = tx2.send(BgEvent::BuildLog("Repositorio listo. Iniciando compilación...".to_string()));
                                    match builder::build_llama_cpp(&opts, &source_dir).await {
                                        Ok(_) => { let _ = tx2.send(BgEvent::BuildComplete); }
                                        Err(e) => { let _ = tx2.send(BgEvent::BuildError(e.to_string())); }
                                    }
                                }
                                Err(e) => { let _ = tx2.send(BgEvent::BuildError(e.to_string())); }
                            }
                        });
                    });
                }

                // If search was triggered, fire it
                if app.search_loading {
                    let query = app.search_input.value().to_string();
                    let tx2 = tx.clone();
                    thread::spawn(move || {
                        let rt = tokio::runtime::Runtime::new().unwrap();
                        rt.block_on(async move {
                            let hf = huggingface::HuggingFaceClient::new();
                            match hf.search_gguf_models(&query).await {
                                Ok(r) => { let _ = tx2.send(BgEvent::SearchResults(r)); }
                                Err(e) => { let _ = tx2.send(BgEvent::SearchError(e.to_string())); }
                            }
                        });
                    });
                    app.search_loading = false; // thread handles it
                }

                // If a model is selected in results, fetch its files
                if app.current_tab == app::Tab::Explorer
                    && app.explorer_focus == app::ExplorerFocus::ResultsList
                {
                    if let Some(idx) = app.search_results_state.selected() {
                        if let Some(model) = app.search_results.get(idx) {
                            let model_id = model.model_id.clone();
                            let tx2 = tx.clone();
                            thread::spawn(move || {
                                let rt = tokio::runtime::Runtime::new().unwrap();
                                rt.block_on(async move {
                                    let hf = huggingface::HuggingFaceClient::new();
                                    match hf.get_model_files(&model_id).await {
                                        Ok(files) => { let _ = tx2.send(BgEvent::ModelFiles(files)); }
                                        Err(e)    => { let _ = tx2.send(BgEvent::SearchError(e.to_string())); }
                                    }
                                });
                            });
                        }
                    }
                }
            }

            // Space: same as Enter for toggles
            (KeyCode::Char(' '), _) => {
                app.handle_enter();
            }

            _ => {}
        }
    }
}
