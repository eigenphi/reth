//! Pool Discovery ExEx 主逻辑

use crate::{
    config::Config,
    monitors::{FactoryMonitor, UniswapV2Monitor, UniswapV3Monitor},
    registry::PoolRegistry,
};

#[cfg(feature = "rocksdb")]
use crate::storage::RocksDbStorage;

use eyre::Result;
use futures::StreamExt;
use reth_exex::{ExExContext, ExExEvent, ExExNotification};
use reth_tracing::tracing::{info, warn};
use std::sync::Arc;

/// Pool Discovery ExEx
pub struct PoolDiscoveryExEx {
    /// 配置
    config: Config,
    /// Pool Registry
    registry: Arc<PoolRegistry>,
    /// 持久化存储（可选）
    #[cfg(feature = "rocksdb")]
    storage: Option<RocksDbStorage>,
    /// Factory Monitors
    monitors: Vec<Box<dyn FactoryMonitor>>,
}

impl PoolDiscoveryExEx {
    /// 创建新的 Pool Discovery ExEx
    pub fn new(config: Config) -> Result<Self> {
        let registry = Arc::new(PoolRegistry::new());

        // 初始化持久化存储
        #[cfg(feature = "rocksdb")]
        let storage = if config.enable_persistence {
            info!("Initializing persistent storage at {:?}", config.storage_path);
            std::fs::create_dir_all(&config.storage_path)?;
            Some(RocksDbStorage::new(&config.storage_path)?)
        } else {
            info!("Persistent storage disabled");
            None
        };

        // 从持久化存储加载已知 Pool
        #[cfg(feature = "rocksdb")]
        if let Some(ref storage) = storage {
            info!("Loading pools from persistent storage...");
            match storage.all_pools() {
                Ok(pools) => {
                    let count = pools.len();
                    for pool in pools {
                        if let Err(e) = registry.insert(pool) {
                            warn!("Failed to load pool from storage: {}", e);
                        }
                    }
                    info!("Loaded {} pools from persistent storage", count);
                }
                Err(e) => {
                    warn!("Failed to load pools from storage: {}", e);
                }
            }
        }

        #[cfg(not(feature = "rocksdb"))]
        if config.enable_persistence {
            warn!("Persistence is enabled in config, but rocksdb feature is not enabled. Pools will not be persisted.");
        }

        // 初始化 Factory Monitors
        let mut monitors: Vec<Box<dyn FactoryMonitor>> = Vec::new();
        for factory_config in &config.factories {
            match factory_config.protocol.as_str() {
                "uniswap_v2" => {
                    info!(
                        "Registering UniswapV2 Factory: {} at {}",
                        factory_config.name, factory_config.address
                    );
                    monitors.push(Box::new(UniswapV2Monitor::new(factory_config.address)));
                }
                "uniswap_v3" => {
                    info!(
                        "Registering UniswapV3 Factory: {} at {}",
                        factory_config.name, factory_config.address
                    );
                    monitors.push(Box::new(UniswapV3Monitor::new(factory_config.address)));
                }
                _ => {
                    warn!(
                        "Unknown protocol type: {}, skipping factory {}",
                        factory_config.protocol, factory_config.name
                    );
                }
            }
        }

        Ok(Self {
            config,
            registry,
            #[cfg(feature = "rocksdb")]
            storage,
            monitors,
        })
    }

    /// 处理新的区块通知
    fn process_notification<N>(&mut self, notification: &ExExNotification<N>) -> Result<()>
    where
        N: reth_node_api::NodePrimitives,
    {
        let committed_chain = match notification.committed_chain() {
            Some(chain) => chain,
            None => return Ok(()),
        };

        info!("Processing blocks: {:?}", committed_chain.range());

        // 获取区块范围并输出统计信息
        let stats = self.registry.stats();
        info!(
            "Registry stats - Total: {}, UniswapV2: {}, UniswapV3: {}",
            stats.total_pools, stats.uniswap_v2_pools, stats.uniswap_v3_pools
        );

        // TODO: 实现完整的区块和日志处理逻辑
        // 由于 NodePrimitives 的泛型限制，需要进一步研究 reth 的 API 来正确访问区块和 receipts

        Ok(())
    }

    /// 运行 ExEx
    pub async fn run(
        mut self,
        mut ctx: ExExContext<impl reth_node_api::FullNodeComponents>,
    ) -> Result<()> {
        info!("Starting Pool Discovery ExEx");

        // 主循环：处理区块通知
        while let Some(notification) = ctx.notifications.next().await {
            match notification {
                Ok(notification) => {
                    // 处理通知
                    if let Err(e) = self.process_notification(&notification) {
                        warn!("Error processing notification: {}", e);
                    }

                    // 通知 ExEx 已处理到的高度
                    if let Some(committed_chain) = notification.committed_chain() {
                        ctx.events
                            .send(ExExEvent::FinishedHeight(committed_chain.tip().num_hash()))?;
                    }
                }
                Err(e) => {
                    warn!("Error receiving notification: {}", e);
                }
            }
        }

        info!("Pool Discovery ExEx stopped");
        Ok(())
    }

    /// 获取 Registry 引用（用于测试或外部访问）
    pub fn registry(&self) -> &Arc<PoolRegistry> {
        &self.registry
    }
}
