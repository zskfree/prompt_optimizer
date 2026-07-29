use serde::{Deserialize, Deserializer, Serialize};
use std::fmt::{Display, Formatter};
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

const MAX_API_PROFILES: usize = 20;
pub const DEFAULT_API_PROFILE_NAME: &str = "默认配置";

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct ApiProfile {
    pub name: String,
    pub api_key: String,
    pub base_url: String,
    pub model: String,
    pub temperature: f64,
    pub max_tokens: u32,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct Config {
    pub active_profile: String,
    pub api_profiles: Vec<ApiProfile>,
    pub hotkey: String,
    pub system_prompt: String,
    pub result_mode: String,
    pub play_sound: bool,
    pub auto_start: bool,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            active_profile: DEFAULT_API_PROFILE_NAME.into(),
            api_profiles: vec![ApiProfile::default()],
            hotkey: "Ctrl+TripleA".into(),
            system_prompt: "你是提示词优化助手。请在不改变原意、不虚构需求的前提下，对用户的原始提示词做轻量优化：表达清楚、结构规范，删除重复、空泛和不必要的内容。使用简洁、规范的 Markdown 格式；只有确有必要时才使用标题或列表。不要扩写成完整方案，不要擅自补充大量背景、角色设定、步骤、示例或验收项。输出长度原则上不超过原文的 1.5 倍；原文较短时最多 200 个汉字。只返回优化后的提示词，不要添加解释、前后缀或 Markdown 代码围栏。".into(),
            result_mode: "clipboard".into(),
            play_sound: true,
            auto_start: false,
        }
    }
}

impl Default for ApiProfile {
    fn default() -> Self {
        Self {
            name: DEFAULT_API_PROFILE_NAME.into(),
            api_key: String::new(),
            base_url: "https://api.openai.com/v1".into(),
            model: "gpt-4o-mini".into(),
            temperature: 0.3,
            max_tokens: 512,
        }
    }
}

#[derive(Deserialize)]
#[serde(default)]
struct ConfigFile {
    active_profile: Option<String>,
    api_profiles: Vec<ApiProfile>,
    hotkey: String,
    system_prompt: String,
    result_mode: String,
    play_sound: bool,
    auto_start: bool,
    // v1.1 and earlier stored the active API values here as a second copy.
    api_key: Option<String>,
    base_url: Option<String>,
    model: Option<String>,
    temperature: Option<f64>,
    max_tokens: Option<u32>,
}

impl Default for ConfigFile {
    fn default() -> Self {
        let config = Config::default();
        Self {
            active_profile: None,
            api_profiles: Vec::new(),
            hotkey: config.hotkey,
            system_prompt: config.system_prompt,
            result_mode: config.result_mode,
            play_sound: config.play_sound,
            auto_start: config.auto_start,
            api_key: None,
            base_url: None,
            model: None,
            temperature: None,
            max_tokens: None,
        }
    }
}

impl<'de> Deserialize<'de> for Config {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let file = ConfigFile::deserialize(deserializer)?;
        Ok(Config::from_file(file))
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
    fn from_file(file: ConfigFile) -> Self {
        let mut profiles = file.api_profiles;
        let legacy_present = file.api_key.is_some()
            || file.base_url.is_some()
            || file.model.is_some()
            || file.temperature.is_some()
            || file.max_tokens.is_some();

        if profiles.is_empty() {
            profiles.push(ApiProfile::default());
        }

        let requested_name = file
            .active_profile
            .as_deref()
            .map(str::trim)
            .filter(|name| !name.is_empty());
        let active_index = requested_name
            .and_then(|name| {
                profiles
                    .iter()
                    .position(|profile| profile.name.trim().eq_ignore_ascii_case(name))
            })
            .unwrap_or(0);

        if legacy_present {
            let active = &mut profiles[active_index];
            if let Some(value) = file.api_key {
                active.api_key = value;
            }
            if let Some(value) = file.base_url {
                active.base_url = value;
            }
            if let Some(value) = file.model {
                active.model = value;
            }
            if let Some(value) = file.temperature {
                active.temperature = value;
            }
            if let Some(value) = file.max_tokens {
                active.max_tokens = value;
            }
        }

        let active_profile = profiles[active_index].name.trim().to_string();
        Self {
            active_profile,
            api_profiles: profiles,
            hotkey: file.hotkey,
            system_prompt: file.system_prompt,
            result_mode: file.result_mode,
            play_sound: file.play_sound,
            auto_start: file.auto_start,
        }
    }

