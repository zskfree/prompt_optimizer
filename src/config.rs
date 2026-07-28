use serde::{Deserialize, Serialize};
use std::fmt::{Display, Formatter};
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(default)]
pub struct Config {
    pub api_key: String,
    pub base_url: String,
    pub model: String,
    pub hotkey: String,
    pub temperature: f64,
    pub max_tokens: u32,
    pub system_prompt: String,
    pub result_mode: String,
    pub play_sound: bool,
    pub auto_start: bool,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            api_key: String::new(),
            base_url: "https://api.openai.com/v1".into(),
            model: "gpt-4o-mini".into(),
            hotkey: "Ctrl+TripleA".into(),
            temperature: 0.3,
            max_tokens: 512,
            system_prompt: "你是提示词优化助手。请在不改变原意、不虚构需求的前提下，对用户的原始提示词做轻量优化：表达清楚、结构规范，删除重复、空泛和不必要的内容。使用简洁、规范的 Markdown 格式；只有确有必要时才使用标题或列表。不要扩写成完整方案，不要擅自补充大量背景、角色设定、步骤、示例或验收项。输出长度原则上不超过原文的 1.5 倍；原文较短时最多 200 个汉字。只返回优化后的提示词，不要添加解释、前后缀或 Markdown 代码围栏。".into(),
            result_mode: "clipboard".into(),
            play_sound: true,
            auto_start: false,
        }
    }
}

#[derive(Debug)]
pub enum ConfigError {
    Io(io::Error),
    Invalid(String),
    InvalidJson {
        source: serde_json::Error,
        backup: PathBuf,
    },
}

impl Display for ConfigError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io(error) => write!(formatter, "配置文件读写失败：{error}"),
            Self::Invalid(message) => write!(formatter, "配置无效：{message}"),
            Self::InvalidJson { source, backup } => write!(
                formatter,
                "配置文件格式损坏（已备份到 {}）：{source}",
                backup.display()
            ),
        }
    }
}

impl std::error::Error for ConfigError {}

impl From<io::Error> for ConfigError {
    fn from(value: io::Error) -> Self {
        Self::Io(value)
    }
}

impl Config {
    pub fn validate(&self) -> Result<(), ConfigError> {
        if !(self.base_url.starts_with("http://") || self.base_url.starts_with("https://")) {
            return Err(ConfigError::Invalid(
                "base_url 必须以 http:// 或 https:// 开头".into(),
            ));
        }
        if self.base_url.trim_end_matches('/').len() <= "https:".len() {
            return Err(ConfigError::Invalid("base_url 缺少主机地址".into()));
        }
        if self.model.trim().is_empty() {
            return Err(ConfigError::Invalid("model 不能为空".into()));
        }
        if !self.temperature.is_finite() || !(0.0..=2.0).contains(&self.temperature) {
            return Err(ConfigError::Invalid(
                "temperature 必须位于 0.0 到 2.0 之间".into(),
            ));
        }
        if self.max_tokens == 0 {
            return Err(ConfigError::Invalid("max_tokens 必须大于 0".into()));
        }
        if self.result_mode != "clipboard" {
            return Err(ConfigError::Invalid("result_mode 仅支持 clipboard".into()));
        }
        Ok(())
    }

    pub fn endpoint(&self) -> String {
        format!("{}/chat/completions", self.base_url.trim_end_matches('/'))
    }
}

pub fn load_or_create(path: &Path) -> Result<(Config, bool), ConfigError> {
    if !path.exists() {
        let config = Config::default();
        write_atomic(path, &config)?;
        return Ok((config, true));
    }

    let contents = fs::read_to_string(path)?;
    match serde_json::from_str::<Config>(&contents) {
        Ok(config) => {
            config.validate()?;
            Ok((config, false))
        }
        Err(source) => {
            let backup = invalid_backup_path(path);
            fs::rename(path, &backup)?;
            write_atomic(path, &Config::default())?;
            Err(ConfigError::InvalidJson { source, backup })
        }
    }
}

pub fn load_existing(path: &Path) -> Result<Config, ConfigError> {
    let contents = fs::read_to_string(path)?;
    let config = serde_json::from_str::<Config>(&contents)
        .map_err(|error| ConfigError::Invalid(format!("JSON 格式错误：{error}")))?;
    config.validate()?;
    Ok(config)
}

fn write_atomic(path: &Path, config: &Config) -> Result<(), ConfigError> {
    let temp_path = path.with_extension("json.tmp");
    let mut contents = serde_json::to_string_pretty(config)
        .map_err(|error| ConfigError::Invalid(format!("无法序列化默认配置：{error}")))?;
    contents.push('\n');
    fs::write(&temp_path, contents.as_bytes())?;
    if path.exists() {
        fs::remove_file(path)?;
    }
    fs::rename(temp_path, path)?;
    Ok(())
}

fn invalid_backup_path(path: &Path) -> PathBuf {
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    path.with_file_name(format!("config.invalid-{timestamp}.json"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    fn temp_path(name: &str) -> PathBuf {
        std::env::temp_dir().join(format!(
            "prompt-optimizer-{name}-{}-{}.json",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ))
    }

    #[test]
    fn creates_and_reads_default_config() {
        let path = temp_path("default");
        let (created, was_created) = load_or_create(&path).unwrap();
        assert!(was_created);
        assert_eq!(created, Config::default());
        let (loaded, was_created) = load_or_create(&path).unwrap();
        assert!(!was_created);
        assert_eq!(loaded, created);
        let _ = fs::remove_file(path);
    }

    #[test]
    fn missing_fields_use_defaults() {
        let config: Config = serde_json::from_str(r#"{"model":"custom"}"#).unwrap();
        assert_eq!(config.model, "custom");
        assert_eq!(config.hotkey, "Ctrl+TripleA");
    }

    #[test]
    fn validates_boundaries() {
        let mut config = Config {
            temperature: 2.1,
            ..Config::default()
        };
        assert!(config.validate().is_err());
        config.temperature = 1.0;
        config.max_tokens = 0;
        assert!(config.validate().is_err());
        config.max_tokens = 1;
        config.result_mode = "popup".into();
        assert!(config.validate().is_err());
    }

    #[test]
    fn endpoint_handles_trailing_slash() {
        let config = Config {
            base_url: "http://localhost:1234/v1/".into(),
            ..Config::default()
        };
        assert_eq!(
            config.endpoint(),
            "http://localhost:1234/v1/chat/completions"
        );
    }

    #[test]
    fn supports_clipboard_result_mode() {
        let config = Config::default();
        assert!(config.validate().is_ok());
    }

    #[test]
    fn damaged_json_is_backed_up_and_replaced() {
        let path = temp_path("damaged");
        fs::write(&path, "{not-json").unwrap();
        let error = load_or_create(&path).unwrap_err();
        let backup = match error {
            ConfigError::InvalidJson { backup, .. } => backup,
            other => panic!("unexpected error: {other}"),
        };
        assert!(backup.exists());
        assert!(path.exists());
        std::thread::sleep(Duration::from_millis(1));
        let _ = fs::remove_file(path);
        let _ = fs::remove_file(backup);
    }
}
