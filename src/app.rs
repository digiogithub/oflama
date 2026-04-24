use ratatui::widgets::ListState;
use tui_input::Input;
use tui_input::backend::crossterm::EventHandler;
use crossterm::event::KeyEvent;
use crate::huggingface::ModelMeta;
use crate::hardware::{SystemInfo, get_system_info};
use std::collections::HashSet;

#[derive(Clone, Copy, PartialEq)]
pub enum Tab {
    Builder,
    Explorer,
    Wizard,
    Dashboard,
}

#[derive(Clone, Copy, PartialEq)]
pub enum BuilderFocus { Backend, TurboQuant, BuildBtn }

#[derive(Clone, Copy, PartialEq)]
pub enum ExplorerFocus { SearchInput, ResultsList, FileList, DownloadBtn }

#[derive(Clone, Copy, PartialEq)]
pub enum WizardFocus { ModelList, CtxSize, Ngl, FlashAttn, StartBtn }

/// When true, the focused input widget is capturing all keypresses
pub enum InputMode {
    Normal,
    Editing,
}

pub struct App {
    pub current_tab: Tab,
    pub input_mode: InputMode,
    pub notifications: Vec<String>,

    // Builder State
    pub builder_focus: BuilderFocus,
    pub builder_backends: Vec<&'static str>,
    pub builder_backend_state: ListState,
    /// Set of selected backend indices (multi-select)
    pub builder_selected_backends: HashSet<usize>,
    pub builder_turboquant: bool,
    pub builder_log: Vec<String>,

    // Explorer State
    pub explorer_focus: ExplorerFocus,
    pub search_input: Input,
    pub search_results: Vec<ModelMeta>,
    pub search_results_state: ListState,
    /// Files (quantizations) available for the currently selected model
    pub model_files: Vec<String>,
    pub model_files_state: ListState,
    pub search_loading: bool,

    // Wizard State
    pub wizard_focus: WizardFocus,
    pub wizard_models: Vec<String>,
    pub wizard_models_state: ListState,
    pub wizard_ctx_size: Input,
    pub wizard_ngl: Input,
    pub wizard_flash_attn: bool,

    // Dashboard / System Info
    pub sys_info: Option<SystemInfo>,
}

impl App {
    pub fn new() -> Self {
        let mut backend_state = ListState::default();
        backend_state.select(Some(0));

        let mut selected = HashSet::new();
        selected.insert(0); // CUDA selected by default

        let mut models_state = ListState::default();
        models_state.select(Some(0));

        let mut search_results_state = ListState::default();
        search_results_state.select(None);

        let mut model_files_state = ListState::default();
        model_files_state.select(None);

        Self {
            current_tab: Tab::Builder,
            input_mode: InputMode::Normal,
            notifications: Vec::new(),

            builder_focus: BuilderFocus::Backend,
            builder_backends: vec!["CUDA (Nvidia)", "SYCL (Intel iGPU/Arc)", "Vulkan (AMD/Intel)", "CPU (sin GPU)"],
            builder_backend_state: backend_state,
            builder_selected_backends: selected,
            builder_turboquant: false,
            builder_log: Vec::new(),

            explorer_focus: ExplorerFocus::SearchInput,
            search_input: Input::default(),
            search_results: Vec::new(),
            search_results_state,
            model_files: Vec::new(),
            model_files_state,
            search_loading: false,

            wizard_focus: WizardFocus::ModelList,
            wizard_models: Vec::new(),
            wizard_models_state: models_state,
            wizard_ctx_size: Input::default().with_value("4096".to_string()),
            wizard_ngl: Input::default().with_value("33".to_string()),
            wizard_flash_attn: false,

            sys_info: Some(get_system_info()),
        }
    }

    pub fn is_editing(&self) -> bool {
        matches!(self.input_mode, InputMode::Editing)
    }

    pub fn push_toast(&mut self, message: String) {
        self.notifications.push(message);
        if self.notifications.len() > 5 {
            self.notifications.remove(0);
        }
    }

    pub fn push_log(&mut self, message: String) {
        self.builder_log.push(message);
        if self.builder_log.len() > 100 {
            self.builder_log.remove(0);
        }
    }

    pub fn next_tab(&mut self) {
        self.input_mode = InputMode::Normal;
        self.current_tab = match self.current_tab {
            Tab::Builder => Tab::Explorer,
            Tab::Explorer => Tab::Wizard,
            Tab::Wizard => Tab::Dashboard,
            Tab::Dashboard => Tab::Builder,
        };
    }

    pub fn previous_tab(&mut self) {
        self.input_mode = InputMode::Normal;
        self.current_tab = match self.current_tab {
            Tab::Builder => Tab::Dashboard,
            Tab::Dashboard => Tab::Wizard,
            Tab::Wizard => Tab::Explorer,
            Tab::Explorer => Tab::Builder,
        };
    }

