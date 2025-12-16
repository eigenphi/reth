//! UniswapV3 Factory Monitor 实现

use super::factory_monitor::FactoryMonitor;
use crate::registry::PoolMetadata;
use alloy_primitives::{Address, Log};
use alloy_sol_types::{sol, SolEvent};

// 定义 UniswapV3 PoolCreated 事件
sol! {
    /// PoolCreated(address indexed token0, address indexed token1, uint24 indexed fee, int24 tickSpacing, address pool)
    #[derive(Debug)]
    event PoolCreated(
        address indexed token0,
        address indexed token1,
        uint24 indexed fee,
        int24 tickSpacing,
        address pool
    );
}

/// UniswapV3 Factory Monitor
#[derive(Debug)]
pub struct UniswapV3Monitor {
    /// Factory 合约地址
    factory_address: Address,
}

impl UniswapV3Monitor {
    /// 创建新的 UniswapV3 Monitor
    ///
    /// # 参数
    /// * `factory_address` - UniswapV3 Factory 合约地址 主网:
    ///   0x1F98431c8aD98523631AE4a59f267346ea31F984
    pub fn new(factory_address: Address) -> Self {
        Self { factory_address }
    }

    /// UniswapV3 主网 Factory 地址
    pub const MAINNET_FACTORY: Address = Address::new([
        0x1F, 0x98, 0x43, 0x1c, 0x8a, 0xD9, 0x85, 0x23, 0x63, 0x1A, 0xE4, 0xa5, 0x9f, 0x26, 0x73,
        0x46, 0xea, 0x31, 0xF9, 0x84,
    ]);
}

impl FactoryMonitor for UniswapV3Monitor {
    fn factory_address(&self) -> Address {
        self.factory_address
    }

    fn pool_created_event_signature(&self) -> [u8; 32] {
        PoolCreated::SIGNATURE_HASH.0
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
        if log.topics().first()? != &PoolCreated::SIGNATURE_HASH {
            return None;
        }

        // 解码事件
        let event = PoolCreated::decode_log(log).ok()?;

        // 提取数据
        let token0 = event.data.token0;
        let token1 = event.data.token1;
        let fee = event.data.fee;
        let tick_spacing = event.data.tickSpacing;
        let pool = event.data.pool;

        // 验证 token 顺序（UniswapV3 约定 token0 < token1）
        if token0 >= token1 {
            tracing::warn!(?token0, ?token1, "Invalid token order in UniswapV3 PoolCreated event");
            return None;
        }

        // 创建 Pool 元数据
        Some(PoolMetadata::new_v3(
            pool,
            self.factory_address,
            token0,
            token1,
            fee.to::<u32>(),
            tick_spacing.as_i32(),
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
    fn test_parse_pool_created_event() {
        let monitor = UniswapV3Monitor::new(UniswapV3Monitor::MAINNET_FACTORY);

        // 模拟 PoolCreated 事件
        let token0 = address!("0000000000000000000000000000000000000001");
        let token1 = address!("0000000000000000000000000000000000000002");
        let pool = address!("0000000000000000000000000000000000000003");
        let fee = 3000u32; // 0.3% fee tier
        let tick_spacing = 60i32;

        let event = PoolCreated {
            token0,
            token1,
            fee: alloy_primitives::Uint::<24, 1>::from(fee),
            tickSpacing: alloy_primitives::Signed::<24, 1>::from_limbs([tick_spacing as u64]),
            pool,
        };

        let log_data = event.encode_log_data();
        let log = Log::new(
            UniswapV3Monitor::MAINNET_FACTORY,
            vec![
                PoolCreated::SIGNATURE_HASH,
                token0.into_word(),
                token1.into_word(),
                alloy_primitives::U256::from(fee).into(),
            ],
            log_data.data,
        )
        .unwrap();

        let metadata = monitor.parse_pool_created_event(&log, 1000, 0, 1638360000);

        assert!(metadata.is_some());
        let metadata = metadata.unwrap();
        assert_eq!(metadata.pool_address, pool);
        assert_eq!(metadata.token0, token0);
        assert_eq!(metadata.token1, token1);
        assert_eq!(metadata.fee, fee);
        assert_eq!(metadata.tick_spacing, Some(tick_spacing));
        assert_eq!(metadata.created_block, 1000);
    }

    #[test]
    fn test_invalid_token_order() {
        let monitor = UniswapV3Monitor::new(UniswapV3Monitor::MAINNET_FACTORY);

        // token0 > token1（无效）
        let token0 = address!("0000000000000000000000000000000000000002");
        let token1 = address!("0000000000000000000000000000000000000001");
        let pool = address!("0000000000000000000000000000000000000003");

        let event = PoolCreated {
            token0,
            token1,
            fee: alloy_primitives::Uint::<24, 1>::from(3000u32),
            tickSpacing: alloy_primitives::Signed::<24, 1>::from_limbs([60u64]),
            pool,
        };

        let log_data = event.encode_log_data();
        let log = Log::new(
            UniswapV3Monitor::MAINNET_FACTORY,
            vec![
                PoolCreated::SIGNATURE_HASH,
                token0.into_word(),
                token1.into_word(),
                alloy_primitives::U256::from(3000u32).into(),
            ],
            log_data.data,
        )
        .unwrap();

        let metadata = monitor.parse_pool_created_event(&log, 1000, 0, 1638360000);

        assert!(metadata.is_none());
    }
}