    pub fn validate(&self) -> Result<(), ConfigError> {
        if self.result_mode != "clipboard" {
            return Err(ConfigError::Invalid("result_mode 仅支持 clipboard".into()));
        }
        if self.api_profiles.is_empty() {
            return Err(ConfigError::Invalid("至少需要一个 API 配置".into()));
        }
        if self.api_profiles.len() > MAX_API_PROFILES {
            return Err(ConfigError::Invalid(format!(
                "API 配置最多保存 {MAX_API_PROFILES} 个"
            )));
        }
        let mut names = std::collections::HashSet::new();
        for profile in &self.api_profiles {
            profile.validate()?;
            let normalized = profile.name.trim().to_lowercase();
            if !names.insert(normalized) {
                return Err(ConfigError::Invalid(format!(
                    "API 配置名称重复：{}",
                    profile.name.trim()
                )));
            }
        }
        if !self.api_profiles.iter().any(|profile| {
            profile
                .name
                .trim()
                .eq_ignore_ascii_case(self.active_profile.trim())
        }) {
            return Err(ConfigError::Invalid(format!(
                "当前 API 配置不存在：{}",
                self.active_profile.trim()
            )));
        }
        Ok(())
    }

    pub fn active_api(&self) -> Option<&ApiProfile> {
        self.api_profiles.iter().find(|profile| {
            profile
                .name
                .trim()
                .eq_ignore_ascii_case(self.active_profile.trim())
        })
    }

    pub fn active_api_mut(&mut self) -> Option<&mut ApiProfile> {
        self.api_profiles.iter_mut().find(|profile| {
            profile
                .name
                .trim()
                .eq_ignore_ascii_case(self.active_profile.trim())
        })
    }

    pub fn endpoint(&self) -> Option<String> {
        self.active_api().map(ApiProfile::endpoint)
    }
}

impl ApiProfile {
    pub fn endpoint(&self) -> String {
        format!("{}/chat/completions", self.base_url.trim_end_matches('/'))
    }

    pub fn validate(&self) -> Result<(), ConfigError> {
        let name = self.name.trim();
        if name.is_empty() {
            return Err(ConfigError::Invalid("API 配置名称不能为空".into()));
        }
        if name.chars().count() > 40 || name.chars().any(char::is_control) {
            return Err(ConfigError::Invalid(
                "API 配置名称不能超过 40 个字符或包含控制字符".into(),
            ));
        }
        validate_api_fields(
            &self.base_url,
            &self.model,
            self.temperature,
            self.max_tokens,
        )
    }
}

