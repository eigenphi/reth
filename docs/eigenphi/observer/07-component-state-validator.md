# 组件设计：State Validator

## 文档元信息

| 项目 | 内容 |
|------|------|
| 组件名称 | State Validator |
| 文档版本 | v1.0 |
| 创建日期 | 2025-12-12 |
| 最后更新 | 2025-12-12 |
| 文档状态 | 草稿 |
| 责任人 | 架构组 |

## 1. 组件概述

### 1.1 职责定义

State Validator 负责验证 Pool State 的**正确性**和**可用性**，确保只有通过验证的 State 才能进入决策系统。

**核心职责**：

| 职责 | 说明 | 优先级 |
|------|------|--------|
| 快速失败规则 | 使用轻量级规则快速过滤不可用 Pool | P0 |
| 完整验证 | 通过 eth_call 验证 State 正确性 | P0 |
| 并行验证 | 并发验证多个 Pool State | P0 |
| 失败处理 | 记录验证失败原因和统计 | P1 |
| 阈值配置 | 支持动态调整验证阈值 | P1 |

### 1.2 输入输出

```mermaid
graph LR
    subgraph 输入
        A[BusinessPoolState<br/>待验证]
        B[验证规则配置]
        C[历史验证统计]
    end
    
    subgraph State Validator
        D[快速失败规则]
        E[完整验证器]
        F[结果聚合器]
    end
    
    subgraph 输出
        G[已验证 State]
        H[验证失败 State]
        I[验证统计报告]
    end
    
    A --> D
    B --> D
    C --> D
    
    D --> E
    E --> F
    
    F --> G
    F --> H
    F --> I
    
    style D fill:#ffd700
    style E fill:#e1f5ff
    style G fill:#c8e6c9
    style H fill:#ffe0e0
```

### 1.3 接口定义

| 接口名称 | 输入参数 | 输出 | 用途 |
|---------|---------|------|------|
| `ValidateState` | business_state | ValidationResult | 验证单个 Pool State |
| `ValidateBatch` | Vec\<business_state\> | Vec\<ValidationResult\> | 批量验证 |
| `ApplyFastFailRules` | business_state | Result\<(), FailReason\> | 快速失败检查 |
| `FullValidation` | business_state | Result\<(), ValidationError\> | 完整验证 |
| `GetValidationStats` | time_range | ValidationStats | 获取验证统计 |

## 2. 验证流程设计

### 2.1 完整验证流程

```mermaid
flowchart TD
    A[Pool State] --> B[快速失败规则]
    
    B --> C{通过?}
    C -->|否| D[失败: 快速过滤<br/>成本: 1.8ms]
    C -->|是| E[完整验证]
    
    E --> F[并行 eth_call]
    F --> G{结果一致?}
    
    G -->|是| H[验证通过<br/>加入缓存]
    G -->|否| I{误差 < 阈值?}
    
    I -->|是| J[通过（警告）<br/>记录差异]
    I -->|否| K[失败: 数据不一致]
    
    D --> L[记录失败原因]
    K --> L
    
    L --> M[更新统计]
    
    style A fill:#e1f5ff
    style B fill:#ffd700
    style E fill:#fff4e1
    style H fill:#c8e6c9
    style J fill:#c8e6c9
    style D fill:#ffe0e0
    style K fill:#ffe0e0
```

### 2.2 验证阶段分解

| 阶段 | 成本 | 通过率 | 累计通过率 | 说明 |
|------|------|--------|-----------|------|
| **阶段 0：输入检查** | 0.1ms | 99% | 99% | 检查数据格式 |
| **阶段 1：黑名单** | 0.1ms | 85% | 84.15% | 检查黑名单 Pool |
| **阶段 2：TVL 阈值** | 0.5ms | 65% | 54.7% | 检查流动性 |
| **阶段 3：价格异常** | 1ms | 95% | 51.97% | 检查价格合理性 |
| **阶段 4：完整验证** | 50ms | 80% | **41.58%** | eth_call 验证 |

**总成本分析**（100 Pools）：

