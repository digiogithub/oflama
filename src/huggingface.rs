#![allow(dead_code)]
use reqwest::Client;
use serde::Deserialize;
use std::error::Error;
use std::path::PathBuf;

#[derive(Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ModelMeta {
    pub model_id: String,
    pub last_modified: Option<String>,
}

pub struct HuggingFaceClient {
    client: Client,
}

impl HuggingFaceClient {
    pub fn new() -> Self {
        Self {
            client: Client::new(),
        }
    }

    pub async fn search_gguf_models(&self, query: &str) -> Result<Vec<ModelMeta>, Box<dyn Error>> {
        let url = format!(
            "https://huggingface.co/api/models?search={}&filter=gguf&limit=20",
            query
        );
        let res = self.client.get(&url).send().await?;
        let models: Vec<ModelMeta> = res.json().await?;
        Ok(models)
    }

    pub async fn get_model_files(&self, model_id: &str) -> Result<Vec<String>, Box<dyn Error>> {
        let url = format!("https://huggingface.co/api/models/{}/tree/main", model_id);
        let res = self.client.get(&url).send().await?;
        
        #[derive(Deserialize)]
        struct FileEntry {
            path: String,
            r#type: String, // "file" or "directory"
        }
        
        let entries: Vec<FileEntry> = res.json().await?;
        let files = entries
            .into_iter()
            .filter(|e| e.r#type == "file" && e.path.ends_with(".gguf"))
            .map(|e| e.path)
            .collect();
            
        Ok(files)
    }

    pub async fn download_file(
        &self,
        model_id: &str,
        file: &str,
        dest: PathBuf,
    ) -> Result<(), Box<dyn Error>> {
        use tokio::io::AsyncWriteExt;
        
        let url = format!("https://huggingface.co/{}/resolve/main/{}", model_id, file);
        let mut response = self.client.get(&url).send().await?;
        
        let mut out = tokio::fs::File::create(dest).await?;
        
        while let Some(chunk) = response.chunk().await? {
            out.write_all(&chunk).await?;
        }
        
        Ok(())
    }
}
