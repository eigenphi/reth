# 术语表

## 文档元信息

| 项目 | 内容 |
|------|------|
| 文档版本 | v1.0 |
| 创建日期 | 2025-12-12 |
| 最后更新 | 2025-12-12 |
| 文档状态 | 草稿 |
| 责任人 | 架构组 |

## A

### AMM (Automated Market Maker)
**中文**：自动化做市商  
**定义**：使用数学公式（如 x × y = k）自动提供流动性和定价的机制，无需传统做市商。  
**示例**：Uniswap, Curve, Balancer

### Arbitrage
**中文**：套利  
**定义**：利用不同市场间的价差进行无风险交易获利。  
**相关**：MEV, Searcher

## B

### Block Number
**中文**：区块号  
**定义**：区块在区块链上的序号，从 0（创世区块）开始递增。  
**用途**：标识 State 所属的区块位置

### Business State
**中文**：业务状态  
**定义**：经过转换和标准化的 Pool State，直接可供业务系统使用。  
**对比**：Raw State（原始 Storage 数据）

## C

### Constant Product Formula
**中文**：恒定乘积公式  
**定义**：UniswapV2 的核心公式 `x × y = k`，确保流动性池的数学不变性。  
**公式**：reserve0 × reserve1 = k

## D

### DEX (Decentralized Exchange)
**中文**：去中心化交易所  
**定义**：无需中心化中介，基于智能合约实现代币交易的平台。  
**示例**：Uniswap, SushiSwap

### Decimals
**中文**：精度/小数位数  
**定义**：Token 的最小可分割单位，如 USDC decimals=6 表示 1 USDC = 10^6 个最小单位。  
**重要性**：价格计算必须考虑 decimals 差异

## E

### ExEx (Execution Extension)
**中文**：执行扩展  
**定义**：Reth 提供的扩展机制，允许开发者订阅和处理链上数据，无需修改节点核心。  
**优势**：非侵入式、高性能、官方支持

### eth_call
**中文**：以太坊调用  
**定义**：不消耗 Gas 的只读合约调用，用于查询合约状态。  
**用途**：State 验证

## F

### Factory Contract
**中文**：工厂合约  
**定义**：负责创建新 Pool 的合约，如 UniswapV2Factory, UniswapV3Factory。  
**事件**：PoolCreated / PairCreated

### Fee Tier
**中文**：手续费级别  
**定义**：UniswapV3 的手续费等级，如 0.05%, 0.3%, 1%。  
**影响**：决定 Tick Spacing

## G

### Gas
**中文**：燃料费  
**定义**：执行以太坊交易所需支付的费用，单位为 Gwei。  
**相关**：MEV 竞争中，Gas 价格决定交易优先级

## L

### Liquidity
**中文**：流动性  
**定义**：池子中可用于交易的资产数量。  
**V2**：reserve0 + reserve1  
**V3**：集中在某个价格区间的流动性

### LRU (Least Recently Used)
**中文**：最近最少使用  
**定义**：缓存淘汰策略，优先淘汰最久未访问的数据。  
**用途**：State Cache 内存管理

## M

### MEV (Maximal Extractable Value)
**中文**：最大可提取价值  
**定义**：通过重新排序、插入或审查交易来从区块链中提取的额外价值。  
**类型**：Front-running, Sandwich Attack, Arbitrage

### Mempool
**中文**：内存池  
**定义**：存储待确认交易的缓冲区。  
**用途**：Pending Tx 观测

## O

### Onchain State
**中文**：链上状态  
**定义**：已确认区块中的 Pool 状态，是权威数据来源。  
**对比**：Pending State（待确认交易的模拟状态）

### Oracle
**中文**：预言机  
**定义**：为智能合约提供链下数据（如价格）的服务。  
**示例**：Chainlink, Uniswap TWAP

## P

### Pending Transaction
**中文**：待确认交易  
**定义**：已提交但未被打包进区块的交易。  
**用途**：抢先交易分析

### Pool
**中文**：流动性池  
**定义**：存储两种 Token 的智能合约，用于去中心化交易。  
**类型**：UniswapV2 Pair, UniswapV3 Pool

### Protocol
**中文**：协议  
**定义**：DEX 的具体实现，如 UniswapV2, UniswapV3。  
**区别**：不同协议有不同的 AMM 算法和 Storage Layout

## R

### Raw State
**中文**：原始状态  
**定义**：从 Contract Storage 直接读取的原始 bytes 数据。  
**对比**：Business State（经过转换的状态）

### Reorg (Reorganization)
**中文**：重组  
**定义**：区块链的分叉被更长链替代，导致部分区块无效。  
**影响**：需要回滚 State Cache

### Reserve
**中文**：储备量  
**定义**：UniswapV2 Pool 中两种 Token 的数量。  
**公式**：reserve0 × reserve1 = k