```
无快速失败：100 × 50ms = 5000ms
有快速失败：
  - 快速失败过滤：100 × 1.8ms = 180ms
  - 完整验证：52 × 50ms = 2600ms
  - 总计：2780ms
节省：44.4%
```

## 3. 快速失败规则设计

### 3.1 规则 1：黑名单检查

**目标**：过滤已知不可用的 Pool

```mermaid
flowchart LR
    A[Pool Address] --> B{在黑名单中?}
    B -->|是| C[快速失败<br/>原因: 黑名单]
    B -->|否| D[继续验证]
    
    style A fill:#e1f5ff
    style C fill:#ffe0e0
    style D fill:#c8e6c9
```

**黑名单来源**：

| 来源 | 示例 | 添加方式 |
|------|------|---------|
| 已暂停 Pool | Emergency Pause | 检测到 Paused 事件 |
| 攻击 Pool | 闪电贷攻击 | 人工审核后添加 |
| 异常 Pool | 持续验证失败 | 自动添加（失败 > 10 次） |
| 测试 Pool | Testnet Pool | 配置文件 |

**数据结构**：

```mermaid
classDiagram
    class Blacklist {
        +HashSet~Address~ permanent
        +HashMap~Address, Expiry~ temporary
        +add_permanent(address, reason)
        +add_temporary(address, duration, reason)
        +is_blacklisted(address) bool
    }
    
    note for Blacklist "永久黑名单：人工审核<br/>临时黑名单：自动添加，过期移除"
```

### 3.2 规则 2：TVL 阈值检查

**目标**：过滤流动性不足的 Pool（35% 失败率）

```mermaid
flowchart TD
    A[Pool State] --> B[计算 TVL<br/>tvl = reserve0 × price0 + reserve1 × price1]
    
    B --> C{TVL >= 阈值?}
    C -->|否| D[快速失败<br/>原因: 流动性不足]
    C -->|是| E[继续验证]
    
    style A fill:#e1f5ff
    style D fill:#ffe0e0
    style E fill:#c8e6c9
```

**阈值配置表**：

| Pool 类型 | TVL 阈值 | 说明 |
|----------|---------|------|
| 主流币对 (ETH/USDC) | 1000 USD | 确保深度足够 |
| 长尾币对 | 5000 USD | 避免高滑点 |
| 稳定币对 | 500 USD | 价格稳定，阈值可降低 |

**TVL 计算方法**：

| Protocol | 计算方法 | 数据来源 |
|----------|---------|---------|
| UniswapV2 | reserve0 × price0 + reserve1 × price1 | Price 来自 Oracle |
| UniswapV3 | liquidity × sqrtPrice | 使用 V3 流动性公式 |

### 3.3 规则 3：价格异常检测

**目标**：过滤价格异常的 Pool（5% 失败率）

```mermaid
flowchart TD
    A[Pool Price] --> B[获取 Oracle 价格]
    B --> C[计算偏离度<br/>deviation = abs(price - oracle_price) / oracle_price]
    
    C --> D{deviation < 阈值?}
    D -->|否| E[快速失败<br/>原因: 价格异常]
    D -->|是| F[继续验证]
    
    style A fill:#e1f5ff
    style E fill:#ffe0e0
    style F fill:#c8e6c9
```

**偏离度阈值**：

| Token Pair 类型 | 阈值 | 说明 |
|----------------|------|------|
| 主流币对 (ETH/USDC) | 10% | 价格稳定 |
| 长尾币对 | 50% | 价格波动大 |
| 稳定币对 (USDC/DAI) | 2% | 锚定 1:1 |

**Oracle 来源优先级**：

| 优先级 | Oracle | 延迟 | 可靠性 |
|--------|--------|------|--------|
| P0 | Chainlink | 10ms | 高 |
| P1 | Uniswap TWAP | 5ms | 中 |
| P2 | 其他 DEX 平均价 | 20ms | 低 |

### 3.4 规则 4：Protocol 白名单

**目标**：过滤非标准协议（3% 失败率）

```mermaid
flowchart LR
    A[Pool Protocol] --> B{在白名单中?}
    B -->|否| C[快速失败<br/>原因: 未知协议]
    B -->|是| D[继续验证]
    
    style A fill:#e1f5ff
    style C fill:#ffe0e0
    style D fill:#c8e6c9
```

