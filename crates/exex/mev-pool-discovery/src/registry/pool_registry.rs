//! Pool Registry 实现
//!
//! 提供四层索引结构支持多维查询

use super::pool_metadata::{PoolMetadata, ProtocolType};
use alloy_primitives::Address;
use dashmap::DashMap;
use std::{
    collections::{BTreeMap, HashMap, HashSet},
    sync::{Arc, RwLock},
};

/// Pool Registry
///
/// 维护所有已发现的 Pool 信息，支持多维查询
pub struct PoolRegistry {
    /// 主索引：按 pool 地址索引
    pools: DashMap<Address, PoolMetadata>,

    /// 辅助索引：按协议类型索引
    by_protocol: Arc<RwLock<HashMap<ProtocolType, HashSet<Address>>>>,

    /// 辅助索引：按 token pair 索引
    by_token_pair: Arc<RwLock<HashMap<(Address, Address), Vec<Address>>>>,

    /// 辅助索引：按区块号索引
    by_block: Arc<RwLock<BTreeMap<u64, Vec<Address>>>>,
}

impl Default for PoolRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl PoolRegistry {
    /// 创建新的 Pool Registry
    pub fn new() -> Self {
        Self {
            pools: DashMap::new(),
            by_protocol: Arc::new(RwLock::new(HashMap::new())),
            by_token_pair: Arc::new(RwLock::new(HashMap::new())),
            by_block: Arc::new(RwLock::new(BTreeMap::new())),
        }
    }

    /// 插入新的 Pool
    ///
    /// 如果 Pool 已存在，返回 false；否则插入并返回 true
    pub fn insert(&self, metadata: PoolMetadata) -> Result<bool, String> {
        // 验证元数据
        metadata.validate()?;

        let pool_address = metadata.pool_address;

        // 检查是否已存在
        if self.pools.contains_key(&pool_address) {
            return Ok(false);
        }

        // 插入主索引
        self.pools.insert(pool_address, metadata.clone());

        // 更新协议类型索引
        {
            let mut by_protocol = self.by_protocol.write().unwrap();
            by_protocol
                .entry(metadata.protocol_id)
                .or_insert_with(HashSet::new)
                .insert(pool_address);
        }

        // 更新 token pair 索引
        {
            let mut by_token_pair = self.by_token_pair.write().unwrap();
            let pair_key = (metadata.token0, metadata.token1);
            by_token_pair.entry(pair_key).or_insert_with(Vec::new).push(pool_address);
        }

        // 更新区块号索引
        {
            let mut by_block = self.by_block.write().unwrap();
            by_block.entry(metadata.created_block).or_insert_with(Vec::new).push(pool_address);
        }

        Ok(true)
    }

    /// 检查 Pool 是否存在
    pub fn contains(&self, pool_address: &Address) -> bool {
        self.pools.contains_key(pool_address)
    }

    /// 获取 Pool 元数据
    pub fn get(&self, pool_address: &Address) -> Option<PoolMetadata> {
        self.pools.get(pool_address).map(|entry| entry.value().clone())
    }

    /// 获取所有 Pool 地址
    pub fn all_pools(&self) -> Vec<Address> {
        self.pools.iter().map(|entry| *entry.key()).collect()
    }

    /// 按协议类型获取 Pool
    pub fn pools_by_protocol(&self, protocol: ProtocolType) -> Vec<Address> {
        let by_protocol = self.by_protocol.read().unwrap();
        by_protocol.get(&protocol).map(|addrs| addrs.iter().copied().collect()).unwrap_or_default()
    }

    /// 按 token pair 获取 Pool
    pub fn pools_by_token_pair(&self, token0: Address, token1: Address) -> Vec<Address> {
        let by_token_pair = self.by_token_pair.read().unwrap();
        by_token_pair.get(&(token0, token1)).cloned().unwrap_or_default()
    }

    /// 获取指定区块创建的 Pool
    pub fn pools_by_block(&self, block_number: u64) -> Vec<Address> {
        let by_block = self.by_block.read().unwrap();
        by_block.get(&block_number).cloned().unwrap_or_default()
    }

    /// 获取指定区块范围内创建的 Pool
    pub fn pools_by_block_range(&self, from: u64, to: u64) -> Vec<Address> {
        let by_block = self.by_block.read().unwrap();
        by_block.range(from..=to).flat_map(|(_, addrs)| addrs.clone()).collect()
    }

    /// 获取 Pool 总数
    pub fn len(&self) -> usize {
        self.pools.len()
    }

    /// 检查是否为空
    pub fn is_empty(&self) -> bool {
        self.pools.is_empty()
    }

    /// 获取统计信息
    pub fn stats(&self) -> RegistryStats {
        let total = self.len();
        let by_protocol = self.by_protocol.read().unwrap();
        let v2_count = by_protocol.get(&ProtocolType::UniswapV2).map(|s| s.len()).unwrap_or(0);
        let v3_count = by_protocol.get(&ProtocolType::UniswapV3).map(|s| s.len()).unwrap_or(0);

        RegistryStats { total_pools: total, uniswap_v2_pools: v2_count, uniswap_v3_pools: v3_count }
    }
}

