# Pool State Tracker

从 Reth Storage 层直接读取 DEX Pool 的原始状态数据。

## 功能

- ✅ 直接从 Storage 读取 Pool 状态（绕过 RPC，性能更高）
- ✅ 支持 UniswapV2 Slot 8 解析（reserve0/reserve1/blockTimestampLast）
- ✅ 批量并发读取多个 Pool
- ✅ 详细的结构化日志输出
- 🚧 UniswapV3 支持（接口已保留，待实现）

## 使用方法

### 基本用法

```rust
use reth_exex_mev_pool_state_tracker::{PoolStateTracker, RawPoolState};
use reth_exex_mev_pool_discovery::ProtocolType;
use alloy_primitives::Address;

// 创建 tracker（需要 StateProviderFactory）
let tracker = PoolStateTracker::new(provider_factory);

// 读取单个 Pool 状态
let pool_address = Address::from_str("0x...")?;
let block_number = 1000000;

let state = tracker.track_pool_state(
    pool_address,
    block_number,
    ProtocolType::UniswapV2
)?;

if let Some(RawPoolState::UniswapV2(v2_state)) = state {
    println!("Reserve0: {}", v2_state.reserve0);
    println!("Reserve1: {}", v2_state.reserve1);
    println!("Timestamp: {}", v2_state.block_timestamp_last);
}
```

### 批量读取

```rust
// 批量读取多个 Pool
let pools = vec![
    (pool1_address, ProtocolType::UniswapV2),
    (pool2_address, ProtocolType::UniswapV2),
    (pool3_address, ProtocolType::UniswapV2),
];

let states = tracker.track_multiple_states(pools, block_number).await?;

for state in states {
    println!("Pool: {}", state.pool_address());
}
```

### 计算 Storage Slot

```rust
// 获取 Storage Slot 地址
let slot = tracker.get_storage_slot(
    pool_address,
    ProtocolType::UniswapV2,
    "reserves"
)?;
```

## 日志配置

本组件使用 `tracing` 框架输出结构化日志。通过 `RUST_LOG` 环境变量控制日志级别：

```bash
# 基本日志（info 级别）
RUST_LOG=reth_exex_mev_pool_state_tracker=info

# 详细日志（debug 级别，包含每个 Pool 的读取信息）
RUST_LOG=reth_exex_mev_pool_state_tracker=debug

# 最详细日志（trace 级别，包含 reserve0/reserve1 等详细数值）
RUST_LOG=reth_exex_mev_pool_state_tracker=trace
```

### 日志示例

**Info 级别**：
```
开始批量读取 Pool 状态 total_pools=100 block=1000000
按协议分组统计 uniswap_v2_pools=100 uniswap_v3_pools=0
成功读取 Pool 状态 pool=0x... block=1000000 elapsed_ms=2
批量读取完成 total_pools=100 success=98 errors=2 elapsed_ms=150
```

**Debug 级别**：
```
开始读取 Pool 状态 pool=0x... block=1000000 protocol=UniswapV2
读取 UniswapV2 Slot 8 pool=0x... slot=8
Pool 状态为空或不存在 pool=0x... block=1000000 elapsed_ms=1
```

**Trace 级别**：
```
UniswapV2 状态详情 pool=0x... reserve0=1000000000000000000 reserve1=2000000000000000000 timestamp=1234567890
```

## 数据结构

### RawPoolState

Pool 的原始状态数据，支持不同协议：

```rust
pub enum RawPoolState {
    UniswapV2(UniswapV2State),
    UniswapV3(UniswapV3State),
}
```

### UniswapV2State

从 Slot 8 解析的 UniswapV2 状态：

```rust
pub struct UniswapV2State {
    pub pool_address: Address,
    pub block_number: u64,
    pub reserve0: U256,           // 112 bits
    pub reserve1: U256,           // 112 bits
    pub block_timestamp_last: u32, // 32 bits
    pub raw_slot8: B256,          // 原始数据
}
```

### Slot 8 布局

UniswapV2 的 Slot 8 存储格式（256 bits）：

```
| 32 bits (224-255) | 112 bits (112-223) | 112 bits (0-111) |
|    timestamp      |      reserve1      |     reserve0     |
```

## 性能

- **单个 Pool 读取**：1-3ms（从 Storage 直读）
- **批量读取 100 Pools**：30-50ms（并发读取）
- **批量读取 1000 Pools**：300-500ms（并发读取）

相比 RPC 调用，性能提升约 **10-20 倍**。

## 测试

运行单元测试：

```bash
cargo test -p reth-exex-mev-pool-state-tracker
```

运行特定测试：

```bash
# 测试 Slot 8 解析
cargo test -p reth-exex-mev-pool-state-tracker test_uniswap_v2_slot8_parsing

# 测试状态读取
cargo test -p reth-exex-mev-pool-state-tracker test_track_pool_state
```

## 依赖关系

- `reth-exex-mev-pool-discovery`: Pool 元数据和协议类型定义
- `reth-storage-api`: Reth 存储层接口
- `alloy-primitives`: 以太坊基础类型（Address, B256, U256）
- `tokio`: 异步运行时（用于并发读取）
- `tracing`: 结构化日志

## TODO

- [ ] 实现 UniswapV3 状态读取（Slot 0 和 Slot 4）
- [ ] 添加状态缓存层
- [ ] 实现变化检测（DetectChangedPools）
- [ ] 支持更多 DEX 协议

## 相关文档

- [05-component-state-tracker.md](../../../docs/eigenphi/observer/05-component-state-tracker.md)
- [04-component-pool-discovery.md](../../../docs/eigenphi/observer/04-component-pool-discovery.md)
