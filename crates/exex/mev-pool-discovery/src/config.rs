//! 配置管理模块

use alloy_primitives::Address;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Pool Discovery Service 配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    /// 是否启用持久化存储
    pub enable_persistence: bool,

    /// 持久化存储路径
    pub storage_path: PathBuf,

    /// 要监听的 Factory 配置
    pub factories: Vec<FactoryConfig>,
}

/// Factory 配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FactoryConfig {
    /// Factory 名称
    pub name: String,

    /// Factory 合约地址
    pub address: Address,

    /// 协议类型
    pub protocol: String, // "uniswap_v2" 或 "uniswap_v3"
}

impl Default for Config {
    fn default() -> Self {
        Self {
            enable_persistence: true,
            storage_path: PathBuf::from("./pool_discovery_data"),
            factories: vec![
                // UniswapV2 主网 Factory
                FactoryConfig {
                    name: "UniswapV2".to_string(),
                    address: Address::new([
                        0x5C, 0x69, 0xbE, 0xe7, 0x01, 0xef, 0x81, 0x4a, 0x2B, 0x6a, 0x3E, 0xDD,
                        0x4B, 0x16, 0x52, 0xCB, 0x9c, 0xc5, 0xaA, 0x6f,
                    ]),
                    protocol: "uniswap_v2".to_string(),
                },
                // UniswapV3 主网 Factory
                FactoryConfig {
                    name: "UniswapV3".to_string(),
                    address: Address::new([
                        0x1F, 0x98, 0x43, 0x1c, 0x8a, 0xD9, 0x85, 0x23, 0x63, 0x1A, 0xE4, 0xa5,
                        0x9f, 0x26, 0x73, 0x46, 0xea, 0x31, 0xF9, 0x84,
                    ]),
                    protocol: "uniswap_v3".to_string(),
                },
            ],
        }
    }
}

impl Config {
    /// 从 JSON 文件加载配置
    pub fn from_file(path: impl AsRef<std::path::Path>) -> eyre::Result<Self> {
        let content = std::fs::read_to_string(path)?;
        let config: Config = serde_json::from_str(&content)?;
        Ok(config)
    }

    /// 保存配置到 JSON 文件
    pub fn save_to_file(&self, path: impl AsRef<std::path::Path>) -> eyre::Result<()> {
        let content = serde_json::to_string_pretty(self)?;
        std::fs::write(path, content)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::NamedTempFile;

    #[test]
    fn test_default_config() {
        let config = Config::default();
        assert!(config.enable_persistence);
        assert_eq!(config.factories.len(), 2);
    }

    #[test]
    fn test_config_serialization() {
        let config = Config::default();
        let json = serde_json::to_string(&config).unwrap();
        let loaded: Config = serde_json::from_str(&json).unwrap();
        assert_eq!(loaded.factories.len(), config.factories.len());
    }

    #[test]
    fn test_config_file_io() {
        let config = Config::default();
        let temp_file = NamedTempFile::new().unwrap();

        config.save_to_file(temp_file.path()).unwrap();
        let loaded = Config::from_file(temp_file.path()).unwrap();

        assert_eq!(loaded.factories.len(), config.factories.len());
    }
}
