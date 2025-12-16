//! MEV Pool State Tracker
//!
//! 从 Reth Storage 层直接读取 DEX Pool 的原始状态数据。
//!
//! ## 功能
//!
//! - 直接从 Storage 读取 Pool 状态（绕过 RPC）
//! - 支持 UniswapV2 Slot 8 解析（reserve0/reserve1/timestamp）
//! - 批量并发读取多个 Pool
//! - 详细的日志输出，便于调试和性能分析
//!
//! ## 使用示例
//!
//! ```rust,no_run
//! use alloy_primitives::Address;
//! use reth_exex_mev_pool_discovery::ProtocolType;
//! use reth_exex_mev_pool_state_tracker::{PoolStateTracker, RawPoolState};
//!
//! # async fn example() -> eyre::Result<()> {
//! // 假设你有一个 StateProviderFactory
//! # use reth_storage_api::noop::NoopProvider;
//! # let provider_factory = NoopProvider::default();
//!
//! let tracker = PoolStateTracker::new(provider_factory);
//!
//! // 读取单个 Pool 状态
//! let pool_address = Address::ZERO; // 实际地址
//! let block_number = 1000000;
//! let state = tracker.track_pool_state(pool_address, block_number, ProtocolType::UniswapV2)?;
//!
//! // 批量读取
//! let pools = vec![
//!     (pool_address, ProtocolType::UniswapV2),
//!     // ... 更多 pools
//! ];
//! let states = tracker.track_multiple_states(pools, block_number).await?;
//! # Ok(())
//! # }
//! ```

#![warn(missing_docs)]

pub mod raw_state;
pub mod tracker;

pub use raw_state::{RawPoolState, UniswapV2State, UniswapV3State};
pub use tracker::PoolStateTracker;

// Re-export commonly used types
pub use reth_exex_mev_pool_discovery::ProtocolType;