### RPC (Remote Procedure Call)
**中文**：远程过程调用  
**定义**：通过网络调用远程服务的方法，如 eth_call, eth_getStorageAt。  
**缺点**：跨进程开销、延迟不可控

## S

### Sandwich Attack
**中文**：三明治攻击  
**定义**：在目标交易前后插入买入和卖出交易，从价格滑点中获利的 MEV 策略。  
**流程**：Front-run → 受害者交易 → Back-run

### Searcher
**中文**：搜索者  
**定义**：在区块链上寻找 MEV 机会的参与者或程序。  
**目标**：最大化利润

### Slot (Storage Slot)
**中文**：存储槽  
**定义**：合约 Storage 的存储单元，每个 Slot 32 bytes。  
**计算**：通过 Keccak256 计算 mapping 的 Slot 位置

### sqrtPriceX96
**中文**：价格平方根（Q64.96 格式）  
**定义**：UniswapV3 的价格表示方式，price = (sqrtPriceX96 / 2^96)^2。  
**优势**：避免浮点运算，保持精度

### State
**中文**：状态  
**定义**：Pool 在某个时刻的快照数据，包含 reserve, price, liquidity 等。  
**类型**：Onchain State, Pending State, Simulated State

### Storage Layout
**中文**：存储布局  
**定义**：合约中状态变量在 Storage 中的排列方式。  
**重要性**：直读 Storage 必须知道准确的 Layout

## T

### Tick
**中文**：刻度  
**定义**：UniswapV3 的价格离散化单位，price = 1.0001^tick。  
**范围**：[-887272, 887272]

### Tick Spacing
**中文**：刻度间隔  
**定义**：相邻可初始化 Tick 之间的间隔，由 Fee Tier 决定。  
**示例**：0.3% fee → tick_spacing = 60

### TVL (Total Value Locked)
**中文**：总锁仓价值  
**定义**：Pool 中锁定的资产总价值，以 USD 计价。  
**计算**：reserve0 × price0 + reserve1 × price1

### Tx Index
**中文**：交易索引  
**定义**：交易在区块中的位置序号，从 0 开始。  
**用途**：精确标识 State 的位置

## V

### Validator
**中文**：验证器  
**定义**：验证 Pool State 正确性和可用性的组件。  
**方法**：快速失败规则 + 完整验证（eth_call）

### Verified State
**中文**：已验证状态  
**定义**：通过验证的 Pool State，确保准确性和可用性。  
**保证**：与链上数据 100% 一致

## 缩写列表

| 缩写 | 全称 | 中文 |
|------|------|------|
| AMM | Automated Market Maker | 自动化做市商 |
| DEX | Decentralized Exchange | 去中心化交易所 |
| ExEx | Execution Extension | 执行扩展 |
| LRU | Least Recently Used | 最近最少使用 |
| MEV | Maximal Extractable Value | 最大可提取价值 |
| POC | Proof of Concept | 概念验证 |
| QPS | Queries Per Second | 每秒查询数 |
| RPC | Remote Procedure Call | 远程过程调用 |
| TVL | Total Value Locked | 总锁仓价值 |
| TWAP | Time-Weighted Average Price | 时间加权平均价格 |

## 数学符号

| 符号 | 含义 |
|------|------|
| x × y = k | UniswapV2 恒定乘积公式 |
| √(x × y) | 几何平均价格 |
| sqrtPriceX96 | V3 价格平方根（Q64.96 定点数） |
| 1.0001^tick | Tick 到价格的转换公式 |
| Δx, Δy | Token 数量的变化量 |

## 性能指标术语

| 术语 | 定义 | 示例 |
|------|------|------|
| P50 | 第 50 百分位延迟（中位数） | 100ms |
| P95 | 第 95 百分位延迟 | 200ms |
| P99 | 第 99 百分位延迟 | 500ms |
| QPS | 每秒查询数 | 1000 QPS |
| Latency | 延迟时间 | 10ms |
| Throughput | 吞吐量 | 100 Pools/s |

## 技术栈术语

| 术语 | 说明 |
|------|------|
| Rust | 系统编程语言，用于实现 Observer ExEx |
| Reth | Rust 实现的以太坊客户端 |
| Geth | Go 实现的以太坊客户端 |
| RocksDB | 嵌入式 KV 数据库，用于持久化 |
| gRPC | 高性能 RPC 框架，用于进程间通信 |
| Mermaid | 文本描述的图表工具 |

## 业务术语

| 术语 | 定义 |
|------|------|
| 套利 | 利用价差获取无风险利润 |
| 滑点 | 实际成交价与预期价的偏离 |
| 流动性挖矿 | 提供流动性获得奖励 |
| 无常损失 | 做市商因价格变化产生的相对损失 |

---

**相关文档**：
- [01-需求分析](./01-requirements.md)
- [03-总体架构](./03-architecture-overview.md)
- [README](./README.md)
