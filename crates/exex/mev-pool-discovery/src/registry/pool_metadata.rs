//! Pool 元数据定义

use alloy_primitives::Address;
use serde::{Deserialize, Serialize};

/// DEX 协议类型
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ProtocolType {
    /// UniswapV2 及兼容协议
    UniswapV2,
    /// UniswapV3
    UniswapV3,
}

/// Pool 元数据
///
/// 包含 Pool 的基本信息，用于标识和查询
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PoolMetadata {
    /// Pool 合约地址
    pub pool_address: Address,
    /// DEX 协议类型
    pub protocol_id: ProtocolType,
    /// 创建此 Pool 的 Factory 地址
    pub factory_address: Address,
    /// Token0 地址（按地址排序）
    pub token0: Address,
    /// Token1 地址
    pub token1: Address,
    /// 手续费率（basis points 或 V3 的 fee tier）
    pub fee: u32,
    /// Tick 间隔（仅 V3 使用）
    pub tick_spacing: Option<i32>,
    /// Pool 创建的区块号
    pub created_block: u64,
    /// Pool 创建的交易索引
    pub created_tx_index: u32,
    /// 发现时间戳（Unix timestamp）
    pub discovered_at: u64,
}

impl PoolMetadata {
    /// 验证 Pool 元数据的有效性
    ///
    /// 检查：
    /// - 地址非零
    /// - token0 < token1（按地址字典序）
    pub fn validate(&self) -> Result<(), String> {
        // 检查地址非零
        if self.pool_address == Address::ZERO {
            return Err("Pool address cannot be zero".to_string());
        }
        if self.token0 == Address::ZERO {
            return Err("Token0 address cannot be zero".to_string());
        }
        if self.token1 == Address::ZERO {
            return Err("Token1 address cannot be zero".to_string());
        }
        if self.factory_address == Address::ZERO {
            return Err("Factory address cannot be zero".to_string());
        }

        // 检查 token0 < token1
        if self.token0 >= self.token1 {
            return Err(format!(
                "Token0 ({}) must be less than token1 ({})",
                self.token0, self.token1
            ));
        }

        // V3 必须有 tick_spacing
        if self.protocol_id == ProtocolType::UniswapV3 && self.tick_spacing.is_none() {
            return Err("UniswapV3 pool must have tick_spacing".to_string());
        }

        Ok(())
    }

    /// 创建 UniswapV2 Pool 元数据
    #[allow(clippy::too_many_arguments)]
    pub fn new_v2(
        pool_address: Address,
        factory_address: Address,
        token0: Address,
        token1: Address,
        created_block: u64,
        created_tx_index: u32,
        discovered_at: u64,
    ) -> Self {
        Self {
            pool_address,
            protocol_id: ProtocolType::UniswapV2,
            factory_address,
            token0,
            token1,
            fee: 30, // V2 固定 0.3% 手续费
            tick_spacing: None,
            created_block,
            created_tx_index,
            discovered_at,
        }
    }

    /// 创建 UniswapV3 Pool 元数据
    #[allow(clippy::too_many_arguments)]
    pub fn new_v3(
        pool_address: Address,
        factory_address: Address,
        token0: Address,
        token1: Address,
        fee: u32,
        tick_spacing: i32,
        created_block: u64,
        created_tx_index: u32,
        discovered_at: u64,
    ) -> Self {
        Self {
            pool_address,
            protocol_id: ProtocolType::UniswapV3,
            factory_address,
            token0,
            token1,
            fee,
            tick_spacing: Some(tick_spacing),
            created_block,
            created_tx_index,
            discovered_at,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mock_address(n: u8) -> Address {
        Address::from([n; 20])
    }

    #[test]
    fn test_pool_metadata_validation() {
        let metadata = PoolMetadata::new_v2(
            mock_address(1),
            mock_address(2),
            mock_address(3),
            mock_address(4),
            1000,
            0,
            1638360000,
        );

        assert!(metadata.validate().is_ok());
    }

    #[test]
    fn test_pool_metadata_zero_address() {
        let metadata = PoolMetadata::new_v2(
            Address::ZERO,
            mock_address(2),
            mock_address(3),
            mock_address(4),
            1000,
            0,
            1638360000,
        );

        assert!(metadata.validate().is_err());
    }

    #[test]
    fn test_pool_metadata_token_order() {
        // token0 > token1，应该验证失败
        let metadata = PoolMetadata::new_v2(
            mock_address(1),
            mock_address(2),
            mock_address(5), // 大于 token1
            mock_address(4),
            1000,
            0,
            1638360000,
        );

        assert!(metadata.validate().is_err());
    }

    #[test]
    fn test_v3_requires_tick_spacing() {
        let mut metadata = PoolMetadata::new_v2(
            mock_address(1),
            mock_address(2),
            mock_address(3),
            mock_address(4),
            1000,
            0,
            1638360000,
        );
        metadata.protocol_id = ProtocolType::UniswapV3;
        metadata.tick_spacing = None;

        assert!(metadata.validate().is_err());
    }
}
