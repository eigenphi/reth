//! MEV Pool Discovery ExEx
//!
//! 自动发现区块链上新创建的 DEX Pool，无需人工维护白名单。
//!
//! ## 功能
//!
//! - 监听 UniswapV2/V3 Factory 的 PoolCreated 事件
//! - 解析 Pool 元数据（token0, token1, fee 等）
//! - 去重和验证（确保 Pool 唯一性和有效性）
//! - 维护 Pool Registry（持久化 Pool 列表，支持查询）
//! - 支持初始化时导入历史 Pool 数据
//!
//! ## 示例
//!
//! ```no_run
//! use reth_exex_mev_pool_discovery::{Config, PoolDiscoveryExEx};
//!
//! # async fn example() -> eyre::Result<()> {
//! // 创建配置
//! let config = Config::default();
//!
//! // 创建 ExEx
//! let exex = PoolDiscoveryExEx::new(config)?;
//!
//! // 运行（需要 ExExContext）
//! // exex.run(ctx).await?;
//! # Ok(())
//! # }
//! ```

#![warn(missing_docs)]

pub mod config;
pub mod exex;
pub mod monitors;
pub mod registry;
pub mod storage;

pub use config::Config;
pub use exex::PoolDiscoveryExEx;
pub use registry::{PoolMetadata, PoolRegistry, ProtocolType, RegistryStats};