fn validate_api_fields(
    base_url: &str,
    model: &str,
    temperature: f64,
    max_tokens: u32,
) -> Result<(), ConfigError> {
    if !(base_url.starts_with("http://") || base_url.starts_with("https://")) {
        return Err(ConfigError::Invalid(
            "base_url 必须以 http:// 或 https:// 开头".into(),
        ));
    }
    if base_url.trim_end_matches('/').len() <= "https:".len() {
        return Err(ConfigError::Invalid("base_url 缺少主机地址".into()));
    }
    if model.trim().is_empty() {
        return Err(ConfigError::Invalid("model 不能为空".into()));
    }
    if !temperature.is_finite() || !(0.0..=2.0).contains(&temperature) {
        return Err(ConfigError::Invalid(
            "temperature 必须位于 0.0 到 2.0 之间".into(),
        ));
    }
    if max_tokens == 0 {
        return Err(ConfigError::Invalid("max_tokens 必须大于 0".into()));
    }
    Ok(())
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

pub fn save(path: &Path, config: &Config) -> Result<(), ConfigError> {
    config.validate()?;
    write_atomic(path, config)
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
        assert_eq!(config.active_api().unwrap().model, "custom");
        assert_eq!(config.hotkey, "Ctrl+TripleA");
        assert_eq!(config.active_profile, DEFAULT_API_PROFILE_NAME);
        assert_eq!(config.api_profiles.len(), 1);
    }

    #[test]
    fn validates_boundaries() {
        let mut config = Config::default();
        config.active_api_mut().unwrap().temperature = 2.1;
        assert!(config.validate().is_err());
        config.active_api_mut().unwrap().temperature = 1.0;
        config.active_api_mut().unwrap().max_tokens = 0;
        assert!(config.validate().is_err());
        config.active_api_mut().unwrap().max_tokens = 1;
        config.result_mode = "popup".into();
        assert!(config.validate().is_err());
    }

    #[test]
    fn endpoint_handles_trailing_slash() {
        let mut config = Config::default();
        config.active_api_mut().unwrap().base_url = "http://localhost:1234/v1/".into();
        assert_eq!(
            config.endpoint().as_deref(),
            Some("http://localhost:1234/v1/chat/completions")
        );
    }

    #[test]
    fn supports_clipboard_result_mode() {
        let config = Config::default();
        assert!(config.validate().is_ok());
    }

    #[test]
    fn api_profiles_round_trip_without_duplicating_active_api_fields() {
        let mut config = Config::default();
        let siliconflow = ApiProfile {
            name: "硅基流动".into(),
            api_key: "sf-key".into(),
            base_url: "https://api.siliconflow.cn/v1".into(),
            model: "deepseek-ai/DeepSeek-V4-Flash".into(),
            temperature: 0.2,
            max_tokens: 256,
        };
        config.api_profiles.push(siliconflow);
        config.active_profile = "硅基流动".into();

        assert_eq!(config.active_profile, "硅基流动");
        assert_eq!(
            config.active_api().unwrap().base_url,
            "https://api.siliconflow.cn/v1"
        );
        assert_eq!(
            config.active_api().unwrap().model,
            "deepseek-ai/DeepSeek-V4-Flash"
        );
        assert_eq!(config.hotkey, "Ctrl+TripleA");
        assert!(config.validate().is_ok());

        let encoded = serde_json::to_value(&config).unwrap();
        assert!(encoded.get("api_key").is_none());
        assert!(encoded.get("base_url").is_none());
        assert!(encoded.get("model").is_none());
        assert!(encoded.get("temperature").is_none());
        assert!(encoded.get("max_tokens").is_none());
        let encoded = serde_json::to_string(&config).unwrap();
        assert_eq!(serde_json::from_str::<Config>(&encoded).unwrap(), config);
    }

    #[test]
    fn legacy_top_level_api_fields_migrate_into_the_active_profile() {
        let legacy = r#"{
            "api_key": "legacy-key",
            "base_url": "https://api.siliconflow.cn/v1",
            "model": "deepseek-ai/DeepSeek-V4-Flash",
            "temperature": 0.2,
            "max_tokens": 256,
            "active_profile": null,
            "api_profiles": []
        }"#;
        let config: Config = serde_json::from_str(legacy).unwrap();
        let active = config.active_api().unwrap();
        assert_eq!(active.name, DEFAULT_API_PROFILE_NAME);
        assert_eq!(active.api_key, "legacy-key");
        assert_eq!(active.base_url, "https://api.siliconflow.cn/v1");
        assert_eq!(active.model, "deepseek-ai/DeepSeek-V4-Flash");
        assert_eq!(active.temperature, 0.2);
        assert_eq!(active.max_tokens, 256);
        assert!(config.validate().is_ok());
    }

    #[test]
    fn legacy_active_values_override_only_the_selected_profile() {
        let legacy = r#"{
            "api_key": "new-active-key",
            "base_url": "https://active.example/v1",
            "model": "active-model",
            "temperature": 0.4,
            "max_tokens": 128,
            "active_profile": "工作",
            "api_profiles": [
                {"name":"工作","api_key":"old","base_url":"https://old.example/v1","model":"old-model","temperature":0.3,"max_tokens":64},
                {"name":"备用","api_key":"backup","base_url":"https://backup.example/v1","model":"backup-model","temperature":0.5,"max_tokens":256}
            ]
        }"#;
        let config: Config = serde_json::from_str(legacy).unwrap();
        assert_eq!(config.active_api().unwrap().api_key, "new-active-key");
        assert_eq!(config.api_profiles[1].api_key, "backup");
        assert_eq!(config.api_profiles.len(), 2);
    }

    #[test]
    fn saving_a_legacy_file_rewrites_it_without_duplicate_api_fields() {
        let path = temp_path("legacy-migration");
        fs::write(
            &path,
            r#"{
                "api_key":"legacy-key",
                "base_url":"https://api.example/v1",
                "model":"legacy-model",
                "temperature":0.3,
                "max_tokens":512,
                "hotkey":"Ctrl+F8"
            }"#,
        )
        .unwrap();

        let config = load_existing(&path).unwrap();
        save(&path, &config).unwrap();
        let rewritten: serde_json::Value =
            serde_json::from_str(&fs::read_to_string(&path).unwrap()).unwrap();
        assert!(rewritten.get("api_key").is_none());
        assert!(rewritten.get("base_url").is_none());
        assert!(rewritten.get("model").is_none());
        assert_eq!(rewritten["api_profiles"].as_array().unwrap().len(), 1);
        assert_eq!(rewritten["api_profiles"][0]["api_key"], "legacy-key");
        assert_eq!(rewritten["hotkey"], "Ctrl+F8");

        let _ = fs::remove_file(path);
    }

    #[test]
    fn api_profile_names_are_unique_and_active_profile_must_exist() {
        let mut config = Config::default();
        let first = ApiProfile {
            name: "工作".into(),
            ..ApiProfile::default()
        };
        let duplicate = ApiProfile {
            name: " 工作 ".into(),
            ..ApiProfile::default()
        };
        config.api_profiles = vec![first, duplicate];
        config.active_profile = "工作".into();
        assert!(config.validate().is_err());

        config.api_profiles.truncate(1);
        config.active_profile = "不存在".into();
        assert!(config.validate().is_err());
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

    #[test]
    fn saves_valid_config_and_preserves_file_when_validation_fails() {
        let path = temp_path("save");
        let original = Config::default();
        save(&path, &original).unwrap();

        let mut updated = original.clone();
        updated.active_api_mut().unwrap().model = "deepseek-ai/DeepSeek-V4-Flash".into();
        updated.auto_start = true;
        save(&path, &updated).unwrap();
        assert_eq!(load_existing(&path).unwrap(), updated);

        let mut invalid = updated.clone();
        invalid.active_api_mut().unwrap().max_tokens = 0;
        assert!(save(&path, &invalid).is_err());
        assert_eq!(load_existing(&path).unwrap(), updated);

        let _ = fs::remove_file(path);
    }
}