    // ─── Navigation within tabs ─────────────────────────────────────────────

    pub fn handle_down(&mut self) {
        match self.current_tab {
            Tab::Builder => {
                match self.builder_focus {
                    BuilderFocus::Backend => {
                        let i = self.builder_backend_state.selected().unwrap_or(0);
                        let next = (i + 1) % self.builder_backends.len();
                        self.builder_backend_state.select(Some(next));
                    }
                    BuilderFocus::TurboQuant => self.builder_focus = BuilderFocus::BuildBtn,
                    BuilderFocus::BuildBtn => {
                        self.builder_focus = BuilderFocus::Backend;
                    }
                }
            }
            Tab::Explorer => {
                match self.explorer_focus {
                    ExplorerFocus::SearchInput => self.explorer_focus = ExplorerFocus::ResultsList,
                    ExplorerFocus::ResultsList => {
                        let len = self.search_results.len();
                        if len > 0 {
                            let i = self.search_results_state.selected().unwrap_or(0);
                            let next = (i + 1) % len;
                            self.search_results_state.select(Some(next));
                        }
                    }
                    ExplorerFocus::FileList => {
                        let len = self.model_files.len();
                        if len > 0 {
                            let i = self.model_files_state.selected().unwrap_or(0);
                            let next = (i + 1) % len;
                            self.model_files_state.select(Some(next));
                        }
                    }
                    ExplorerFocus::DownloadBtn => self.explorer_focus = ExplorerFocus::SearchInput,
                }
            }
            Tab::Wizard => {
                match self.wizard_focus {
                    WizardFocus::ModelList => {
                        let len = self.wizard_models.len();
                        if len > 0 {
                            let i = self.wizard_models_state.selected().unwrap_or(0);
                            let next = (i + 1) % len;
                            self.wizard_models_state.select(Some(next));
                        }
                    }
                    WizardFocus::CtxSize => self.wizard_focus = WizardFocus::Ngl,
                    WizardFocus::Ngl => self.wizard_focus = WizardFocus::FlashAttn,
                    WizardFocus::FlashAttn => self.wizard_focus = WizardFocus::StartBtn,
                    WizardFocus::StartBtn => self.wizard_focus = WizardFocus::ModelList,
                }
            }
            Tab::Dashboard => {}
        }
    }

    pub fn handle_up(&mut self) {
        match self.current_tab {
            Tab::Builder => {
                match self.builder_focus {
                    BuilderFocus::Backend => {
                        let i = self.builder_backend_state.selected().unwrap_or(0);
                        let next = if i == 0 { self.builder_backends.len() - 1 } else { i - 1 };
                        self.builder_backend_state.select(Some(next));
                    }
                    BuilderFocus::TurboQuant => self.builder_focus = BuilderFocus::Backend,
                    BuilderFocus::BuildBtn => self.builder_focus = BuilderFocus::TurboQuant,
                }
            }
            Tab::Explorer => {
                match self.explorer_focus {
                    ExplorerFocus::SearchInput => self.explorer_focus = ExplorerFocus::DownloadBtn,
                    ExplorerFocus::ResultsList => {
                        let len = self.search_results.len();
                        if len > 0 {
                            let i = self.search_results_state.selected().unwrap_or(0);
                            let next = if i == 0 { len - 1 } else { i - 1 };
                            self.search_results_state.select(Some(next));
                        }
                    }
                    ExplorerFocus::FileList => {
                        let len = self.model_files.len();
                        if len > 0 {
                            let i = self.model_files_state.selected().unwrap_or(0);
                            let next = if i == 0 { len - 1 } else { i - 1 };
                            self.model_files_state.select(Some(next));
                        }
                    }
                    ExplorerFocus::DownloadBtn => self.explorer_focus = ExplorerFocus::FileList,
                }
            }
            Tab::Wizard => {
                match self.wizard_focus {
                    WizardFocus::ModelList => {
                        let len = self.wizard_models.len();
                        if len > 0 {
                            let i = self.wizard_models_state.selected().unwrap_or(0);
                            let next = if i == 0 { len - 1 } else { i - 1 };
                            self.wizard_models_state.select(Some(next));
                        }
                    }
                    WizardFocus::CtxSize => self.wizard_focus = WizardFocus::ModelList,
                    WizardFocus::Ngl => self.wizard_focus = WizardFocus::CtxSize,
                    WizardFocus::FlashAttn => self.wizard_focus = WizardFocus::Ngl,
                    WizardFocus::StartBtn => self.wizard_focus = WizardFocus::FlashAttn,
                }
            }
            Tab::Dashboard => {}
        }
    }

