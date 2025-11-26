use anyhow::{Context, Result};
use clap::Subcommand;
use colored::Colorize;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;
use url::Url;

use crate::storage::repo::Repository;

//////////////////////////////////////

#[derive(Subcommand, Debug)]
pub enum ConfigCommands {
    Show,
    SetUsername { username: String },
    SetEmail { email: String },
    SetUrl { url: String },
    SetApiKey { api_key: String },
}

#[derive(Serialize, Deserialize, Debug, Default)]
pub struct Config {
    user: UserConfig,
    server: Option<ServerConfig>,
    remotes: Vec<Repository>,
}

#[derive(Serialize, Deserialize, Debug, Default)]
pub struct UserConfig {
    username: String,
    email: String,
}

#[derive(Serialize, Deserialize, Debug, Default)]
pub struct ServerConfig {
    url: String,
    api_key: Option<String>,
}

pub trait PersistentConfig: Serialize + for<'de> Deserialize<'de> + Default {
    fn read_from_file(path: &Path) -> Result<Self> {
        if !path.exists() {
            println!("{}", "Config file not found, using default config".yellow());
            return Ok(Self::default());
        }

        let data = fs::read_to_string(path)
            .with_context(|| format!("Failed to read config file: {}", path.display()))?;
        let config: Self = toml::from_str(&data)
            .with_context(|| format!("Failed to parse config file: {}", path.display()))?;
        Ok(config)
    }

    fn write_to_file(&self, path: &Path) -> Result<()> {
        let data =
            toml::to_string(self).with_context(|| format!("Failed to serialize config to TOML"))?;
        fs::write(path, data)
            .with_context(|| format!("Failed to write config file: {}", path.display()))?;
        println!(
            "{}",
            format!("Config file saved to : {}", path.display()).blue()
        );
        Ok(())
    }
}

impl PersistentConfig for Config {}

impl Config {
    pub fn set_username(&mut self, username: impl Into<String>) {
        self.user.username = username.into();
    }

    pub fn set_email(&mut self, email: impl Into<String>) {
        self.user.email = email.into();
    }

    pub fn set_url(&mut self, url: impl Into<String>) {
        if self.server.is_none() {
            self.server = Some(ServerConfig::default());
        }
        if let Some(server) = &mut self.server {
            server.url = url.into();
        }
    }

    pub fn set_api_key(&mut self, api_key: impl Into<Option<String>>) {
        if self.server.is_none() {
            self.server = Some(ServerConfig::default());
        }
        if let Some(server) = &mut self.server {
            server.api_key = api_key.into();
        }
    }

    pub fn username(&self) -> &str {
        &self.user.username
    }

    pub fn email(&self) -> &str {
        &self.user.email
    }

    pub fn url(&self) -> Option<&str> {
        self.server.as_ref().map(|server| server.url.as_str())
    }

    pub fn api_key(&self) -> Option<&String> {
        self.server
            .as_ref()
            .and_then(|server| server.api_key.as_ref())
    }

    pub fn remotes(&self) -> &[Repository] {
        &self.remotes
    }

    pub fn add_remote(&mut self, name: &str, url: &str, workdir: &Path) -> Result<()> {
        let url_obj = Url::parse(url).with_context(|| format!("Invalid URL format: {}", url))?;

        if self
            .remotes
            .iter()
            .any(|r| r.name == name || r.url() == Some(&url_obj))
        {
            return Err(anyhow::anyhow!(
                "Remote with name '{}' or URL '{}' already exists",
                name,
                url
            ));
        }

        self.remotes
            .push(Repository::new_remote(name, workdir, url_obj));
        Ok(())
    }

    pub fn remove_remote(&mut self, name: &str) -> Result<()> {
        let init_len = self.remotes.len();
        self.remotes.retain(|remote| remote.name != name);

        if self.remotes.len() == init_len {
            return Err(anyhow::anyhow!("Remote '{}' doesn't exist", name));
        }

        Ok(())
    }

    pub fn rename_remote(&mut self, old_name: &str, new_name: &str) -> Result<()> {
        if self.remotes.iter().any(|remote| remote.name == new_name) {
            return Err(anyhow::anyhow!(
                "Remote with name '{}' already exists",
                new_name
            ));
        }

        if let Some(remote) = self
            .remotes
            .iter_mut()
            .find(|remote| remote.name == old_name)
        {
            remote.name = new_name.to_string();
        } else {
            return Err(anyhow::anyhow!("Remote '{}' doesn't exist", old_name));
        }

        Ok(())
    }

    pub fn get_remote(&self, name: &str) -> Result<&Repository> {
        self.remotes
            .iter()
            .find(|remote| remote.name == name)
            .ok_or_else(|| anyhow::anyhow!("Remote '{}' doesn't exist", name))
    }
}