**白名单配置**：

| Protocol | 版本 | 状态 |
|----------|------|------|
| UniswapV2 | 所有版本 | 启用 |
| UniswapV3 | 所有版本 | 启用 |
| SushiSwap | v1, v2 | 启用 |
| Curve | v1 | 计划支持 |

### 3.5 快速失败规则汇总

```mermaid
flowchart TD
    A[Pool State] --> B[规则 1:<br/>黑名单检查<br/>成本: 0.1ms]
    B --> C{通过?}
    C -->|否| Z1[失败: 黑名单<br/>过滤: 15%]
    
    C -->|是| D[规则 2:<br/>TVL 阈值<br/>成本: 0.5ms]
    D --> E{通过?}
    E -->|否| Z2[失败: 流动性<br/>过滤: 35%]
    
    E -->|是| F[规则 3:<br/>价格异常<br/>成本: 1ms]
    F --> G{通过?}
    G -->|否| Z3[失败: 价格<br/>过滤: 5%]
    
    G -->|是| H[规则 4:<br/>Protocol 白名单<br/>成本: 0.2ms]
    H --> I{通过?}
    I -->|否| Z4[失败: 协议<br/>过滤: 3%]
    
    I -->|是| J[进入完整验证<br/>通过率: ~42%]
    
    style A fill:#e1f5ff
    style Z1 fill:#ffe0e0
    style Z2 fill:#ffe0e0
    style Z3 fill:#ffe0e0
    style Z4 fill:#ffe0e0
    style J fill:#c8e6c9
```

## 4. 完整验证策略

### 4.1 验证方法对比

| 方法 | 延迟 | 准确性 | 适用场景 |
|------|------|--------|---------|
| **eth_call getReserves** | 50-150ms | 100% | UniswapV2 验证 |
| **eth_call slot0** | 50-150ms | 100% | UniswapV3 验证 |
| **模拟 Swap** | 100-300ms | 100% | 完整可用性验证 |
| **对比多个节点** | 100-500ms | 99.99% | 检测节点分叉 |

### 4.2 UniswapV2 完整验证

```mermaid
sequenceDiagram
    participant V as Validator
    participant R as Geth RPC
    participant C as 合约
    
    V->>R: eth_call<br/>getReserves()
    R->>C: 执行调用
    C-->>R: (reserve0, reserve1, timestamp)
    R-->>V: 返回结果
    
    V->>V: 对比 Storage 读取结果
    
    alt 一致
        V->>V: 验证通过
    else 不一致
        V->>V: 计算误差
        alt 误差 < 0.01%
            V->>V: 通过（警告）
        else 误差 >= 0.01%
            V->>V: 验证失败
        end
    end
```

### 4.3 UniswapV3 完整验证

```mermaid
sequenceDiagram
    participant V as Validator
    participant R as Geth RPC
    participant C as 合约
    
    par 并行调用
        V->>R: eth_call slot0()
        V->>R: eth_call liquidity()
    end
    
    R->>C: 执行 slot0()
    C-->>R: (sqrtPriceX96, tick, ...)
    
    R->>C: 执行 liquidity()
    C-->>R: liquidity
    
    R-->>V: 返回结果
    
    V->>V: 对比 Storage 读取结果
    
    V->>V: 验证 sqrtPrice ↔ tick 一致性
    
    alt 全部一致
        V->>V: 验证通过
    else 有不一致
        V->>V: 验证失败
    end
```

### 4.4 模拟 Swap 验证（可选）

**目标**：验证 Pool 确实可以进行交易

```mermaid
flowchart TD
    A[Pool State] --> B[构造模拟 Swap<br/>1 ETH → ? USDC]
    B --> C[eth_call<br/>router.getAmountsOut]
    
    C --> D{调用成功?}
    D -->|否| E[Pool 不可用<br/>可能暂停或限制]
    D -->|是| F{输出量合理?}
    
    F -->|否| G[Pool 异常<br/>价格计算错误]
    F -->|是| H[完全可用]
    
    style A fill:#e1f5ff
    style E fill:#ffe0e0
    style G fill:#ffe0e0
    style H fill:#c8e6c9
```