    pub fn handle_tab_focus(&mut self) {
        // Tab within builder moves between section blocks (Backend / TurboQuant / BuildBtn)
        match self.current_tab {
            Tab::Builder => {
                self.builder_focus = match self.builder_focus {
                    BuilderFocus::Backend => BuilderFocus::TurboQuant,
                    BuilderFocus::TurboQuant => BuilderFocus::BuildBtn,
                    BuilderFocus::BuildBtn => BuilderFocus::Backend,
                };
            }
            Tab::Explorer => {
                self.explorer_focus = match self.explorer_focus {
                    ExplorerFocus::SearchInput => ExplorerFocus::ResultsList,
                    ExplorerFocus::ResultsList => ExplorerFocus::FileList,
                    ExplorerFocus::FileList => ExplorerFocus::DownloadBtn,
                    ExplorerFocus::DownloadBtn => ExplorerFocus::SearchInput,
                };
            }
            Tab::Wizard => {
                self.wizard_focus = match self.wizard_focus {
                    WizardFocus::ModelList => WizardFocus::CtxSize,
                    WizardFocus::CtxSize => WizardFocus::Ngl,
                    WizardFocus::Ngl => WizardFocus::FlashAttn,
                    WizardFocus::FlashAttn => WizardFocus::StartBtn,
                    WizardFocus::StartBtn => WizardFocus::ModelList,
                };
            }
            Tab::Dashboard => {}
        }
    }

    pub fn handle_enter(&mut self) {
        match self.current_tab {
            Tab::Builder => match self.builder_focus {
                BuilderFocus::Backend => {
                    // Toggle selected state of the highlighted backend
                    if let Some(i) = self.builder_backend_state.selected() {
                        if self.builder_selected_backends.contains(&i) {
                            self.builder_selected_backends.remove(&i);
                        } else {
                            self.builder_selected_backends.insert(i);
                        }
                    }
                }
                BuilderFocus::TurboQuant => {
                    self.builder_turboquant = !self.builder_turboquant;
                }
                BuilderFocus::BuildBtn => {
                    self.push_toast("🔨 Compilación encolada...".to_string());
                    self.push_log("[INFO] Iniciando compilación de llama.cpp...".to_string());
                }
            },
            Tab::Explorer => match self.explorer_focus {
                ExplorerFocus::SearchInput => {
                    // Enter in text input triggers search
                    self.push_toast(format!("🔍 Buscando: {}", self.search_input.value()));
                    // Actual async search triggered from main loop via flag
                    self.search_loading = true;
                }
                ExplorerFocus::ResultsList => {
                    // Select model → show file list
                    if !self.search_results.is_empty() {
                        self.explorer_focus = ExplorerFocus::FileList;
                        self.model_files_state.select(Some(0));
                    }
                }
                ExplorerFocus::FileList => {
                    // Enter on file → go to download button
                    self.explorer_focus = ExplorerFocus::DownloadBtn;
                }
                ExplorerFocus::DownloadBtn => {
                    self.push_toast("⬇️  Descarga iniciada...".to_string());
                }
            },
            Tab::Wizard => match self.wizard_focus {
                WizardFocus::CtxSize | WizardFocus::Ngl => {
                    // Enter starts editing the text field
                    self.input_mode = InputMode::Editing;
                }
                WizardFocus::FlashAttn => {
                    self.wizard_flash_attn = !self.wizard_flash_attn;
                }
                WizardFocus::StartBtn => {
                    self.push_toast("🚀 Iniciando llama-server...".to_string());
                }
                _ => {}
            },
            Tab::Dashboard => {}
        }
    }

    /// Feed key to the focused text input (only when InputMode::Editing)
    pub fn handle_char_key(&mut self, key: KeyEvent) {
        match self.current_tab {
            Tab::Explorer if self.explorer_focus == ExplorerFocus::SearchInput => {
                self.search_input.handle_event(&crossterm::event::Event::Key(key));
            }
            Tab::Wizard if self.wizard_focus == WizardFocus::CtxSize => {
                self.wizard_ctx_size.handle_event(&crossterm::event::Event::Key(key));
            }
            Tab::Wizard if self.wizard_focus == WizardFocus::Ngl => {
                self.wizard_ngl.handle_event(&crossterm::event::Event::Key(key));
            }
            _ => {}
        }
    }

    /// Returns true if the current focus is a text-entry field (always capturing keys)
    pub fn focus_is_text_input(&self) -> bool {
        matches!(
            (&self.current_tab, &self.explorer_focus, &self.wizard_focus, &self.input_mode),
            (Tab::Explorer, ExplorerFocus::SearchInput, _, _)
            | (Tab::Wizard, _, WizardFocus::CtxSize, InputMode::Editing)
            | (Tab::Wizard, _, WizardFocus::Ngl, InputMode::Editing)
        )
    }
}
