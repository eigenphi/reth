//! Raw Pool State 数据结构定义

use alloy_primitives::{Address, B256, U256};
use reth_exex_mev_pool_discovery::ProtocolType;
use serde::{Deserialize, Serialize};

/// Pool 的原始状态数据（从 Storage 直接读取）
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum RawPoolState {
    /// UniswapV2 Pool 状态
    UniswapV2(UniswapV2State),
    /// UniswapV3 Pool 状态（未实现）
    #[allow(dead_code)]
    UniswapV3(UniswapV3State),
}

/// UniswapV2 Pool 的原始状态
///
/// 从 Slot 8 读取的压缩数据：
/// - reserve0: bits[0..112)
/// - reserve1: bits[112..224)
/// - blockTimestampLast: bits[224..256)
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UniswapV2State {
    /// Pool 地址
    pub pool_address: Address,
    /// 区块号
    pub block_number: u64,
    /// Token0 储备量 (112 bits)
    pub reserve0: U256,
    /// Token1 储备量 (112 bits)
    pub reserve1: U256,
    /// 最后更新时间戳 (32 bits)
    pub block_timestamp_last: u32,
    /// 原始 Slot 8 数据（用于调试）
    pub raw_slot8: B256,
}

impl UniswapV2State {
    /// 从 Slot 8 的原始数据解析 UniswapV2 状态
    ///
    /// Slot 8 布局（256 bits）：
    /// ```text
    /// |  32 bits   |    112 bits    |    112 bits    |
    /// | timestamp  |    reserve1    |    reserve0    |
    /// | [224..256) |   [112..224)   |    [0..112)    |
    /// ```
    pub fn from_slot8(pool_address: Address, block_number: u64, slot8_data: B256) -> Self {
        // 将 B256 转换为 U256 进行位操作
        let slot8 = U256::from_be_bytes(slot8_data.0);

        // 提取 reserve0: 低 112 bits
        // 创建掩码: 2^112 - 1
        let mask_112 = (U256::from(1u128) << 112) - U256::from(1u128);
        let reserve0 = slot8 & mask_112;

        // 提取 reserve1: 中间 112 bits [112..224)
        let reserve1 = (slot8 >> 112) & mask_112;

        // 提取 blockTimestampLast: 高 32 bits [224..256)
        let timestamp_u256: U256 = (slot8 >> 224) & U256::from(u32::MAX);
        let timestamp = timestamp_u256.to::<u32>();

        Self {
            pool_address,
            block_number,
            reserve0,
            reserve1,
            block_timestamp_last: timestamp,
            raw_slot8: slot8_data,
        }
    }

    /// 验证状态数据的有效性
    pub fn validate(&self) -> Result<(), String> {
        // 检查 reserve0 和 reserve1 不超过 112 bits
        let max_112_bits = (U256::from(1u128) << 112) - U256::from(1u128);

        if self.reserve0 > max_112_bits {
            return Err(format!("reserve0 exceeds 112 bits: {}", self.reserve0));
        }

        if self.reserve1 > max_112_bits {
            return Err(format!("reserve1 exceeds 112 bits: {}", self.reserve1));
        }

        // Pool 地址不能为零
        if self.pool_address == Address::ZERO {
            return Err("Pool address cannot be zero".to_string());
        }

        Ok(())
    }
}

/// UniswapV3 Pool 的原始状态（未实现，保留接口）
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UniswapV3State {
    /// Pool 地址
    pub pool_address: Address,
    /// 区块号
    pub block_number: u64,
    /// 当前价格的平方根 (160 bits from Slot 0)
    pub sqrt_price_x96: U256,
    /// 当前 Tick (24 bits from Slot 0)
    pub tick: i32,
    /// 观测点索引 (16 bits from Slot 0)
    pub observation_index: u16,
    /// 当前流动性 (128 bits from Slot 4)
    pub liquidity: u128,
}

impl RawPoolState {
    /// 获取 Pool 地址
    pub fn pool_address(&self) -> Address {
        match self {
            RawPoolState::UniswapV2(state) => state.pool_address,
            RawPoolState::UniswapV3(state) => state.pool_address,
        }
    }

    /// 获取区块号
    pub fn block_number(&self) -> u64 {
        match self {
            RawPoolState::UniswapV2(state) => state.block_number,
            RawPoolState::UniswapV3(state) => state.block_number,
        }
    }

