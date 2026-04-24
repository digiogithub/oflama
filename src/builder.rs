#![allow(dead_code)]
use std::path::PathBuf;
use std::process::Stdio;
use tokio::process::Command;
use anyhow::Result;

#[derive(Clone, PartialEq)]
pub enum Backend {
    Cpu,
    Cuda,
    Sycl,
    Vulkan,
}

#[derive(Clone)]
pub struct BuildOptions {
    pub repo_url: String,
    pub branch: String,
    pub backend: Backend,
    pub use_turboquant: bool,
}

impl Default for BuildOptions {
    fn default() -> Self {
        Self {
            repo_url: "https://github.com/ggml-org/llama.cpp.git".to_string(),
            branch: "master".to_string(),
            backend: Backend::Cuda,
            use_turboquant: false,
        }
    }
}

pub async fn clone_repo(opts: &BuildOptions, target_dir: &PathBuf) -> Result<()> {
    if target_dir.exists() {
        return Ok(()); // Already cloned, could just pull or skip
    }

    let status = Command::new("git")
        .arg("clone")
        .arg("--depth").arg("1")
        .arg(&opts.repo_url)
        .arg(target_dir)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .await?;

    if !status.success() {
        anyhow::bail!("Failed to clone repository");
    }

    if opts.use_turboquant {
        let fetch_status = Command::new("git")
            .current_dir(target_dir)
            .arg("fetch")
            .arg("origin")
            .arg("pull/21089/head:pr-21089")
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .await?;
            
        if !fetch_status.success() {
            anyhow::bail!("Failed to fetch PR 21089");
        }
        
        let checkout_status = Command::new("git")
            .current_dir(target_dir)
            .arg("checkout")
            .arg("pr-21089")
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .await?;
            
        if !checkout_status.success() {
            anyhow::bail!("Failed to checkout PR 21089");
        }
    }

    Ok(())
}

pub async fn build_llama_cpp(opts: &BuildOptions, source_dir: &PathBuf) -> Result<()> {
    let build_dir = source_dir.join("build");
    
    if build_dir.exists() {
        let _ = std::fs::remove_dir_all(&build_dir);
    }

    let mut cmake_args = vec![
        "-B".to_string(),
        build_dir.to_string_lossy().to_string(),
        "-DCMAKE_BUILD_TYPE=Release".to_string(),
    ];

    match opts.backend {
        Backend::Cpu => {}
        Backend::Cuda => {
            cmake_args.push("-DGGML_CUDA=ON".to_string());
        }
        Backend::Sycl => {
            cmake_args.push("-DGGML_SYCL=ON".to_string());
            cmake_args.push("-DCMAKE_C_COMPILER=icx".to_string());
            cmake_args.push("-DCMAKE_CXX_COMPILER=icpx".to_string());
        }
        Backend::Vulkan => {
            cmake_args.push("-DGGML_VULKAN=ON".to_string());
        }
    }

    let conf_status = Command::new("cmake")
        .current_dir(source_dir)
        .args(&cmake_args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .await?;

    if !conf_status.status.success() {
        let stderr = String::from_utf8_lossy(&conf_status.stderr);
        anyhow::bail!("CMake configure failed:\n{}", stderr);
    }

    let build_status = Command::new("cmake")
        .current_dir(source_dir)
        .arg("--build")
        .arg(&build_dir)
        .arg("-j")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .await?;

    if !build_status.status.success() {
        let stderr = String::from_utf8_lossy(&build_status.stderr);
        anyhow::bail!("CMake build failed:\n{}", stderr);
    }

    Ok(())
}
