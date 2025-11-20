use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::env;
use std::fs;
use std::path::Path;
use colored::Colorize;

use crate::error::{Result, SageError};

const CONFIG_FILE: &str = ".sage-config.json";

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct ProviderConfig {
    pub api_key: String,
    pub model: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Config {
    pub active_provider: String,
    pub providers: HashMap<String, ProviderConfig>,
    pub max_tokens: Option<usize>,
    pub default_style: Option<String>,
    #[serde(default)]
    pub preferences: Preferences,
}

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct Preferences {
    pub auto_push: Option<bool>,
    pub auto_stage_all: Option<bool>,
    pub show_diff: Option<bool>,
    pub skip_confirmation: Option<bool>,
    pub verbose: Option<bool>,
}

impl Default for Config {
    fn default() -> Self {
        let mut providers = HashMap::new();
        providers.insert("openai".to_string(), ProviderConfig::default());

        Config {
            active_provider: "openai".to_string(),
            providers,
            max_tokens: Some(300),
            default_style: None,
            preferences: Preferences::default(),
        }
    }
}

impl Config {
    pub fn get_active_provider_config(&self) -> Result<(&String, &ProviderConfig)> {
        let provider = &self.active_provider;
        let config = self.providers.get(provider)
            .ok_or_else(|| SageError::ConfigProviderNotFound {
                provider: provider.clone()
            })?;

        if config.api_key.is_empty() {
            return Err(SageError::ConfigApiKeyNotSet {
                provider: provider.clone()
            });
        }

        Ok((provider, config))
    }

    pub fn set_provider(&mut self, provider: &str, api_key: Option<String>, model: Option<String>) -> Result<()> {
        let config = self.providers.entry(provider.to_string())
            .or_insert_with(ProviderConfig::default);

        if let Some(key) = api_key {
            config.api_key = key;
        }

        if let Some(m) = model {
            config.model = Some(m);
        }

        self.active_provider = provider.to_string();
        Ok(())
    }

    pub fn update_key(&mut self, provider: &str, api_key: &str) -> Result<()> {
        let config = self.providers.entry(provider.to_string())
            .or_insert_with(ProviderConfig::default);

        config.api_key = api_key.to_string();
        Ok(())
    }

    pub fn set_max_tokens(&mut self, tokens: usize) -> Result<()> {
        self.max_tokens = Some(tokens);
        Ok(())
    }

    pub fn set_preference(&mut self, key: &str, value: bool) -> Result<()> {
        match key {
            "auto_push" => self.preferences.auto_push = Some(value),
            "auto_stage_all" => self.preferences.auto_stage_all = Some(value),
            "show_diff" => self.preferences.show_diff = Some(value),
            "skip_confirmation" => self.preferences.skip_confirmation = Some(value),
            "verbose" => self.preferences.verbose = Some(value),
            _ => return Err(SageError::InvalidInput(format!("Unknown preference: {}", key))),
        }
        Ok(())
    }

    pub fn set_default_style(&mut self, style: &str) -> Result<()> {
        match style {
            "standard" | "conventional" | "detailed" | "short" => {
                let normalized = if style == "conventional" { "standard" } else { style };
                self.default_style = Some(normalized.to_string());
                Ok(())
            }
            _ => Err(SageError::InvalidInput(
                format!("Invalid style: {}. Use: conventional, detailed, or short", style)
            )),
        }
    }

    pub fn show(&self) {
        println!("Current configuration:");
        println!("  Active provider: {}", self.active_provider);

        for (provider, provider_config) in &self.providers {
            println!("\n  Provider: {}{}",
                     provider,
                     if provider == &self.active_provider { " (active)" } else { "" }
            );
            println!("    API Key: {}",
                     if provider_config.api_key.is_empty() {
                         "Not set".red().to_string()
                     } else {
                         "Set (hidden)".green().to_string()
                     }
            );
            if let Some(model) = &provider_config.model {
                println!("    Model: {}", model);
            } else {
                println!("    Model: Default");
            }
        }

        if let Some(style) = &self.default_style {
            println!("\n  Default commit style: {}", style);
        }

        println!("  Max tokens: {}", self.max_tokens.unwrap_or(300));

        println!("\nPreferences:");
        println!("  Auto push: {}", format_bool_pref(self.preferences.auto_push));
        println!("  Auto stage all: {}", format_bool_pref(self.preferences.auto_stage_all));
        println!("  Show diff: {}", format_bool_pref(self.preferences.show_diff));
        println!("  Skip confirmation: {}", format_bool_pref(self.preferences.skip_confirmation));
        println!("  Verbose: {}", format_bool_pref(self.preferences.verbose));
    }
}

fn format_bool_pref(value: Option<bool>) -> String {
    match value {
        Some(true) => "enabled".green().to_string(),
        Some(false) => "disabled".red().to_string(),
        None => "not set".yellow().to_string(),
    }
}

pub fn get_config_path() -> Result<String> {
    let home_dir = env::var("HOME").map_err(|_| SageError::ConfigHomeDirNotFound)?;
    Ok(Path::new(&home_dir).join(CONFIG_FILE).to_string_lossy().to_string())
}

pub fn load_config(config_path: &str) -> Result<Config> {
    let path = Path::new(config_path);
    if !path.exists() {
        return Ok(Config::default());
    }

    let config_str = fs::read_to_string(path)?;
    let config: Config = serde_json::from_str(&config_str)?;

    Ok(config)
}

pub fn save_config(config: &Config, config_path: &str) -> Result<()> {
    let config_json = serde_json::to_string_pretty(config)?;
    fs::write(config_path, config_json)?;
    Ok(())
}
