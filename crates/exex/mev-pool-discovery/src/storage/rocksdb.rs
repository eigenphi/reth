//! RocksDB 持久化存储实现

use crate::registry::PoolMetadata;
use alloy_primitives::Address;
use eyre::Result;
use rocksdb::{Options, DB};
use std::path::Path;

/// RocksDB 存储
pub struct RocksDbStorage {
    db: DB,
}

impl RocksDbStorage {
    /// 打开或创建 RocksDB 数据库
    pub fn new<P: AsRef<Path>>(path: P) -> Result<Self> {
        let mut opts = Options::default();
        opts.create_if_missing(true);
        opts.set_compression_type(rocksdb::DBCompressionType::Snappy);

        let db = DB::open(&opts, path)?;
        Ok(Self { db })
    }

    /// 保存 Pool 元数据
    pub fn save_pool(&self, metadata: &PoolMetadata) -> Result<()> {
        let key = metadata.pool_address.to_vec();
        let value = serde_json::to_vec(metadata)?;
        self.db.put(key, value)?;
        Ok(())
    }

    /// 读取 Pool 元数据
    pub fn get_pool(&self, pool_address: &Address) -> Result<Option<PoolMetadata>> {
        let key = pool_address.to_vec();
        match self.db.get(key)? {
            Some(value) => {
                let metadata: PoolMetadata = serde_json::from_slice(&value)?;
                Ok(Some(metadata))
            }
            None => Ok(None),
        }
    }

    /// 删除 Pool 元数据
    pub fn delete_pool(&self, pool_address: &Address) -> Result<()> {
        let key = pool_address.to_vec();
        self.db.delete(key)?;
        Ok(())
    }

    /// 获取所有 Pool
    pub fn all_pools(&self) -> Result<Vec<PoolMetadata>> {
        let mut pools = Vec::new();
        let iter = self.db.iterator(rocksdb::IteratorMode::Start);

        for item in iter {
            let (_, value) = item?;
            let metadata: PoolMetadata = serde_json::from_slice(&value)?;
            pools.push(metadata);
        }

        Ok(pools)
    }

    /// 批量保存 Pool
    pub fn save_batch(&self, pools: &[PoolMetadata]) -> Result<()> {
        let mut batch = rocksdb::WriteBatch::default();

        for metadata in pools {
            let key = metadata.pool_address.to_vec();
            let value = serde_json::to_vec(metadata)?;
            batch.put(key, value);
        }

        self.db.write(batch)?;
        Ok(())
    }

    /// 获取数据库中的 Pool 数量
    pub fn count(&self) -> Result<usize> {
        let mut count = 0;
        let iter = self.db.iterator(rocksdb::IteratorMode::Start);

        for item in iter {
            item?;
            count += 1;
        }

        Ok(count)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn mock_address(n: u8) -> Address {
        Address::from([n; 20])
    }

    fn create_test_pool(pool: u8) -> PoolMetadata {
        PoolMetadata::new_v2(
            mock_address(pool),
            mock_address(100),
            mock_address(2),
            mock_address(3),
            1000,
            0,
            1638360000,
        )
    }

    #[test]
    fn test_save_and_get_pool() {
        let temp_dir = TempDir::new().unwrap();
        let storage = RocksDbStorage::new(temp_dir.path()).unwrap();

        let pool = create_test_pool(1);
        storage.save_pool(&pool).unwrap();

        let loaded = storage.get_pool(&pool.pool_address).unwrap();
        assert!(loaded.is_some());
        assert_eq!(loaded.unwrap(), pool);
    }

    #[test]
    fn test_delete_pool() {
        let temp_dir = TempDir::new().unwrap();
        let storage = RocksDbStorage::new(temp_dir.path()).unwrap();

        let pool = create_test_pool(1);
        storage.save_pool(&pool).unwrap();
        assert!(storage.get_pool(&pool.pool_address).unwrap().is_some());

        storage.delete_pool(&pool.pool_address).unwrap();
        assert!(storage.get_pool(&pool.pool_address).unwrap().is_none());
    }

    #[test]
    fn test_all_pools() {
        let temp_dir = TempDir::new().unwrap();
        let storage = RocksDbStorage::new(temp_dir.path()).unwrap();

        let pool1 = create_test_pool(1);
        let pool2 = create_test_pool(2);

        storage.save_pool(&pool1).unwrap();
        storage.save_pool(&pool2).unwrap();

        let all_pools = storage.all_pools().unwrap();
        assert_eq!(all_pools.len(), 2);
    }

    #[test]
    fn test_batch_save() {
        let temp_dir = TempDir::new().unwrap();
        let storage = RocksDbStorage::new(temp_dir.path()).unwrap();

        let pools = vec![create_test_pool(1), create_test_pool(2), create_test_pool(3)];

        storage.save_batch(&pools).unwrap();

        let count = storage.count().unwrap();
        assert_eq!(count, 3);
    }
}
