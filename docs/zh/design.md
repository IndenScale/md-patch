# 设计文档

深入了解 `md-patch` 的设计哲学和实现细节。

## 核心设计目标

### 1. 为 AI Agent 设计

传统文本编辑工具基于行号，这对 AI 来说过于脆弱。`md-patch` 使用**语义寻址**：

```text
❌ 脆弱：在第 42 行插入
✓ 鲁棒：在 "## API" 章节后追加
```

### 2. 安全第一

破坏性操作需要显式确认：

- **Fingerprint**：验证你修改的是预期的内容
- **Force 标志**：确认你接受变更风险
- **自动备份**：变更前自动保存副本

### 3. 幂等性

相同的操作执行多次应该产生相同的结果：

```bash
# 第一次执行
mdp patch -f doc.md -H "## 功能" --op append -c "- X" --force
# → 内容被追加

# 第二次执行（相同命令）
mdp patch -f doc.md -H "## 功能" --op append -c "- X" --force
# → 检测到已存在，无操作（no-op）
```

## 架构概览

```text
┌─────────────┐     ┌─────────────┐     ┌─────────────┐
│   CLI 输入   │────▶│   解析器    │────▶│   定位器    │
└─────────────┘     └─────────────┘     └──────┬──────┘
                                                │
                                                ▼
┌─────────────┐     ┌─────────────┐     ┌─────────────┐
│   输出格式化 │◀────│   执行引擎   │◀────│   验证器    │
└─────────────┘     └─────────────┘     └─────────────┘
```

### 组件职责

| 组件 | 职责 |
|------|------|
| CLI 输入 | 参数解析、验证 |
| 解析器 | Markdown 解析、块识别 |
| 定位器 | 标题路径解析、块索引定位 |
| 验证器 | Fingerprint 匹配、安全检查 |
| 执行引擎 | 应用补丁、原子写入 |
| 输出格式化 | 生成 diff/JSON/text 输出 |

## 寻址模型详解

### 文档结构

Markdown 文档被视为**章节树**：

```markdown
# 文档标题                    ← 层级 1

## 第一章                     ← 层级 2

内容块 1

内容块 2

### 第一节                    ← 层级 3

内容块 3

## 第二章                     ← 层级 2

内容块 4
```

### 标题路径解析

标题路径是从根到目标的路径：

```rust
// 输入
heading: "# 文档标题 ## 第一章"

// 解析为
[
    Heading { level: 1, text: "文档标题" },
    Heading { level: 2, text: "第一章" }
]
```

### 块索引

每个章节下的内容被划分为**块**：

```markdown
## 章节

[块 0] 第一段文字...

[块 1] 第二段文字...

[块 2] - 列表项 1
       - 列表项 2
       - 列表项 3

[块 3] ```code
       fn main() {}
       ```
```

块边界由以下元素定义：

- 段落之间的空行
- 列表、代码块等独立元素
- 标题（作为章节边界）

## 安全机制

### Fingerprint 验证

Fingerprint 是内容的**正则表达式验证**：

```bash
# 只有内容匹配 "旧版本.*" 时才替换
mdp patch -f doc.md -H "## API" --op replace \
  -c "新版本" \
  --fingerprint "旧版本.*"
```

验证流程：

```text
1. 定位目标块
2. 提取当前内容
3. 应用 fingerprint 正则
4. 匹配？→ 继续操作
   不匹配？→ 退出码 3
```

### Force 标志

Force 是**权限确认**，不是验证：

| 场景 | 结果 |
|------|------|
| 有 fingerprint，匹配 | ✓ 成功 |
| 有 fingerprint，不匹配 | ✗ 失败（force 无效） |
| 无 fingerprint，有 force | ✓ 成功（接受风险） |
| 无 fingerprint，无 force | ✗ 失败（需要 force） |

### 备份策略

原子写入流程：

```text
1. 读取原始文件 → memory
2. 创建备份文件 → file.bak
3. 写入临时文件 → file.tmp.XXXX
4. 原子重命名   → file.tmp.XXXX → file
```

如果第 3 步失败，原始文件不受影响。
如果第 4 步失败，备份文件可用于恢复。

## 幂等性实现

### Append 幂等性

在追加前检测内容是否已存在：

```rust
fn apply_append(content: &str, block: &Block, new_content: &str) -> Result<String> {
    // 幂等性检查
    if content[block.start..block.end].contains(new_content) {
        return Ok(content.to_string()); // 无操作
    }
    // ... 执行追加
}
```

### Replace 幂等性

替换内容相同时不写入：

```rust
fn apply_replace(content: &str, block: &Block, new_content: &str) -> Result<String> {
    let current = &content[block.start..block.end];
    if current.trim() == new_content.trim() {
        return Ok(content.to_string()); // 无操作
    }
    // ... 执行替换
}
```

### Delete 幂等性

目标块不存在时无操作。

## 输出格式设计

### Diff 格式

使用统一的 unified diff 格式：

```diff
--- a/file.md
+++ b/file.md
@@ -10,3 +10,7 @@
 ## 章节
 
+新内容 1
+
+新内容 2
+
 原有内容
```

### JSON 格式

结构化输出用于程序化集成：

```json
{
  "success": true,
  "applied": true,
  "is_noop": false,
  "changes": [...]
}
```

### Agent 集成模式

```bash
# 1. Agent 计划变更
plan_result=$(mdp plan patches.yaml --format json)

# 2. 解析计划结果，决定操作
if echo "$plan_result" | jq -e '.success' > /dev/null; then
    # 3. 应用变更
    apply_result=$(mdp apply patches.yaml --force --format json)
    
    # 4. 验证结果
    if echo "$apply_result" | jq -e '.is_noop' > /dev/null; then
        echo "无需变更"
    else
        echo "变更已应用"
    fi
fi
```

## 设计权衡

### 基于偏移 vs AST

| 方案 | 优点 | 缺点 |
|------|------|------|
| **基于偏移**（当前） | 保留原始格式 | 对复杂结构支持有限 |
| AST | 结构正确性 | 丢失原始格式 |

`md-patch` 选择偏移方案，因为对于 Agent 工具，保留用户原始格式比强制结构正确性更重要。

### 指纹 vs 哈希

| 方案 | 优点 | 缺点 |
|------|------|------|
| **正则指纹**（当前） | 灵活，容忍小变化 | 可能误匹配 |
| 内容哈希 | 精确匹配 | 任何变化都失败 |

选择正则指纹以支持版本号变化等场景。

### 单文件 vs 批量

`patch` 命令用于交互式使用，`apply` 用于批处理。
分离设计使每个命令的参数更简单。

## 未来扩展

### 可能的增强

1. **条件操作**：`when: content matches "..."`
2. **模板支持**：内容中支持变量替换
3. **插件系统**：自定义操作类型
4. **并发批量操作**：并行处理多个文件

### 兼容性考虑

- YAML 格式版本控制
- 命令行接口稳定性
- JSON 输出字段向后兼容