**使用场景**：

| 场景 | 是否启用模拟 Swap | 原因 |
|------|----------------|------|
| 新 Pool 首次验证 | 是 | 确保完全可用 |
| 定期抽查 | 是（10% 抽样） | 监控 Pool 健康度 |
| 常规验证 | 否 | 成本太高（100-300ms） |

## 5. 验证性能优化策略

### 5.1 并行验证

```mermaid
flowchart TB
    A[40 Pools<br/>通过快速失败] --> B[分成 4 批<br/>每批 10 Pools]
    
    B --> C1[批次 1]
    B --> C2[批次 2]
    B --> C3[批次 3]
    B --> C4[批次 4]
    
    C1 --> D[并行执行<br/>eth_call]
    C2 --> D
    C3 --> D
    C4 --> D
    
    D --> E[汇总结果<br/>总延迟 ~100ms]
    
    style A fill:#e1f5ff
    style D fill:#fff4e1
    style E fill:#c8e6c9
```

**并行度配置**：

| Pool 数量 | 并行度 | 单批大小 | 预计延迟 |
|----------|--------|---------|---------|
| 10 | 10 | 1 | 50ms |
| 40 | 4 | 10 | 100ms |
| 100 | 10 | 10 | 150ms |

### 5.2 批量验证优化

**RPC 批量调用**：

```mermaid
sequenceDiagram
    participant V as Validator
    participant R as RPC Server
    
    V->>R: BatchRequest[<br/>  call1: getReserves(pool1),<br/>  call2: getReserves(pool2),<br/>  ...<br/>]
    
    Note over R: 服务器端并行执行
    
    R-->>V: BatchResponse[<br/>  result1,<br/>  result2,<br/>  ...<br/>]
    
    Note over V: 减少网络往返<br/>延迟降低 50%
```

### 5.3 缓存验证结果

**验证结果缓存策略**：

| 缓存项 | TTL | 用途 |
|-------|-----|------|
| 黑名单 Pool 验证结果 | 永久 | 避免重复验证已知无效 Pool |
| 正常 Pool 验证结果 | 1 小时 | 减少重复验证 |
| Oracle 价格 | 10 秒 | 价格异常检测 |

**缓存收益**：

```
无缓存：每次验证 50ms
有缓存（命中率 80%）：
  - 80% × 0.1ms（缓存命中）= 0.08ms
  - 20% × 50ms（缓存未命中）= 10ms
  - 平均：10.08ms
性能提升：79.8%
```

### 5.4 异步非阻塞验证

```mermaid
flowchart LR
    A[Pool State] --> B[快速失败]
    B --> C{通过?}
    
    C -->|否| D[立即返回失败]
    C -->|是| E[提交验证任务]
    
    E --> F[继续处理下一个 Pool]
    E -.异步.-> G[完整验证]
    
    G --> H[验证完成<br/>回调通知]
    
    style A fill:#e1f5ff
    style D fill:#ffe0e0
    style E fill:#fff4e1
    style F fill:#c8e6c9
    style H fill:#c8e6c9
```

**收益**：
- 主流程不被验证阻塞
- 吞吐量提升 5x
- 延迟稳定性提升

## 6. 验证失败处理流程

### 6.1 失败分类

```mermaid
flowchart TD
    A[验证失败] --> B{失败类型?}
    
    B -->|快速失败| C[过滤类失败]
    B -->|完整验证失败| D[数据不一致]
    B -->|RPC 错误| E[临时故障]
    
    C --> F[记录原因<br/>不重试]
    D --> G[记录差异<br/>可选重试]
    E --> H[重试 3 次]
    
    H --> I{重试成功?}
    I -->|是| J[验证通过]
    I -->|否| K[标记为不可用]
    
    style A fill:#ffe0e0
    style F fill:#fff4e1
    style G fill:#fff4e1
    style K fill:#ffe0e0
    style J fill:#c8e6c9
```

### 6.2 失败原因统计

