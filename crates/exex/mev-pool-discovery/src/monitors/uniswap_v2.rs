//! UniswapV2 Factory Monitor 实现

use super::factory_monitor::FactoryMonitor;
use crate::registry::PoolMetadata;
use alloy_primitives::{Address, Log};
use alloy_sol_types::{sol, SolEvent};

// 定义 UniswapV2 PairCreated 事件
sol! {
    /// PairCreated(address indexed token0, address indexed token1, address pair, uint)
    #[derive(Debug)]
    event PairCreated(
        address indexed token0,
        address indexed token1,
        address pair,
        uint256 pairIndex
    );
}

/// UniswapV2 Factory Monitor
#[derive(Debug)]
pub struct UniswapV2Monitor {
    /// Factory 合约地址
    factory_address: Address,
}

impl UniswapV2Monitor {
    /// 创建新的 UniswapV2 Monitor
    ///
    /// # 参数
    /// * `factory_address` - UniswapV2 Factory 合约地址 主网:
    ///   0x5C69bEe701ef814a2B6a3EDD4B1652CB9cc5aA6f
    pub fn new(factory_address: Address) -> Self {
        Self { factory_address }
    }

    /// UniswapV2 主网 Factory 地址
    pub const MAINNET_FACTORY: Address = Address::new([
        0x5C, 0x69, 0xbE, 0xe7, 0x01, 0xef, 0x81, 0x4a, 0x2B, 0x6a, 0x3E, 0xDD, 0x4B, 0x16, 0x52,
        0xCB, 0x9c, 0xc5, 0xaA, 0x6f,
    ]);
}

impl FactoryMonitor for UniswapV2Monitor {
    fn factory_address(&self) -> Address {
        self.factory_address
    }

    fn pool_created_event_signature(&self) -> [u8; 32] {
        PairCreated::SIGNATURE_HASH.0
    }

    fn parse_pool_created_event(
        &self,
        log: &Log,
        block_number: u64,
        tx_index: u32,
        discovered_at: u64,
    ) -> Option<PoolMetadata> {
        // 检查事件发送者是否为 Factory
        if log.address != self.factory_address {
            return None;
        }

        // 检查事件签名
        if log.topics().first()? != &PairCreated::SIGNATURE_HASH {
            return None;
        }

        // 解码事件
        let event = PairCreated::decode_log(log).ok()?;

        // 提取数据
        let token0 = event.data.token0;
        let token1 = event.data.token1;
        let pair = event.data.pair;

        // 验证 token 顺序（UniswapV2 约定 token0 < token1）
        if token0 >= token1 {
            tracing::warn!(?token0, ?token1, "Invalid token order in UniswapV2 PairCreated event");
            return None;
        }

        // 创建 Pool 元数据
        Some(PoolMetadata::new_v2(
            pair,
            self.factory_address,
            token0,
            token1,
            block_number,
            tx_index,
            discovered_at,
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloy_primitives::address;

    #[test]
    fn test_parse_pair_created_event() {
        let monitor = UniswapV2Monitor::new(UniswapV2Monitor::MAINNET_FACTORY);

        // 模拟 PairCreated 事件
        let token0 = address!("0000000000000000000000000000000000000001");
        let token1 = address!("0000000000000000000000000000000000000002");
        let pair = address!("0000000000000000000000000000000000000003");

        let event =
            PairCreated { token0, token1, pair, pairIndex: alloy_primitives::U256::from(1) };

        let log_data = event.encode_log_data();
        let log = Log::new(
            UniswapV2Monitor::MAINNET_FACTORY,
            vec![PairCreated::SIGNATURE_HASH, token0.into_word(), token1.into_word()],
            log_data.data,
        )
        .unwrap();

        let metadata = monitor.parse_pool_created_event(&log, 1000, 0, 1638360000);

        assert!(metadata.is_some());
        let metadata = metadata.unwrap();
        assert_eq!(metadata.pool_address, pair);
        assert_eq!(metadata.token0, token0);
        assert_eq!(metadata.token1, token1);
        assert_eq!(metadata.created_block, 1000);
    }

    #[test]
    fn test_invalid_token_order() {
        let monitor = UniswapV2Monitor::new(UniswapV2Monitor::MAINNET_FACTORY);

        // token0 > token1（无效）
        let token0 = address!("0000000000000000000000000000000000000002");
        let token1 = address!("0000000000000000000000000000000000000001");
        let pair = address!("0000000000000000000000000000000000000003");

        let event =
            PairCreated { token0, token1, pair, pairIndex: alloy_primitives::U256::from(1) };

        let log_data = event.encode_log_data();
        let log = Log::new(
            UniswapV2Monitor::MAINNET_FACTORY,
            vec![PairCreated::SIGNATURE_HASH, token0.into_word(), token1.into_word()],
            log_data.data,
        )
        .unwrap();

        let metadata = monitor.parse_pool_created_event(&log, 1000, 0, 1638360000);

        assert!(metadata.is_none());
    }
}
