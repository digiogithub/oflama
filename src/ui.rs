use crate::app::{App, Tab, BuilderFocus, ExplorerFocus, WizardFocus};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Tabs, List, ListItem, Wrap},
    Frame,
};

const FOCUSED: Style = Style::new().fg(Color::Yellow);
const ACTIVE_BG: Style = Style::new().bg(Color::DarkGray).add_modifier(Modifier::BOLD);
const HELP_STYLE: Style = Style::new().fg(Color::DarkGray);

fn focused_block(title: &str, focused: bool) -> Block {
    let b = Block::default().borders(Borders::ALL).title(title);
    if focused { b.border_style(FOCUSED) } else { b }
}

pub fn draw(f: &mut Frame, app: &mut App) {
    let area = f.area();

    // ── Status bar height: always 1 line at bottom ──
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // tab bar
            Constraint::Min(0),    // content
            Constraint::Length(1), // help bar
        ])
        .split(area);

    // ── Tab bar ──────────────────────────────────────
    let tab_titles: Vec<Line> = vec![
        "  1. Compilación  ",
        "  2. Explorador HF  ",
        "  3. Asistente (Launch)  ",
        "  4. Dashboard  ",
    ]
    .into_iter()
    .map(Line::from)
    .collect();

    let tab_index = match app.current_tab {
        Tab::Builder => 0,
        Tab::Explorer => 1,
        Tab::Wizard => 2,
        Tab::Dashboard => 3,
    };

    let tabs = Tabs::new(tab_titles)
        .block(Block::default().borders(Borders::ALL).title(" ⚡ Oflama – Llama.cpp Manager "))
        .select(tab_index)
        .style(Style::default().fg(Color::Cyan))
        .highlight_style(Style::default().add_modifier(Modifier::BOLD).fg(Color::Yellow));

    f.render_widget(tabs, chunks[0]);

    // ── Content area: split for optional notification panel ──
    let n = app.notifications.len() as u16;
    let content_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(0),
            Constraint::Length(if n == 0 { 0 } else { n + 2 }),
        ])
        .split(chunks[1]);

    match app.current_tab {
        Tab::Builder  => draw_builder_tab(f, app, content_chunks[0]),
        Tab::Explorer => draw_explorer_tab(f, app, content_chunks[0]),
        Tab::Wizard   => draw_wizard_tab(f, app, content_chunks[0]),
        Tab::Dashboard => draw_dashboard_tab(f, app, content_chunks[0]),
    }

    if n > 0 {
        let text = app.notifications.join("\n");
        let widget = Paragraph::new(text)
            .style(Style::default().fg(Color::LightGreen))
            .block(Block::default().title(" Notificaciones ").borders(Borders::ALL));
        f.render_widget(widget, content_chunks[1]);
    }

    // ── Help bar ─────────────────────────────────────
    let help = help_line(app);
    f.render_widget(Paragraph::new(help).style(HELP_STYLE), chunks[2]);
}

fn help_line(app: &App) -> String {
    if app.focus_is_text_input() {
        match app.current_tab {
            Tab::Explorer => "  ESC salir edición  ·  INTRO buscar  ·  ↕ navegar resultados".to_string(),
            _ => "  ESC confirmar  ·  escribe para editar".to_string(),
        }
    } else {
        "  ← → Tab: cambiar pestaña  ·  ↕: navegar  ·  Tab: siguiente sección  ·  INTRO: seleccionar/ejecutar  ·  ESC/q: salir".to_string()
    }
}

