# md-patch

声明式、幂等的 Markdown 块级补丁工具。

## 简介

`mdp`（md-patch）提供了一种结构化的方式来修改 Markdown 文件，
通过**语义寻址**（路径 + 标题 + 块索引）而非脆弱的行号。
专为 AI Agent 设计，支持：

- **声明式操作**：append（追加）、replace（替换）、delete（删除）
- **安全第一设计**：破坏性操作使用指纹正则验证
- **原子批量操作**：通过 YAML 配置多文件补丁
- **非破坏性工作流**：应用（`apply`）前预览（`plan`）
- **基于偏移的编辑**：保留原始格式（无 AST 往返）
- **自动备份**：破坏性操作前创建 `.bak` 文件
- **幂等性**：重复操作被检测并报告为无操作（no-op）

## 安装

```bash
# 从源码安装
git clone https://github.com/IndenScale/md-patch.git
cd md-patch
cargo build --release

# 二进制文件位于 target/release/mdp
cp target/release/mdp ~/.local/bin/
```

## 快速开始

### 单补丁操作

```bash
# 在特定块后追加内容（干运行预览）
mdp patch -f doc.md -H "## 快速开始" -i 0 --op append -c "新内容"

# 带指纹验证的替换操作
mdp patch -f doc.md -H "# API" -i 1 --op replace \
  -c "新描述" \
  --fingerprint "旧.*模式" \
  --force

# 删除块
mdp patch -f doc.md -H "## 已废弃" -i 0 --op delete \
  --fingerprint "deprecated.*section" \
  --force

# 跳过备份创建
mdp patch -f doc.md -H "## 章节" --op replace -c "新内容" --force --no-backup
```

### 通过 YAML 批量操作

创建 `patch.yaml`：

```yaml
operations:
  - file: docs/api.md
    heading:
      - "# API 参考"
    index: 0
    operation: append
    content: "\n新的接口文档。"

  - file: docs/auth.md
    heading:
      - "# 认证"
      - "## OAuth2"
    index: 0
    operation: replace
    content: "所有应用使用 PKCE 流程。"
    fingerprint: "implicit.*flow"
```

执行：

```bash
# 预览变更
mdp plan patch.yaml

# 应用变更
mdp apply patch.yaml --force

# 应用而不创建备份
mdp apply patch.yaml --force --no-backup
```

## 寻址模型

```text
file → heading_path → block_index
       (如 "# 标题 ## 子标题")  (从 0 开始)
```

- **标题路径**：从顶层到目标的空间分隔标题
- **块索引**：章节内的位置（0 = 标题后的第一个块）

## 安全特性

| 特性 | 说明 |
|------|------|
| `--force` | 破坏性操作必需 |
| `--fingerprint` | 破坏性变更前的正则验证（无法被 `--force` 绕过） |
| `--no-backup` | 跳过创建 `.bak` 备份文件 |
| `plan` 命令 | 干运行模式，显示 diff 但不应用 |
| 原子写入 | 临时文件 + 重命名保证崩溃安全 |
| 自动备份 | 任何文件修改前创建 `.bak` 文件 |

## 退出码

| 代码 | 含义 |
|------|------|
| 0 | 成功（包括无操作） |
| 1 | 一般错误 / 文件未找到 |
| 2 | 标题未找到 |
| 3 | 指纹不匹配 |
| 4 | 标题匹配歧义 |

## JSON 输出

用于程序化集成：

```bash
mdp patch -f doc.md -H "## 章节" --op append -c "新内容" --force -F json
```

输出格式：

```json
{
  "success": true,
  "applied": true,
  "is_noop": false,
  "changes": [
    {
      "file": "doc.md",
      "operation": "append",
      "heading": "## 章节",
      "index": 0,
      "status": "applied"
    }
  ]
}
```

状态值：

- `"applied"` - 变更成功应用
- `"noop"` - 无需变更（幂等操作）
- `"dry-run"` - 预览模式，未做修改

## Agent 集成

专为 AI Agent 工作流设计：

```bash
# Agent 检查将要变更的内容
mdp plan operations.yaml --format json

# Agent 验证后应用
mdp apply operations.yaml --force --format json

# 检查操作是否为无操作（幂等）
mdp patch -f doc.md -H "## 章节" --op append -c "已有内容" --force -F json | jq '.is_noop'
```

JSON 输出支持程序化错误处理和决策。

## 设计理念

### Force 与 Fingerprint

`--force` 和 `--fingerprint` 服务于不同目的：

- **Fingerprint**：内容验证（"这是正确的块吗？"）
  - 指纹不匹配 = 退出码 3
  - **无法**被 `--force` 绕过（它是定位器，不是权限）
  
- **Force**：执行权限（"我接受风险"）
  - 破坏性操作无需 fingerprint 时需要
  - 绕过安全检查但**不**绕过指纹验证

```bash
# ✓ 有效：fingerprint 验证内容
mdp patch ... --fingerprint "TODO.*" --force

# ✓ 有效：force 无 fingerprint 接受风险  
mdp patch ... --force

# ✗ 失败：fingerprint 不匹配（force 无帮助）
mdp patch ... --fingerprint "错误" --force  # 退出码 3
```

### 备份策略

默认情况下，`mdp` 在任何文件修改前创建 `.bak` 备份：

1. 原始内容复制到 `file.bak`
2. 变更写入临时文件
3. 临时文件原子重命名为目标文件

在 CI/CD 等只读文件系统环境中使用 `--no-backup` 跳过备份创建。

## 文档

- [快速开始](docs/zh/quickstart.md)
- [API 参考](docs/zh/api-reference.md)
- [设计文档](docs/zh/design.md)

## 许可证

MIT
