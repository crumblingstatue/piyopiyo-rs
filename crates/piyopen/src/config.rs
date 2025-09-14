use {
    serde::{Deserialize, Serialize},
    std::path::{Path, PathBuf},
};

#[derive(Serialize, Deserialize, Default)]
pub struct Config {
    #[serde(default)]
    pub fallback_fonts: Vec<FallbackFont>,
    #[serde(default)]
    pub last_opened: Option<String>,
}

#[derive(Serialize, Deserialize)]
pub struct FallbackFont {
    pub name: String,
    pub path: String,
}

fn cfg_path() -> PathBuf {
    dirs::config_dir().unwrap().join("piyopen/config.ron")
}

/// Returns None if the config doesn't exist, and should be created
pub fn load() -> Option<anyhow::Result<Config>> {
    let path = cfg_path();
    if path.exists() {
        Some(load_config_file(&path))
    } else {
        None
    }
}

pub fn save(cfg: &Config) -> anyhow::Result<()> {
    let path = cfg_path();
    std::fs::create_dir_all(path.parent().unwrap())?;
    let text = ron::to_string(cfg)?;
    std::fs::write(path, text)?;
    Ok(())
}

fn load_config_file(path: &Path) -> anyhow::Result<Config> {
    let text = std::fs::read_to_string(path)?;
    let cfg: Config = ron::from_str(&text)?;
    Ok(cfg)
}