// ════════════════════════════════════════════════════
//  TAB 1 – Builder
// ════════════════════════════════════════════════════
fn draw_builder_tab(f: &mut Frame, app: &mut App, area: Rect) {
    let layout = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(45), Constraint::Percentage(55)])
        .split(area);

    // ── Left panel ───────────────────────────────────
    let left = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(8), // backend list
            Constraint::Length(3), // turboquant checkbox
            Constraint::Length(3), // build button
            Constraint::Min(0),
        ])
        .split(layout[0]);

    // Backend multi-select list
    let focused_backend = app.builder_focus == BuilderFocus::Backend;
    let items: Vec<ListItem> = app.builder_backends.iter().enumerate().map(|(i, &b)| {
        let checked = if app.builder_selected_backends.contains(&i) { "◉ " } else { "○ " };
        let style = if app.builder_selected_backends.contains(&i) {
            Style::default().fg(Color::LightGreen).add_modifier(Modifier::BOLD)
        } else {
            Style::default()
        };
        ListItem::new(format!("{}{}", checked, b)).style(style)
    }).collect();

    let list = List::new(items)
        .block(focused_block(" Backend de GPU (ESPACIO/INTRO para seleccionar) ", focused_backend))
        .highlight_style(Style::default().bg(Color::DarkGray).add_modifier(Modifier::BOLD))
        .highlight_symbol("▶ ");
    f.render_stateful_widget(list, left[0], &mut app.builder_backend_state);

    // TurboQuant
    let tq = if app.builder_turboquant { "  ◉  TurboQuant  (activado)" } else { "  ○  TurboQuant  (desactivado)" };
    let tq_style = if app.builder_turboquant { Style::default().fg(Color::LightGreen) } else { Style::default() };
    let tq_focused = app.builder_focus == BuilderFocus::TurboQuant;
    f.render_widget(
        Paragraph::new(Span::styled(tq, tq_style)).block(focused_block(" Opciones Extra ", tq_focused)),
        left[1],
    );

    // Build button
    let btn_focused = app.builder_focus == BuilderFocus::BuildBtn;
    let btn_style = if btn_focused {
        Style::default().fg(Color::Black).bg(Color::Yellow).add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(Color::Black).bg(Color::Blue)
    };
    f.render_widget(
        Paragraph::new(Span::styled("  🔨  INTRO → Compilar llama.cpp  ", btn_style))
            .block(Block::default().borders(Borders::ALL)),
        left[2],
    );

    // ── Right panel: log ─────────────────────────────
    let log_text = if app.builder_log.is_empty() {
        "Selecciona backends, ajusta opciones y presiona Compilar.\n\nBacks disponibles:\n  CUDA  → tarjetas Nvidia\n  SYCL  → Intel iGPU / Arc (requiere oneAPI)\n  Vulkan → AMD / Intel genérico\n  CPU   → sin aceleración GPU".to_string()
    } else {
        app.builder_log.join("\n")
    };
    f.render_widget(
        Paragraph::new(log_text)
            .wrap(Wrap { trim: false })
            .block(Block::default().borders(Borders::ALL).title(" Log de Compilación ")),
        layout[1],
    );
}

