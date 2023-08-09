use crate::filesystem::{path_exists, pick_free_file_name};
use rocket::async_trait;
use serde::de::DeserializeOwned;
use serde::Serialize;
use std::path::Path;

#[async_trait]
pub trait ConfigTrait: Sized + DeserializeOwned + Serialize + Send + Sync + Clone {
    async fn neutralize_broken_config_file(config_file: &Path) -> anyhow::Result<()> {
        let mut renamed_file = config_file.to_path_buf();
        let file_name = renamed_file
            .file_name()
            .unwrap()
            .to_str()
            .unwrap()
            .to_owned();
        renamed_file.set_file_name(file_name + "_old");
        let renamed_file = pick_free_file_name(&renamed_file).await;
        tokio::fs::rename(config_file, renamed_file).await?;
        Ok(())
    }

    async fn load(config_file: &Path) -> anyhow::Result<Self> {
        let config_contents = tokio::fs::read_to_string(config_file).await?;
        let contents: Self = serde_yaml::from_str(&config_contents)?;
        let copied = contents.clone();
        drop(contents);
        Ok(copied)
    }

    async fn save(&self, config_file: &Path) -> anyhow::Result<()> {
        let data = serde_yaml::to_string(&self)?;
        tokio::fs::create_dir_all(config_file.parent().unwrap()).await?;
        tokio::fs::write(config_file, data).await?;
        Ok(())
    }

    async fn init(config_file: &Path, default: Self) -> Self {
        let config: Option<Self> = if path_exists(config_file).await {
            match Self::load(config_file).await {
                Ok(config) => {
                    if Self::check_validity(&config).await {
                        Some(config)
                    } else {
                        None
                    }
                }
                Err(_) => {
                    let _ = Self::neutralize_broken_config_file(config_file).await;
                    None
                }
            }
        } else {
            None
        };
        if let Some(config) = config {
            config
        } else {
            let config = default;
            config.save(config_file).await.unwrap();
            config
        }
    }

    async fn check_validity(config: &Self) -> bool;
}
