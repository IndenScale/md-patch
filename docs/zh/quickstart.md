# 快速开始

本指南将帮助你在几分钟内开始使用 `mdp` 工具。

## 安装

### 从源码构建

```bash
# 克隆仓库
git clone https://github.com/IndenScale/md-patch.git
cd md-patch

# 构建发布版本
cargo build --release

# 安装到系统路径
cp target/release/mdp ~/.local/bin/
```

### 验证安装

```bash
mdp --version
```

## 第一个补丁

创建一个测试文档：

```bash
cat > hello.md << 'EOF'
# 你好世界

## 介绍

这是介绍部分。

## 功能

- 基础功能
EOF
```

### 1. 追加内容

在 "## 功能" 章节后追加新功能：

```bash
# 干运行 - 查看将要做什么
mdp patch -f hello.md -H "## 功能" --op append -c "- 新增功能" --format diff

# 实际应用
mdp patch -f hello.md -H "## 功能" --op append -c "- 新增功能" --force
```

### 2. 替换内容

替换 "## 介绍" 章节的内容：

```bash
mdp patch -f hello.md -H "## 介绍" \
  --op replace \
  -c "这是更新后的介绍。" \
  --fingerprint "这是介绍部分。" \
  --force
```

### 3. 删除内容

删除整个 "## 功能" 章节：

```bash
mdp patch -f hello.md -H "## 功能" \
  --op delete \
  --fingerprint "- 基础功能" \
  --force
```

## 批量操作

对于多个文件的修改，使用 YAML 配置文件。

创建 `batch-patch.yaml`：

```yaml
operations:
  - file: hello.md
    heading:
      - "# 你好世界"
      - "## 介绍"
    index: 0
    operation: append
    content: "\n更多介绍内容。"

  - file: hello.md
    heading:
      - "# 你好世界"
      - "## 功能"
    index: 0
    operation: replace
    content: "- 功能 A\n- 功能 B"
    fingerprint: "- 新增功能"
```

执行批量操作：

```bash
# 预览
mdp plan batch-patch.yaml

# 应用
mdp apply batch-patch.yaml --force
```

## 寻址详解

### 标题路径格式

标题路径是从文档根到目标章节的路径：

```markdown
# 文档标题

## 第一章

### 第一节

内容 1

### 第二节

内容 2

## 第二章

内容 3
```

寻址示例：

| 目标 | 标题路径 | 块索引 |
|------|----------|--------|
| 第一章后的内容 | `# 文档标题 ## 第一章` | 0 |
| 第一节 | `# 文档标题 ## 第一章 ### 第一节` | 0 |
| 第二节 | `# 文档标题 ## 第一章 ### 第二节` | 0 |
| 第二章 | `# 文档标题 ## 第二章` | 0 |

### 块索引

块索引表示章节内的位置，从 0 开始：

```markdown
## 章节

[块 0] 第一个段落

[块 1] 第二个段落

[块 2] - 列表项 1
       - 列表项 2
```

- 索引 0 指向 "第一个段落"
- 索引 1 指向 "第二个段落"
- 索引 2 指向整个列表

## 安全最佳实践

### 1. 总是先使用 `plan`

```bash
mdp plan config.yaml --format diff
```

### 2. 破坏性操作使用 fingerprint

```bash
# 好的做法 - 验证内容后再替换
mdp patch -f doc.md -H "## API" \
  --op replace \
  -c "新 API" \
  --fingerprint "旧 API 名称" \
  --force
```

### 3. 检查退出码

```bash
mdp patch -f doc.md -H "## 章节" --op append -c "内容" --force
exit_code=$?

case $exit_code in
  0) echo "成功" ;;
  1) echo "错误：一般错误" ;;
  2) echo "错误：标题未找到" ;;
  3) echo "错误：指纹不匹配" ;;
  4) echo "错误：标题匹配歧义" ;;
esac
```

### 4. 保留备份

默认会自动创建 `.bak` 文件。如需恢复：

```bash
cp doc.md.bak doc.md
```

## 下一步

- 阅读 [API 参考](api-reference.md) 了解所有命令和选项
- 阅读 [设计文档](design.md) 了解设计原理