    /// 获取协议类型
    pub fn protocol(&self) -> ProtocolType {
        match self {
            RawPoolState::UniswapV2(_) => ProtocolType::UniswapV2,
            RawPoolState::UniswapV3(_) => ProtocolType::UniswapV3,
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
    fn test_uniswap_v2_slot8_parsing_basic() {
        // 构造一个简单的 slot8 值：
        // reserve0 = 1000 (0x3E8)
        // reserve1 = 2000 (0x7D0)
        // timestamp = 1234567890 (0x499602D2)

        let reserve0 = U256::from(1000u64);
        let reserve1 = U256::from(2000u64);
        let timestamp = U256::from(1234567890u64);

        // 组装 slot8: timestamp << 224 | reserve1 << 112 | reserve0
        let slot8_value: U256 = (timestamp << 224) | (reserve1 << 112) | reserve0;
        let slot8_bytes = B256::from(slot8_value.to_be_bytes::<32>());

        let state = UniswapV2State::from_slot8(mock_address(1), 100, slot8_bytes);

        assert_eq!(state.reserve0, U256::from(1000u64));
        assert_eq!(state.reserve1, U256::from(2000u64));
        assert_eq!(state.block_timestamp_last, 1234567890);
        assert_eq!(state.block_number, 100);
        assert_eq!(state.pool_address, mock_address(1));
    }

    #[test]
    fn test_uniswap_v2_slot8_parsing_max_values() {
        // 测试最大值边界：
        // reserve0 = 2^112 - 1 (最大 112 bits 值)
        // reserve1 = 2^112 - 1
        // timestamp = 2^32 - 1 (最大 32 bits 值)

        let max_112_bits = (U256::from(1u128) << 112) - U256::from(1u128);
        let max_32_bits = U256::from(u32::MAX);

        let slot8_value: U256 = (max_32_bits << 224) | (max_112_bits << 112) | max_112_bits;
        let slot8_bytes = B256::from(slot8_value.to_be_bytes::<32>());

        let state = UniswapV2State::from_slot8(mock_address(2), 200, slot8_bytes);

        assert_eq!(state.reserve0, max_112_bits);
        assert_eq!(state.reserve1, max_112_bits);
        assert_eq!(state.block_timestamp_last, u32::MAX);
        assert!(state.validate().is_ok());
    }

    #[test]
    fn test_uniswap_v2_slot8_parsing_zero_values() {
        // 测试零值
        let slot8_bytes = B256::ZERO;

        let state = UniswapV2State::from_slot8(mock_address(3), 300, slot8_bytes);

        assert_eq!(state.reserve0, U256::ZERO);
        assert_eq!(state.reserve1, U256::ZERO);
        assert_eq!(state.block_timestamp_last, 0);
    }

    #[test]
    fn test_uniswap_v2_slot8_parsing_random() {
        // 测试随机值
        let reserve0 = U256::from(123456789u64);
        let reserve1 = U256::from(987654321u64);
        let timestamp = U256::from(1700000000u64);

        let slot8_value: U256 = (timestamp << 224) | (reserve1 << 112) | reserve0;
        let slot8_bytes = B256::from(slot8_value.to_be_bytes::<32>());

        let state = UniswapV2State::from_slot8(mock_address(4), 400, slot8_bytes);

        assert_eq!(state.reserve0, U256::from(123456789u64));
        assert_eq!(state.reserve1, U256::from(987654321u64));
        assert_eq!(state.block_timestamp_last, 1700000000);
        assert!(state.validate().is_ok());
    }

    #[test]
    fn test_validation_zero_address() {
        let slot8_bytes = B256::ZERO;
        let state = UniswapV2State::from_slot8(Address::ZERO, 100, slot8_bytes);

        assert!(state.validate().is_err());
    }

    #[test]
    fn test_raw_pool_state_accessors() {
        let slot8_bytes = B256::ZERO;
        let state = UniswapV2State::from_slot8(mock_address(5), 500, slot8_bytes);
        let raw_state = RawPoolState::UniswapV2(state.clone());

        assert_eq!(raw_state.pool_address(), state.pool_address);
        assert_eq!(raw_state.block_number(), state.block_number);
        assert_eq!(raw_state.protocol(), ProtocolType::UniswapV2);
    }
}
