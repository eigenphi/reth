# 错误处理与降级策略

## 文档元信息

| 项目 | 内容 |
|------|------|
| 文档版本 | v1.0 |
| 创建日期 | 2025-12-12 |
| 最后更新 | 2025-12-12 |
| 文档状态 | 草稿 |
| 责任人 | 架构组 |

## 1. 错误分类体系

### 1.1 错误分类表

| 错误类别 | 严重级别 | 恢复策略 | 示例 |
|---------|---------|---------|------|
| **可恢复错误** | 低 | 重试 | 网络超时、临时性 RPC 错误 |
| **数据错误** | 中 | 跳过 | 无效 Pool 地址、解析失败 |
| **性能降级** | 中 | 降级处理 | Storage 读取慢、验证超时 |
| **致命错误** | 高 | 重启 | 内存溢出、Panic |
| **配置错误** | 高 | 人工介入 | 连接串错误、权限问题 |

### 1.2 错误代码设计

```mermaid
classDiagram
    class ObserverError {
        <<enumeration>>
        +StorageReadError
        +ValidationError
        +NetworkError
        +ConfigError
        +InternalError
    }
    
    class StorageReadError {
        +SlotNotFound
        +ParseError
        +AccessDenied
    }
    
    class ValidationError {
        +DataInconsistent
        +VerificationFailed
        +TimeoutError
    }
    
    ObserverError --> StorageReadError
    ObserverError --> ValidationError
```

## 2. 错误处理流程

### 2.1 统一错误处理流程

```mermaid
flowchart TD
    A[发生错误] --> B{错误类型?}
    
    B -->|可恢复| C[重试机制]
    B -->|数据错误| D[跳过并记录]
    B -->|性能降级| E[启用降级方案]
    B -->|致命错误| F[触发重启]
    B -->|配置错误| G[告警并暂停]
    
    C --> H{重试成功?}
    H -->|是| I[继续处理]
    H -->|否| J{达到重试上限?}
    J -->|是| D
    J -->|否| C
    
    D --> K[记录到错误日志]
    E --> L[使用备用方案]
    F --> M[优雅关闭<br/>保存状态]
    G --> N[等待人工修复]
    
    K --> I
    L --> I
    M --> O[自动重启]
    O --> P[从 checkpoint 恢复]
    
    style F fill:#ffe0e0
    style G fill:#ffe0e0
    style I fill:#c8e6c9
    style P fill:#fff4e1
```

### 2.2 重试策略

**指数退避重试**：

| 重试次数 | 等待时间 | 累计时间 |
|---------|---------|---------|
| 1 | 100ms | 100ms |
| 2 | 500ms | 600ms |
| 3 | 2s | 2.6s |
| 4 | 5s | 7.6s |
| 5 | 10s | 17.6s |

**重试条件**：

| 错误类型 | 是否重试 | 最大次数 |
|---------|---------|---------|
| 网络超时 | ✓ | 5 |
| RPC 限流 | ✓ | 3 |
| Storage 读取失败 | ✓ | 3 |
| 解析错误 | ✗ | 0 |
| 验证失败 | ✓ | 1 |

## 3. 降级策略

### 3.1 性能降级场景

**场景 1：Storage 直读延迟高**

```mermaid
flowchart TD
    A[检测延迟 > 阈值] --> B{连续 N 次?}
    B -->|是| C[触发降级]
    B -->|否| D[继续监控]
    
    C --> E[切换到 Batch RPC]
    E --> F[记录降级事件]
    F --> G[持续监控]
    
    G --> H{延迟恢复正常?}
    H -->|是| I[切回 Storage 直读]
    H -->|否| G
    
    style C fill:#ffd700
    style E fill:#fff4e1
    style I fill:#c8e6c9
```

**降级触发条件**：

| 场景 | 触发条件 | 降级方案 |
|------|---------|---------|
| Storage 读取慢 | P99 > 10ms，持续 1 分钟 | 降级到 Batch RPC |
| 验证超时 | P99 > 1s，持续 5 分钟 | 减少验证采样率 |
| 内存压力 | 使用率 > 90% | 触发 LRU 清理 |
| ExEx 不稳定 | 崩溃 > 3 次/小时 | 降级到 Geth RPC |

### 3.2 ExEx 处理速度跟不上出块速度

**问题描述**：ExEx 处理一个 Block 耗时 > 12s（出块间隔）

**检测方法**：

```mermaid
flowchart LR
    A[接收 Block N] --> B[记录时间 T1]
    B --> C[处理 Block N]
    C --> D[完成时间 T2]
    D --> E{T2 - T1 > 12s?}
    
    E -->|是| F[延迟累积]
    E -->|否| G[正常]
    
    F --> H{延迟 > 60s?}
    H -->|是| I[触发告警]
    
    style F fill:#fff4e1
    style I fill:#ffe0e0
    style G fill:#c8e6c9
```

**应对策略**：

| 策略 | 说明 | 效果 |
|------|------|------|
| **优先级队列** | 优先处理最新 Block，跳过部分历史 | 保证实时性 |
| **批量处理** | 合并处理多个 Block | 提升吞吐量 |
| **异步验证** | 验证不阻塞主流程 | 减少延迟 |
| **降级** | 暂停 Pending Tx 模拟 | 释放资源 |
| **扩容** | 增加机器资源 | 根本解决 |