/// Registry 统计信息
#[derive(Debug, Clone)]
pub struct RegistryStats {
    /// Pool 总数
    pub total_pools: usize,
    /// UniswapV2 Pool 数量
    pub uniswap_v2_pools: usize,
    /// UniswapV3 Pool 数量
    pub uniswap_v3_pools: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mock_address(n: u8) -> Address {
        Address::from([n; 20])
    }

    fn create_test_pool(pool: u8, token0: u8, token1: u8, block: u64) -> PoolMetadata {
        PoolMetadata::new_v2(
            mock_address(pool),
            mock_address(100),
            mock_address(token0),
            mock_address(token1),
            block,
            0,
            1638360000,
        )
    }

    #[test]
    fn test_insert_and_get() {
        let registry = PoolRegistry::new();
        let pool = create_test_pool(1, 2, 3, 1000);

        assert!(registry.insert(pool.clone()).unwrap());
        assert!(registry.contains(&pool.pool_address));
        assert_eq!(registry.get(&pool.pool_address), Some(pool));
    }

    #[test]
    fn test_duplicate_insert() {
        let registry = PoolRegistry::new();
        let pool = create_test_pool(1, 2, 3, 1000);

        assert!(registry.insert(pool.clone()).unwrap());
        assert!(!registry.insert(pool).unwrap()); // 第二次插入应该返回 false
    }

    #[test]
    fn test_query_by_protocol() {
        let registry = PoolRegistry::new();
        let pool1 = create_test_pool(1, 2, 3, 1000);
        let mut pool2 = create_test_pool(4, 5, 6, 1000);
        pool2.protocol_id = ProtocolType::UniswapV3;
        pool2.tick_spacing = Some(60);

        registry.insert(pool1.clone()).unwrap();
        registry.insert(pool2.clone()).unwrap();

        let v2_pools = registry.pools_by_protocol(ProtocolType::UniswapV2);
        assert_eq!(v2_pools.len(), 1);
        assert!(v2_pools.contains(&pool1.pool_address));

        let v3_pools = registry.pools_by_protocol(ProtocolType::UniswapV3);
        assert_eq!(v3_pools.len(), 1);
        assert!(v3_pools.contains(&pool2.pool_address));
    }

    #[test]
    fn test_query_by_token_pair() {
        let registry = PoolRegistry::new();
        let pool1 = create_test_pool(1, 2, 3, 1000);
        let pool2 = create_test_pool(4, 2, 3, 1000); // 同样的 token pair

        registry.insert(pool1.clone()).unwrap();
        registry.insert(pool2.clone()).unwrap();

        let pools = registry.pools_by_token_pair(mock_address(2), mock_address(3));
        assert_eq!(pools.len(), 2);
        assert!(pools.contains(&pool1.pool_address));
        assert!(pools.contains(&pool2.pool_address));
    }

    #[test]
    fn test_query_by_block() {
        let registry = PoolRegistry::new();
        let pool1 = create_test_pool(1, 2, 3, 1000);
        let pool2 = create_test_pool(4, 5, 6, 1000);
        let pool3 = create_test_pool(7, 8, 9, 2000);

        registry.insert(pool1.clone()).unwrap();
        registry.insert(pool2.clone()).unwrap();
        registry.insert(pool3.clone()).unwrap();

        let pools_1000 = registry.pools_by_block(1000);
        assert_eq!(pools_1000.len(), 2);

        let pools_2000 = registry.pools_by_block(2000);
        assert_eq!(pools_2000.len(), 1);
        assert!(pools_2000.contains(&pool3.pool_address));
    }

    #[test]
    fn test_query_by_block_range() {
        let registry = PoolRegistry::new();
        registry.insert(create_test_pool(1, 2, 3, 1000)).unwrap();
        registry.insert(create_test_pool(4, 5, 6, 1500)).unwrap();
        registry.insert(create_test_pool(7, 8, 9, 2000)).unwrap();

        let pools = registry.pools_by_block_range(1000, 1500);
        assert_eq!(pools.len(), 2);
    }

    #[test]
    fn test_stats() {
        let registry = PoolRegistry::new();
        let pool1 = create_test_pool(1, 2, 3, 1000);
        let mut pool2 = create_test_pool(4, 5, 6, 1000);
        pool2.protocol_id = ProtocolType::UniswapV3;
        pool2.tick_spacing = Some(60);

        registry.insert(pool1).unwrap();
        registry.insert(pool2).unwrap();

        let stats = registry.stats();
        assert_eq!(stats.total_pools, 2);
        assert_eq!(stats.uniswap_v2_pools, 1);
        assert_eq!(stats.uniswap_v3_pools, 1);
    }
}