**失败原因分布表**（基于历史数据）：

| 失败原因 | 占比 | 是否重试 | 处理策略 |
|---------|------|---------|---------|
| 流动性不足 | 35% | 否 | 快速失败，记录日志 |
| Pool 暂停 | 15% | 否 | 加入黑名单 |
| 价格异常 | 5% | 是 | 重试 3 次 |
| RPC 超时 | 3% | 是 | 重试 3 次，降级方案 |
| 数据不一致 | 2% | 是 | 重试 3 次，记录差异 |

### 6.3 告警规则

| 告警条件 | 级别 | 处理 |
|---------|------|------|
| 验证失败率 > 70% | 警告 | 检查规则配置 |
| 验证失败率 > 90% | 严重 | 检查系统故障 |
| RPC 超时率 > 10% | 警告 | 检查节点状态 |
| 数据不一致率 > 5% | 严重 | 检查 Storage 读取逻辑 |

## 7. 验证阈值配置策略

### 7.1 动态阈值调整

```mermaid
stateDiagram-v2
    [*] --> 初始阈值
    初始阈值 --> 收集统计: 运行 1 周
    收集统计 --> 分析验证率: 验证通过率
    
    分析验证率 --> 调高阈值: < 30%（过于宽松）
    分析验证率 --> 调低阈值: > 60%（过于严格）
    分析验证率 --> 保持: 30-60%（合理）
    
    调高阈值 --> 收集统计
    调低阈值 --> 收集统计
    保持 --> 收集统计
    
    note right of 分析验证率: 目标验证通过率：40%<br/>考虑 60% 本身不可用
```

### 7.2 阈值配置表

**初始推荐阈值**：

| 参数 | 初始值 | 调整范围 | 说明 |
|------|--------|---------|------|
| TVL 最小值 | 1000 USD | [500, 5000] | 主流币对 |
| 价格偏离度 | 10% | [5%, 50%] | 主流币对 |
| 验证超时 | 5s | [1s, 30s] | eth_call 超时 |
| 误差容忍度 | 0.01% | [0.001%, 0.1%] | 数据对比阈值 |

### 7.3 分 Token Pair 配置

```mermaid
classDiagram
    class ValidationConfig {
        +default_config: Config
        +token_pair_configs: HashMap~(Token0, Token1), Config~
        +get_config(token0, token1) Config
    }
    
    class Config {
        +tvl_threshold: Decimal
        +price_deviation_threshold: Percentage
        +validation_timeout: Duration
        +error_tolerance: Percentage
    }
    
    ValidationConfig --> Config
    
    note for ValidationConfig "支持分 Token Pair 配置<br/>例如稳定币对使用更严格的阈值"
```

## 8. 监控和统计

### 8.1 监控指标

| 指标 | 说明 | 告警阈值 |
|------|------|---------|
| validation_total | 总验证次数 | - |
| validation_pass_rate | 验证通过率 | < 30% 或 > 60% |
| fast_fail_filter_rate | 快速失败过滤比例 | < 50% |
| validation_latency | 验证延迟 | P99 > 200ms |
| rpc_timeout_rate | RPC 超时比例 | > 5% |
| data_inconsistency_rate | 数据不一致比例 | > 1% |

### 8.2 验证统计报告

**统计维度**：

| 维度 | 统计内容 | 用途 |
|------|---------|------|
| 按时间 | 每小时验证通过/失败数 | 趋势分析 |
| 按 Protocol | V2/V3 验证通过率 | Protocol 对比 |
| 按失败原因 | 各类失败原因分布 | 优化快速失败规则 |
| 按 Pool | 每个 Pool 的验证历史 | 识别问题 Pool |

### 8.3 审计日志

| 日志类型 | 内容 | 保留时长 |
|---------|------|---------|
| 验证通过 | Pool ID、Block、验证时长 | 7 天 |
| 验证失败 | Pool ID、Block、失败原因、详细数据 | 30 天 |
| 数据不一致 | Pool ID、Storage 结果、RPC 结果、误差 | 永久 |

---

**下一步**：阅读 [08-State Cache](./08-component-state-cache.md) 了解缓存设计
