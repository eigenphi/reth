//! Registry 模块
//!
//! 提供 Pool 元数据管理和查询功能

pub mod pool_metadata;
pub mod pool_registry;

pub use pool_metadata::{PoolMetadata, ProtocolType};
pub use pool_registry::{PoolRegistry, RegistryStats};
