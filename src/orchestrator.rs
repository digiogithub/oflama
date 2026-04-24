#![allow(dead_code)]
use tokio::process::{Child, Command};
use std::process::Stdio;
use std::error::Error;

pub struct Orchestrator {
    pub server_process: Option<Child>,
}

impl Orchestrator {
    pub fn new() -> Self {
        Self {
            server_process: None,
        }
    }

    /// Calculate how many layers can fit into VRAM
    /// Heuristic: assume each parameter takes roughly 0.6 bytes (e.g. Q4), + context memory
    pub fn suggest_offload_layers(vram_free: u64, model_size_bytes: u64, total_layers: u32) -> u32 {
        if vram_free == 0 {
            return 0;
        }
        
        let safe_vram = vram_free.saturating_sub(1024 * 1024 * 1024); // leave 1GB free
        if safe_vram >= model_size_bytes {
            return total_layers; // Fit entirely in VRAM
        }
        
        let bytes_per_layer = model_size_bytes / total_layers as u64;
        let layers = safe_vram / bytes_per_layer.max(1);
        
        layers.min(total_layers as u64) as u32
    }

    pub fn start_server(
        &mut self,
        model_path: &str,
        ctx_size: u32,
        threads: usize,
        ngl: u32,
        flash_attention: bool,
    ) -> Result<(), Box<dyn Error>> {
        if self.server_process.is_some() {
            return Err("Server is already running".into());
        }

        let mut cmd = Command::new("./llama-server"); // Assuming it's built in current dir
        cmd.arg("-m")
           .arg(model_path)
           .arg("-c")
           .arg(ctx_size.to_string())
           .arg("-t")
           .arg(threads.to_string())
           .arg("-ngl")
           .arg(ngl.to_string())
           .arg("--jinja"); // Tool calling support

        if flash_attention {
            cmd.arg("-fa");
        }

        cmd.stdout(Stdio::piped());
        cmd.stderr(Stdio::piped());

        let child = cmd.spawn()?;
        self.server_process = Some(child);

        Ok(())
    }

    pub fn stop_server(&mut self) -> Result<(), Box<dyn Error>> {
        if let Some(mut child) = self.server_process.take() {
            // Attempt to kill it
            let _ = child.start_kill();
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_suggest_offload_layers() {
        let vram_free = 8 * 1024 * 1024 * 1024; // 8GB
        let model_size = 4 * 1024 * 1024 * 1024; // 4GB
        let total_layers = 32;

        let layers = Orchestrator::suggest_offload_layers(vram_free, model_size, total_layers);
        // VRAM is enough for the whole model even with 1GB buffer
        assert_eq!(layers, 32);

        let vram_free_small = 2 * 1024 * 1024 * 1024; // 2GB
        let layers_small = Orchestrator::suggest_offload_layers(vram_free_small, model_size, total_layers);
        // 1GB safe vram. bytes_per_layer = 4GB / 32 = 128MB.
        // 1GB / 128MB = 8 layers.
        assert_eq!(layers_small, 8);
    }
}
