//! 持久化存储模块

#[cfg(feature = "rocksdb")]
pub mod rocksdb;

#[cfg(feature = "rocksdb")]
pub use self::rocksdb::RocksDbStorage;