**优先级队列实现**：

```mermaid
flowchart TD
    A[新区块队列] --> B{队列积压 > 10?}
    B -->|否| C[正常处理]
    B -->|是| D[清空旧区块]
    
    D --> E[仅保留最新 5 个]
    E --> F[标记跳过的 Block]
    F --> G[后台补齐数据]
    
    C --> H[处理完成]
    G --> H
    
    style D fill:#ffd700
    style G fill:#fff4e1
    style H fill:#c8e6c9
```

## 4. 故障恢复方案

### 4.1 故障类型和恢复

| 故障类型 | 恢复方法 | RTO | RPO |
|---------|---------|-----|-----|
| **进程崩溃** | 自动重启 + Checkpoint 恢复 | < 30s | 0 |
| **数据损坏** | 从持久化层恢复 | < 5min | 最后一次同步 |
| **节点故障** | 切换到备节点 | < 10s | 0 |
| **网络分区** | 等待网络恢复 | 取决于网络 | 0 |

### 4.2 Checkpoint 机制

```mermaid
sequenceDiagram
    participant E as ExEx
    participant C as Checkpoint
    participant R as RocksDB
    
    loop 每 100 Blocks
        E->>C: 保存 Checkpoint
        C->>R: 持久化<br/>last_processed_block
        C->>R: 持久化<br/>PoolLatestIndex
    end
    
    Note over E: 崩溃

    E->>C: 读取 Checkpoint
    C->>R: 加载数据
    R-->>E: last_processed_block = N
    
    E->>E: 从 Block N+1 继续
```

**Checkpoint 内容**：

| 数据项 | 说明 |
|-------|------|
| last_processed_block | 最后处理的区块号 |
| PoolLatestIndex | 最新 State 索引 |
| 统计信息 | 处理的 Pool 数量等 |

### 4.3 数据一致性恢复

**恢复流程**：

```mermaid
flowchart TD
    A[启动恢复] --> B[读取 Checkpoint]
    B --> C[加载 PoolLatestIndex]
    C --> D[读取 last_block]
    
    D --> E{与 Reth 对比}
    E -->|一致| F[直接继续]
    E -->|不一致| G[检测 Reorg]
    
    G --> H[回滚到分叉点]
    H --> I[重放正确分支]
    I --> F
    
    F --> J[恢复完成]
    
    style G fill:#fff4e1
    style H fill:#ffd700
    style J fill:#c8e6c9
```

## 5. 熔断机制

### 5.1 熔断触发条件

| 指标 | 阈值 | 熔断动作 |
|------|------|---------|
| 错误率 | > 50% | 暂停处理，等待人工介入 |
| 验证失败率 | > 90% | 停止验证，仅做 Storage 读取 |
| 内存使用 | > 95% | 触发紧急清理 |
| CPU 使用 | > 95% 持续 5 分钟 | 限流 |

### 5.2 熔断状态机

```mermaid
stateDiagram-v2
    [*] --> 正常
    正常 --> 降级: 错误率 30-50%
    降级 --> 正常: 错误率 < 20%
    降级 --> 熔断: 错误率 > 50%
    
    熔断 --> 半开: 人工恢复或超时
    半开 --> 正常: 测试通过
    半开 --> 熔断: 测试失败
    
    note right of 熔断: 暂停处理<br/>记录日志<br/>发送告警
    note right of 半开: 小流量测试<br/>验证恢复
```

### 5.3 熔断恢复

**自动恢复条件**：
- 错误率降低到阈值以下
- 资源使用率恢复正常
- 连续 N 次检查通过

**人工介入场景**：
- 配置错误
- 外部依赖不可用（Reth 节点）
- 数据损坏

## 6. 监控和告警

### 6.1 错误监控指标

| 指标 | 告警级别 | 阈值 |
|------|---------|------|
| 错误率 | 警告 | > 10% |
| 错误率 | 严重 | > 30% |
| 致命错误 | 严重 | > 0 |
| 降级事件 | 警告 | 发生时 |
| 熔断事件 | 严重 | 发生时 |

### 6.2 告警通知

**通知渠道**：
- Slack 告警频道
- 邮件
- PagerDuty（严重级别）

**告警内容**：
- 错误类型和数量
- 影响范围
- 建议的处理步骤

## 7. 日志记录

### 7.1 日志级别

| 级别 | 用途 | 示例 |
|------|------|------|
| ERROR | 错误事件 | Storage 读取失败 |
| WARN | 警告事件 | 验证失败率高 |
| INFO | 关键信息 | Block 处理完成 |
| DEBUG | 调试信息 | State 详细数据 |
| TRACE | 详细追踪 | 函数调用链 |

### 7.2 错误日志格式

**结构化日志示例**（JSON 格式）：
```json
{
  "timestamp": "2025-12-12T10:30:00Z",
  "level": "ERROR",
  "component": "StateTracker",
  "error_code": "STORAGE_READ_FAILED",
  "message": "Failed to read storage slot",
  "context": {
    "pool_id": "0x1234...",
    "block_number": 20000000,
    "slot": 8,
    "retry_count": 3
  }
}
```

---

**相关文档**：
- [14-监控与运维](./14-monitoring.md)
- [03-总体架构](./03-architecture-overview.md)
