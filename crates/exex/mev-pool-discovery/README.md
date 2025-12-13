# reth-exex-mev-pool-discovery

MEV Pool Discovery ExEx 用于自动发现区块链上新创建的 DEX Pool，无需人工维护白名单。

## 功能特性

- ✅ 监听 UniswapV2/V3 Factory 的 PoolCreated 事件
- ✅ 解析 Pool 元数据（token0, token1, fee, tick_spacing 等）
- ✅ 去重和验证（确保 Pool 唯一性和有效性）
- ✅ 维护 Pool Registry（支持多维查询）
- ✅ 可选的持久化存储（RocksDB）
- ✅ 支持初始化时导入历史 Pool 数据

## 架构设计

### 核心组件

1. **PoolMetadata**: Pool 元数据定义
2. **PoolRegistry**: Pool 注册表，提供四层索引：
   - 按 pool 地址索引
   - 按协议类型索引（UniswapV2/V3）
   - 按 token pair 索引
   - 按区块号索引
3. **FactoryMonitor trait**: 插件化的 Factory 监听器接口
4. **UniswapV2Monitor**: UniswapV2 Factory 监听器实现
5. **UniswapV3Monitor**: UniswapV3 Factory 监听器实现
6. **RocksDbStorage**: 可选的持久化存储

### 支持的协议

| 协议 | Factory 地址（主网） | 状态 |
|------|---------------------|------|
| UniswapV2 | `0x5C69bEe701ef814a2B6a3EDD4B1652CB9cc5aA6f` | ✅ 已支持 |
| UniswapV3 | `0x1F98431c8aD98523631AE4a59f267346ea31F984` | ✅ 已支持 |

## 使用方法

### 作为 ExEx 运行

```rust
use reth_exex_mev_pool_discovery::{Config, PoolDiscoveryExEx};

// 创建配置
let config = Config::default();

// 创建 ExEx
let exex = PoolDiscoveryExEx::new(config)?;

// 在 reth 节点中安装
builder
    .node(EthereumNode::default())
    .install_exex("pool-discovery", |ctx| async move {
        exex.run(ctx).await
    })
    .launch()
    .await?;
```

### 配置选项

创建自定义配置文件 `pool_discovery.json`:

```json
{
  "enable_persistence": true,
  "storage_path": "./pool_discovery_data",
  "factories": [
    {
      "name": "UniswapV2",
      "address": "0x5C69bEe701ef814a2B6a3EDD4B1652CB9cc5aA6f",
      "protocol": "uniswap_v2"
    },
    {
      "name": "UniswapV3",
      "address": "0x1F98431c8aD98523631AE4a59f267346ea31F984",
      "protocol": "uniswap_v3"
    }
  ]
}
```

然后加载配置：

```rust
let config = Config::from_file("pool_discovery.json")?;
let exex = PoolDiscoveryExEx::new(config)?;
```

## 查询 API

### 按地址查询 Pool

```rust
let pool = registry.get(&pool_address);
```

### 按协议类型查询

```rust
let v2_pools = registry.pools_by_protocol(ProtocolType::UniswapV2);
let v3_pools = registry.pools_by_protocol(ProtocolType::UniswapV3);
```

### 按 Token Pair 查询

```rust
let pools = registry.pools_by_token_pair(weth_address, usdc_address);
```

### 按区块范围查询

```rust
// 单个区块
let pools = registry.pools_by_block(12345678);

// 区块范围
let pools = registry.pools_by_block_range(12345678, 12356789);
```

### 获取统计信息

```rust
let stats = registry.stats();
println!("Total pools: {}", stats.total_pools);
println!("UniswapV2: {}", stats.uniswap_v2_pools);
println!("UniswapV3: {}", stats.uniswap_v3_pools);
```

## 数据结构

### PoolMetadata

```rust
pub struct PoolMetadata {
    pub pool_address: Address,
    pub protocol_id: ProtocolType,
    pub factory_address: Address,
    pub token0: Address,
    pub token1: Address,
    pub fee: u32,
    pub tick_spacing: Option<i32>,  // V3 专用
    pub created_block: u64,
    pub created_tx_index: u32,
    pub discovered_at: u64,
}
```

## 扩展性

### 添加新的 DEX 协议

1. 实现 `FactoryMonitor` trait:

```rust
pub struct MyDexMonitor {
    factory_address: Address,
}

impl FactoryMonitor for MyDexMonitor {
    fn factory_address(&self) -> Address {
        self.factory_address
    }

    fn parse_pool_created_event(
        &self,
        log: &Log,
        block_number: u64,
        tx_index: u32,
        discovered_at: u64,
    ) -> Option<PoolMetadata> {
        // 实现事件解析逻辑
    }

    fn pool_created_event_signature(&self) -> [u8; 32] {
        // 返回事件签名
    }
}
```

2. 在配置中添加新的 Factory

## 测试

运行单元测试：

```bash
cargo test -p reth-exex-mev-pool-discovery
```

## Feature Flags

- `rocksdb`: 启用 RocksDB 持久化存储（默认禁用）

启用 RocksDB:

```toml
[dependencies]
reth-exex-mev-pool-discovery = { version = "1.9", features = ["rocksdb"] }
```

## 性能考虑

- 使用 `DashMap` 实现高并发的 Pool Registry
- 四层索引支持 O(1) 或 O(log n) 的查询性能
- 可选的 RocksDB 持久化，避免每次重启都重新扫描区块

## 许可证

与 reth 主项目保持一致: MIT OR Apache-2.0
