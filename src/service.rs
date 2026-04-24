#![allow(dead_code)]
use std::error::Error;
use std::fs;

pub struct SystemdManager;

impl SystemdManager {
    pub fn generate_service_file(
        model_path: &str,
        ctx_size: u32,
        threads: usize,
        ngl: u32,
        flash_attention: bool,
    ) -> Result<String, Box<dyn Error>> {
        let current_dir = std::env::current_dir()?;
        let llama_server_path = current_dir.join("llama-server").to_string_lossy().to_string();

        let mut exec_start = format!(
            "{} -m {} -c {} -t {} -ngl {} --jinja",
            llama_server_path, model_path, ctx_size, threads, ngl
        );

        if flash_attention {
            exec_start.push_str(" -fa");
        }

        let service_content = format!(
            "[Unit]\n\
            Description=Oflama Llama.cpp Server\n\
            After=network.target\n\
            \n\
            [Service]\n\
            Type=simple\n\
            ExecStart={}\n\
            Restart=always\n\
            RestartSec=3\n\
            \n\
            [Install]\n\
            WantedBy=default.target\n",
            exec_start
        );

        Ok(service_content)
    }

    pub fn install_user_service(service_content: &str) -> Result<(), Box<dyn Error>> {
        let home_dir = dirs::home_dir().ok_or("Could not find home directory")?;
        let systemd_dir = home_dir.join(".config").join("systemd").join("user");
        
        fs::create_dir_all(&systemd_dir)?;
        
        let service_path = systemd_dir.join("oflama-server.service");
        fs::write(&service_path, service_content)?;
        
        // Note: Enabling the service would require:
        // systemctl --user daemon-reload
        // systemctl --user enable --now oflama-server.service
        
        Ok(())
    }
}