// ════════════════════════════════════════════════════
//  TAB 2 – HuggingFace Explorer
// ════════════════════════════════════════════════════
fn draw_explorer_tab(f: &mut Frame, app: &mut App, area: Rect) {
    let layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Search input
            Constraint::Min(0),    // Two columns: model list + file list
            Constraint::Length(3), // Download button
        ])
        .split(area);

    // ── Search bar ───────────────────────────────────
    let search_focused = app.explorer_focus == ExplorerFocus::SearchInput;
    let search_block = focused_block(
        if search_focused { " 🔍 Buscar modelos GGUF en HuggingFace (escribe y pulsa INTRO) " }
        else              { " 🔍 Buscar modelos GGUF en HuggingFace " },
        search_focused,
    );
    let input_val = app.search_input.value();
    let cursor_pos = app.search_input.cursor() as u16;
    f.render_widget(Paragraph::new(input_val).block(search_block), layout[0]);
    if search_focused {
        f.set_cursor_position((layout[0].x + cursor_pos + 1, layout[0].y + 1));
    }

    // ── Results + File list ──────────────────────────
    let cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(layout[1]);

    // Model results list
    let results_focused = app.explorer_focus == ExplorerFocus::ResultsList;
    let results: Vec<ListItem> = if app.search_loading {
        vec![ListItem::new("  ⏳ Buscando…").style(Style::default().fg(Color::Yellow))]
    } else if app.search_results.is_empty() {
        vec![ListItem::new("  (sin resultados)").style(Style::default().fg(Color::DarkGray))]
    } else {
        app.search_results.iter().map(|m| ListItem::new(format!("  {}", m.model_id))).collect()
    };
    let model_list = List::new(results)
        .block(focused_block(" Modelos (↕ navegar · INTRO expandir archivos) ", results_focused))
        .highlight_style(ACTIVE_BG)
        .highlight_symbol("▶ ");
    f.render_stateful_widget(model_list, cols[0], &mut app.search_results_state);

    // GGUF file list (quantizations)
    let files_focused = app.explorer_focus == ExplorerFocus::FileList;
    let file_items: Vec<ListItem> = if app.model_files.is_empty() {
        vec![ListItem::new("  Selecciona un modelo →").style(Style::default().fg(Color::DarkGray))]
    } else {
        app.model_files.iter().map(|f| ListItem::new(format!("  {}", f))).collect()
    };
    let file_list = List::new(file_items)
        .block(focused_block(" Cuantizaciones GGUF (↕ navegar · INTRO seleccionar) ", files_focused))
        .highlight_style(ACTIVE_BG)
        .highlight_symbol("▶ ");
    f.render_stateful_widget(file_list, cols[1], &mut app.model_files_state);

    // ── Download button ──────────────────────────────
    let dl_focused = app.explorer_focus == ExplorerFocus::DownloadBtn;
    let dl_style = if dl_focused {
        Style::default().fg(Color::Black).bg(Color::LightGreen).add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(Color::Black).bg(Color::Green)
    };
    f.render_widget(
        Paragraph::new(Span::styled("  ⬇️   INTRO → Descargar cuantización seleccionada  ", dl_style))
            .block(Block::default().borders(Borders::ALL)),
        layout[2],
    );
}

// ════════════════════════════════════════════════════
//  TAB 3 – Launch Wizard
// ════════════════════════════════════════════════════
fn draw_wizard_tab(f: &mut Frame, app: &mut App, area: Rect) {
    let layout = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(45), Constraint::Percentage(55)])
        .split(area);

    // ── Left: config form ────────────────────────────
    let left = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(0),    // model list
            Constraint::Length(3), // ctx size
            Constraint::Length(3), // ngl
            Constraint::Length(3), // flash attn
            Constraint::Length(3), // start button
        ])
        .split(layout[0]);

    // Model list
    let ml_focused = app.wizard_focus == WizardFocus::ModelList;
    let model_items: Vec<ListItem> = if app.wizard_models.is_empty() {
        vec![ListItem::new("  (sin modelos descargados)").style(Style::default().fg(Color::DarkGray))]
    } else {
        app.wizard_models.iter().map(|m| ListItem::new(format!("  {}", m))).collect()
    };
    let model_list = List::new(model_items)
        .block(focused_block(" 1. Modelo local (↕ navegar) ", ml_focused))
        .highlight_style(ACTIVE_BG)
        .highlight_symbol("▶ ");
    f.render_stateful_widget(model_list, left[0], &mut app.wizard_models_state);

    // ctx_size
    let ctx_focused = app.wizard_focus == WizardFocus::CtxSize;
    let editing_ctx = ctx_focused && app.is_editing();
    let ctx_title = if editing_ctx { " 2. Contexto -c (editando — ESC para confirmar) " } else { " 2. Contexto -c (INTRO para editar) " };
    f.render_widget(Paragraph::new(app.wizard_ctx_size.value()).block(focused_block(ctx_title, ctx_focused)), left[1]);
    if editing_ctx { f.set_cursor_position((left[1].x + app.wizard_ctx_size.cursor() as u16 + 1, left[1].y + 1)); }

    // ngl
    let ngl_focused = app.wizard_focus == WizardFocus::Ngl;
    let editing_ngl = ngl_focused && app.is_editing();
    let ngl_title = if editing_ngl { " 3. Capas GPU -ngl (editando — ESC para confirmar) " } else { " 3. Capas GPU -ngl (INTRO para editar) " };
    f.render_widget(Paragraph::new(app.wizard_ngl.value()).block(focused_block(ngl_title, ngl_focused)), left[2]);
    if editing_ngl { f.set_cursor_position((left[2].x + app.wizard_ngl.cursor() as u16 + 1, left[2].y + 1)); }

    // Flash attention toggle
    let fa_focused = app.wizard_focus == WizardFocus::FlashAttn;
    let fa_text = if app.wizard_flash_attn { "  ◉  Flash Attention -fa  (activado)" } else { "  ○  Flash Attention -fa  (desactivado)" };
    let fa_style = if app.wizard_flash_attn { Style::default().fg(Color::LightGreen) } else { Style::default() };
    f.render_widget(Paragraph::new(Span::styled(fa_text, fa_style)).block(focused_block(" 4. Flash Attention ", fa_focused)), left[3]);

    // Start button
    let sb_focused = app.wizard_focus == WizardFocus::StartBtn;
    let sb_style = if sb_focused {
        Style::default().fg(Color::Black).bg(Color::LightRed).add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(Color::Black).bg(Color::Red)
    };
    f.render_widget(
        Paragraph::new(Span::styled("  🚀  INTRO → Iniciar llama-server  ", sb_style))
            .block(Block::default().borders(Borders::ALL)),
        left[4],
    );

    // ── Right: preview command ────────────────────────
    let model_name = app.wizard_models_state.selected()
        .and_then(|i| app.wizard_models.get(i))
        .cloned()
        .unwrap_or_else(|| "<ninguno>".to_string());
    let ctx = app.wizard_ctx_size.value();
    let ngl = app.wizard_ngl.value();
    let fa  = if app.wizard_flash_attn { " -fa" } else { "" };
    let fa_sep = if fa.is_empty() { " \\\n  ".to_string() } else { format!("{} \\\n  ", fa) };
    let cmd = format!(
        "llama-server \\\n  -m {} \\\n  -c {} \\\n  -ngl {} \\\n  --jinja{}\\\n  --host 0.0.0.0 \\\n  --port 8080",
        model_name, ctx, ngl, fa_sep
    );
    f.render_widget(
        Paragraph::new(cmd)
            .wrap(Wrap { trim: false })
            .block(Block::default().borders(Borders::ALL).title(" Comando que se ejecutará ")),
        layout[1],
    );
}

