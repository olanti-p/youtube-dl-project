use crate::env::config_trait::ConfigTrait;
use rocket::async_trait;
use serde::{Deserialize, Serialize};
use std::ffi::{OsStr, OsString};

#[derive(Debug, Clone, Serialize, Deserialize)]
struct CommandTemplate {
    pub args: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DownloadFormat {
    pub id: String,
    pub display: String,
    pub ext: String,
    pub args: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct YtdlpConfig {
    command_fetch_url: CommandTemplate,
    command_download: CommandTemplate,
    formats: Vec<DownloadFormat>,
}

impl YtdlpConfig {
    pub(super) fn new() -> Self {
        serde_yaml::from_str(include_str!("ytdlp.yaml")).unwrap()
    }

    fn try_get_format(&self, format: &str) -> Option<&DownloadFormat> {
        self.formats.iter().find(|x| x.id == format)
    }

    pub fn get_format(&self, format: &str) -> &DownloadFormat {
        self.try_get_format(format)
            .expect(&format!("Unknown format: {format}"))
    }

    pub fn get_all_formats(&self) -> &[DownloadFormat] {
        &self.formats
    }

    pub fn render_fetch_url_command<S: AsRef<OsStr>>(&self, source_url: S) -> Vec<OsString> {
        self.command_fetch_url
            .args
            .iter()
            .map(|x| {
                if x == "{{source_url}}" {
                    source_url.as_ref().to_os_string()
                } else {
                    OsString::from(x)
                }
            })
            .collect()
    }

    pub fn render_download_command<S1: AsRef<OsStr>, S2: AsRef<OsStr>>(
        &self,
        source_url: S1,
        format: &DownloadFormat,
        destination_file: S2,
    ) -> Vec<OsString> {
        self.command_download
            .args
            .iter()
            .flat_map(|x| match x.as_str() {
                "{{source_url}}" => vec![OsString::from(source_url.as_ref())],
                "{{format_args}}" => format.args.iter().map(|x| OsString::from(x)).collect(),
                "{{destination_file}}" => vec![OsString::from(destination_file.as_ref())],
                x => vec![OsString::from(x)],
            })
            .collect()
    }
}

#[async_trait]
impl ConfigTrait for YtdlpConfig {
    async fn check_validity(_: &Self) -> bool {
        true
    }
}
