//! Factory Monitor trait 定义
//!
//! 用于监听不同 DEX Factory 的 PoolCreated 事件

use crate::registry::PoolMetadata;
use alloy_primitives::{Address, Log};

/// Factory Monitor trait
///
/// 定义通用接口，支持扩展不同的 DEX 协议
pub trait FactoryMonitor: Send + Sync {
    /// 返回 Factory 合约地址
    fn factory_address(&self) -> Address;

    /// 解析 PoolCreated 事件，提取 Pool 元数据
    ///
    /// 如果日志不是 PoolCreated 事件，返回 None
    fn parse_pool_created_event(
        &self,
        log: &Log,
        block_number: u64,
        tx_index: u32,
        discovered_at: u64,
    ) -> Option<PoolMetadata>;

    /// 返回 PoolCreated 事件的签名（用于快速过滤）
    fn pool_created_event_signature(&self) -> [u8; 32];
}