// ════════════════════════════════════════════════════
//  TAB 4 – Dashboard
// ════════════════════════════════════════════════════
fn draw_dashboard_tab(f: &mut Frame, app: &mut App, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(area);

    // System info
    let info_text = if let Some(sys) = &app.sys_info {
        let nvidia = if sys.has_nvidia {
            format!("  Nvidia GPU:     ✓\n    VRAM total:   {} MB\n    VRAM libre:   {} MB",
                sys.nvidia_vram_total / 1024 / 1024,
                sys.nvidia_vram_free  / 1024 / 1024)
        } else {
            "  Nvidia GPU:     ✗".to_string()
        };
        let intel = if sys.has_intel_gpu { "  Intel GPU/iGPU: ✓" } else { "  Intel GPU/iGPU: ✗" };
        format!(
            "CPU Cores:       {}\nRAM Total:       {} MB\nRAM Usada:       {} MB\n\n{}\n\n{}",
            sys.cpu_cores,
            sys.total_memory / 1024 / 1024,
            sys.used_memory  / 1024 / 1024,
            nvidia,
            intel,
        )
    } else {
        "Cargando...".to_string()
    };

    f.render_widget(
        Paragraph::new(info_text)
            .wrap(Wrap { trim: false })
            .block(Block::default().borders(Borders::ALL).title(" Hardware detectado ")),
        chunks[0],
    );

    // Server status
    let srv_text = "Estado del servidor: Detenido\n\nUsa la pestaña 3. Asistente para\nconfigurarlo y arrancarlo.\n\nEl servidor expone una API\ncompatible con OpenAI en\nhttp://localhost:8080/v1";
    f.render_widget(
        Paragraph::new(srv_text)
            .wrap(Wrap { trim: false })
            .block(Block::default().borders(Borders::ALL).title(" Servidor llama-server ")),
        chunks[1],
    );
}
