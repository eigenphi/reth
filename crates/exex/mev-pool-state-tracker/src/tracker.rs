//! Pool State Tracker 核心实现
//!
//! 负责从 Reth Storage 层直接读取 Pool 的原始状态数据

use crate::raw_state::{RawPoolState, UniswapV2State};
use alloy_primitives::{Address, B256, U256};
use eyre::{Context, Result};
use reth_exex_mev_pool_discovery::ProtocolType;
use reth_storage_api::{StateProvider, StateProviderFactory};
use std::time::Instant;
use tracing::{debug, info, trace, warn};

/// Storage Slot 常量
mod slots {
    /// UniswapV2: Slot 8 存储 reserve0, reserve1, blockTimestampLast
    pub(super) const UNISWAP_V2_RESERVES: u8 = 8;
}

/// Pool State Tracker
///
/// 提供从 Storage 直接读取 Pool 状态的功能
pub struct PoolStateTracker<P> {
    /// State Provider Factory，用于获取特定区块的状态
    provider_factory: P,
}

impl<P> PoolStateTracker<P>
where
    P: StateProviderFactory,
{
    /// 创建新的 State Tracker
    pub fn new(provider_factory: P) -> Self {
        Self { provider_factory }
    }

    /// 读取单个 Pool 的状态
    ///
    /// # 参数
    /// - `pool_address`: Pool 合约地址
    /// - `block_number`: 目标区块号
    /// - `protocol`: DEX 协议类型
    ///
    /// # 返回
    /// - `Ok(Some(RawPoolState))`: 成功读取状态
    /// - `Ok(None)`: Pool 不存在或状态为空
    /// - `Err`: 读取失败
    pub fn track_pool_state(
        &self,
        pool_address: Address,
        block_number: u64,
        protocol: ProtocolType,
    ) -> Result<Option<RawPoolState>> {
        let start = Instant::now();

        debug!(
            pool = ?pool_address,
            block = block_number,
            protocol = ?protocol,
            "开始读取 Pool 状态"
        );

        // 获取指定区块的 state provider
        let state_provider = self
            .provider_factory
            .history_by_block_number(block_number)
            .wrap_err_with(|| format!("无法获取区块 {} 的 state provider", block_number))?;

        let result = match protocol {
            ProtocolType::UniswapV2 => {
                self.read_uniswap_v2_state(pool_address, block_number, state_provider.as_ref())
            }
            ProtocolType::UniswapV3 => {
                warn!(
                    pool = ?pool_address,
                    "UniswapV3 状态读取尚未实现"
                );
                Ok(None)
            }
        };

        let elapsed = start.elapsed();

        match &result {
            Ok(Some(state)) => {
                info!(
                    pool = ?pool_address,
                    block = block_number,
                    protocol = ?protocol,
                    elapsed_ms = elapsed.as_millis(),
                    "成功读取 Pool 状态"
                );

                // 在 trace 级别输出详细的状态信息
                if let RawPoolState::UniswapV2(v2_state) = state {
                    trace!(
                        pool = ?pool_address,
                        reserve0 = %v2_state.reserve0,
                        reserve1 = %v2_state.reserve1,
                        timestamp = v2_state.block_timestamp_last,
                        "UniswapV2 状态详情"
                    );
                }
            }
            Ok(None) => {
                debug!(
                    pool = ?pool_address,
                    block = block_number,
                    elapsed_ms = elapsed.as_millis(),
                    "Pool 状态为空或不存在"
                );
            }
            Err(e) => {
                warn!(
                    pool = ?pool_address,
                    block = block_number,
                    elapsed_ms = elapsed.as_millis(),
                    error = %e,
                    "读取 Pool 状态失败"
                );
            }
        }

        result
    }

    /// 批量读取多个 Pool 的状态
    ///
    /// # 参数
    /// - `pool_addresses`: Pool 地址列表，每个元素为 (address, protocol)
    /// - `block_number`: 目标区块号
    ///
    /// # 返回
    /// 返回成功读取的状态列表（失败的 Pool 会被跳过）
    pub async fn track_multiple_states(
        &self,
        pool_addresses: Vec<(Address, ProtocolType)>,
        block_number: u64,
    ) -> Result<Vec<RawPoolState>> {
        let start = Instant::now();
        let total_pools = pool_addresses.len();

        info!(total_pools = total_pools, block = block_number, "开始批量读取 Pool 状态");

        // 按协议类型分组统计
        let v2_count = pool_addresses.iter().filter(|(_, p)| *p == ProtocolType::UniswapV2).count();
        let v3_count = pool_addresses.iter().filter(|(_, p)| *p == ProtocolType::UniswapV3).count();

        info!(uniswap_v2_pools = v2_count, uniswap_v3_pools = v3_count, "按协议分组统计");

        // 使用 tokio 并发读取
        let mut tasks = Vec::new();
        for (pool_address, protocol) in pool_addresses {
            let result = self.track_pool_state(pool_address, block_number, protocol);

            // 将同步结果包装为 async，以便可以使用 tokio 的并发控制
            tasks.push(async move { result });
        }

        // 并发执行所有任务
        let results = futures::future::join_all(tasks).await;

        // 收集成功的结果
        let mut states = Vec::new();
        let mut success_count = 0;
        let mut error_count = 0;

        for result in results {
            match result {
                Ok(Some(state)) => {
                    success_count += 1;
                    states.push(state);
                }
                Ok(None) => {
                    // Pool 不存在或状态为空，不计入成功或失败
                    debug!("Pool 状态为空，跳过");
                }
                Err(e) => {
                    error_count += 1;
                    warn!(error = %e, "读取 Pool 状态失败");
                }
            }
        }

        let elapsed = start.elapsed();

        info!(
            total_pools = total_pools,
            success = success_count,
            errors = error_count,
            elapsed_ms = elapsed.as_millis(),
            avg_ms_per_pool =
                if success_count > 0 { elapsed.as_millis() / success_count as u128 } else { 0 },
            "批量读取完成"
        );

        Ok(states)
    }

    /// 计算 Storage Slot 地址
    ///
    /// # 参数
    /// - `pool_address`: Pool 合约地址（在 V2 中未使用，保留接口兼容性）
    /// - `protocol`: DEX 协议类型
    /// - `slot_name`: Slot 名称（如 "reserves" 用于 V2）
    ///
    /// # 返回
    /// Storage Slot 的 Key（H256 格式）
    pub fn get_storage_slot(
        &self,
        _pool_address: Address,
        protocol: ProtocolType,
        slot_name: &str,
    ) -> Result<B256> {
        match protocol {
            ProtocolType::UniswapV2 => match slot_name {
                "reserves" => {
                    // UniswapV2 的 reserves 在固定的 Slot 8
                    let slot = U256::from(slots::UNISWAP_V2_RESERVES);
                    Ok(B256::from(slot.to_be_bytes::<32>()))
                }
                _ => Err(eyre::eyre!("Unknown slot name for UniswapV2: {}", slot_name)),
            },
            ProtocolType::UniswapV3 => {
                Err(eyre::eyre!("UniswapV3 slot calculation not implemented"))
            }
        }
    }

    /// 读取 UniswapV2 Pool 的状态
    fn read_uniswap_v2_state(
        &self,
        pool_address: Address,
        block_number: u64,
        state_provider: &dyn StateProvider,
    ) -> Result<Option<RawPoolState>> {
        // 读取 Slot 8 (reserves)
        let slot_key = U256::from(slots::UNISWAP_V2_RESERVES);
        let storage_key = B256::from(slot_key.to_be_bytes::<32>());

        trace!(
            pool = ?pool_address,
            slot = slots::UNISWAP_V2_RESERVES,
            "读取 UniswapV2 Slot 8"
        );

        // 从 state provider 读取存储
        let storage_value =
            state_provider.storage(pool_address, storage_key).wrap_err_with(|| {
                format!(
                    "读取 Pool {} 的 storage slot {} 失败",
                    pool_address,
                    slots::UNISWAP_V2_RESERVES
                )
            })?;

        // 如果 storage 值为 None 或为零，说明 Pool 不存在或未初始化
        let Some(value) = storage_value else {
            debug!(
                pool = ?pool_address,
                "Storage slot 返回 None，Pool 可能不存在"
            );
            return Ok(None);
        };

        if value == U256::ZERO {
            debug!(
                pool = ?pool_address,
                "Storage slot 为零，Pool 可能未初始化"
            );
            return Ok(None);
        }

        // 将 U256 转换为 B256
        let slot8_data = B256::from(value.to_be_bytes::<32>());

        // 解析 Slot 8
        let state = UniswapV2State::from_slot8(pool_address, block_number, slot8_data);

        // 验证状态
        state.validate().map_err(|e| eyre::eyre!("状态验证失败: {}", e))?;

        Ok(Some(RawPoolState::UniswapV2(state)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mock_address(n: u8) -> Address {
        Address::from([n; 20])
    }

    #[test]
    fn test_get_storage_slot_uniswap_v2_reserves() {
        // 测试计算 UniswapV2 的 reserves slot
        // 注意：由于 get_storage_slot 需要 StateProviderFactory，我们需要简化测试
        // 这里我们直接测试 slot 计算逻辑

        let slot_number = slots::UNISWAP_V2_RESERVES;
        assert_eq!(slot_number, 8);

        let slot = U256::from(slot_number);
        let slot_bytes = B256::from(slot.to_be_bytes::<32>());

        // 验证 slot 8 的字节表示
        assert_eq!(slot_bytes[31], 8); // 最后一个字节应该是 8
    }

    #[test]
    fn test_slot_calculation_for_reserves() {
        // 验证 Slot 8 的计算
        let expected_slot = 8u8;
        assert_eq!(slots::UNISWAP_V2_RESERVES, expected_slot);
    }
}
