# API 参考

完整的 `mdp` 命令行接口参考。

## 命令概览

```bash
mdp <COMMAND> [OPTIONS]
```

| 命令    | 描述                       |
| ------- | -------------------------- |
| `patch` | 应用单个补丁操作           |
| `apply` | 从 YAML 配置文件应用补丁   |
| `plan`  | 预览变更而不应用（干运行） |
| `help`  | 打印帮助信息               |

---

## `mdp patch`

对单个 Markdown 文件执行补丁操作。

### 用法

```bash
mdp patch [OPTIONS] --file <FILE> --heading <HEADING> --op <OPERATION>
```

### 必需参数

| 参数        | 短选项 | 描述                                    |
| ----------- | ------ | --------------------------------------- |
| `--file`    | `-f`   | 目标 Markdown 文件路径                  |
| `--heading` | `-H`   | 标题路径（如 `"# 标题 ## 子标题"`）     |
| `--op`      | `-o`   | 操作类型：`append`、`replace`、`delete` |

### 可选参数

| 参数            | 短选项 | 描述                             |
| --------------- | ------ | -------------------------------- |
| `--index`       | `-i`   | 块索引（默认：0）                |
| `--content`     | `-c`   | 要追加或替换的内容               |
| `--fingerprint` | `-p`   | 用于验证的指纹正则表达式         |
| `--force`       | 无     | 确认破坏性操作                   |
| `--no-backup`   | 无     | 跳过创建 `.bak` 备份             |
| `--format`      | `-F`   | 输出格式：`text`、`diff`、`json` |

### 示例

#### 追加内容

```bash
# 基本追加
mdp patch -f doc.md -H "## 功能" --op append -c "- 新功能" --force

# 追加到特定索引
mdp patch -f doc.md -H "## 功能" -i 1 --op append -c "更多内容" --force

# 多级标题
mdp patch -f doc.md -H "# 文档 ## 章节 ### 子章节" --op append \
  -c "子章节内容" --force
```

#### 替换内容

```bash
# 使用 fingerprint 安全替换
mdp patch -f doc.md -H "## API" --op replace \
  -c "新版本" \
  --fingerprint "旧版本.*" \
  --force

# 不带 fingerprint（仅 force）
mdp patch -f doc.md -H "## API" --op replace \
  -c "新版本" \
  --force
```

#### 删除内容

```bash
# 安全删除 - 验证内容
mdp patch -f doc.md -H "## 已废弃" --op delete \
  --fingerprint "此功能已废弃" \
  --force
```

#### JSON 输出

```bash
mdp patch -f doc.md -H "## 章节" --op append -c "内容" --force -F json
```

输出：

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

---

## `mdp apply`

从 YAML 配置文件应用批量补丁。

### 用法

```bash
mdp apply [OPTIONS] <CONFIG_FILE>
```

### 参数

| 参数            | 描述              |
| --------------- | ----------------- |
| `<CONFIG_FILE>` | YAML 配置文件路径 |

### 选项

| 选项                | 描述                             |
| ------------------- | -------------------------------- |
| `--force`           | 确认所有破坏性操作               |
| `--no-backup`       | 跳过创建备份文件                 |
| `--format <FORMAT>` | 输出格式：`text`、`diff`、`json` |

### YAML 配置格式

```yaml
operations:
  - file: path/to/file.md
    heading:
      - "# 一级标题"
      - "## 二级标题"
    index: 0
    operation: append|replace|delete
    content: "要插入的内容"
    fingerprint: "验证正则"
```

### 字段说明

| 字段          | 必需 | 描述                                    |
| ------------- | ---- | --------------------------------------- |
| `file`        | 是   | 目标文件路径（相对或绝对）              |
| `heading`     | 是   | 标题路径数组                            |
| `index`       | 否   | 块索引（默认：0）                       |
| `operation`   | 是   | 操作类型：`append`、`replace`、`delete` |
| `content`     | 条件 | `append` 和 `replace` 必需              |
| `fingerprint` | 否   | 内容验证正则表达式                      |

### 示例配置

````yaml
operations:
  # 追加操作
  - file: docs/guide.md
    heading:
      - "# 用户指南"
      - "## 安装"
    index: 0
    operation: append
    content: "\n### 从源码安装\n\n```bash\ngit clone ...\n```"

  # 替换操作
  - file: docs/api.md
    heading:
      - "# API 参考"
    index: 1
    operation: replace
    content: "新 API 描述"
    fingerprint: "旧 API.*"

  # 删除操作
  - file: docs/legacy.md
    heading:
      - "# 已废弃功能"
    index: 0
    operation: delete
    fingerprint: "此功能不再维护"
````

### 执行

```bash
# 基本应用
mdp apply patches.yaml --force

# JSON 输出
mdp apply patches.yaml --force --format json

# 无备份
mdp apply patches.yaml --force --no-backup
```

---

## `mdp plan`

预览变更而不实际应用（干运行模式）。

### 用法

```bash
mdp plan [OPTIONS] <CONFIG_FILE>
```

### 参数

| 参数            | 描述              |
| --------------- | ----------------- |
| `<CONFIG_FILE>` | YAML 配置文件路径 |

### 选项

| 选项                | 描述                                             |
| ------------------- | ------------------------------------------------ |
| `--format <FORMAT>` | 输出格式：`text`、`diff`、`json`（默认：`diff`） |

### 示例

```bash
# 查看 diff
mdp plan patches.yaml

# JSON 格式的预览
mdp plan patches.yaml --format json

# 文本摘要
mdp plan patches.yaml --format text
```

### Diff 输出示例

````diff
--- a/docs/guide.md
+++ b/docs/guide.md
@@ -10,6 +10,10 @@

 ## 安装

+### 从源码安装
+
+```bash
+git clone ...
+```
+
 使用包管理器安装。
````

---

## 退出码

| 代码 | 常量                        | 含义                               |
| ---- | --------------------------- | ---------------------------------- |
| 0    | `EXIT_SUCCESS`              | 成功（包括无操作）                 |
| 1    | `EXIT_ERROR`                | 一般错误（文件未找到、权限错误等） |
| 2    | `EXIT_HEADING_NOT_FOUND`    | 指定的标题路径未找到               |
| 3    | `EXIT_FINGERPRINT_MISMATCH` | 指纹验证失败                       |
| 4    | `EXIT_AMBIGUOUS_HEADING`    | 标题匹配歧义（多个匹配）           |

### 在脚本中使用

```bash
#!/bin/bash

mdp patch -f doc.md -H "## 章节" --op append -c "内容" --force
exit_code=$?

if [ $exit_code -eq 0 ]; then
    echo "✓ 补丁成功应用"
elif [ $exit_code -eq 3 ]; then
    echo "✗ 指纹不匹配，取消操作"
    exit 1
else
    echo "✗ 错误：退出码 $exit_code"
    exit 1
fi
```

---

## 输出格式

### Text 格式

适合人类阅读的状态摘要：

```text
✓ Applied: doc.md ## 章节 [append]
✓ No-op: doc.md ## 其他 [append] (内容已存在)
```

### Diff 格式

显示统一的 diff 输出，适合查看具体变更：

```diff
--- a/file.md
+++ b/file.md
@@ -5,6 +5,8 @@

 ## 章节

+新追加的内容
+
 原有内容
```

### JSON 格式

适合程序化处理的结构化输出：

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

#### JSON 字段说明

| 字段                  | 类型    | 描述                               |
| --------------------- | ------- | ---------------------------------- |
| `success`             | boolean | 操作是否成功（无错误）             |
| `applied`             | boolean | 是否有实际变更应用                 |
| `is_noop`             | boolean | 是否为无操作（幂等）               |
| `changes`             | array   | 变更详情列表                       |
| `changes[].file`      | string  | 文件路径                           |
| `changes[].operation` | string  | 操作类型                           |
| `changes[].heading`   | string  | 标题路径                           |
| `changes[].index`     | number  | 块索引                             |
| `changes[].status`    | string  | 状态：`applied`、`noop`、`dry-run` |

---

## 环境变量

| 变量             | 描述                                       |
| ---------------- | ------------------------------------------ |
| `MDP_NO_COLOR`   | 禁用彩色输出                               |
| `MDP_BACKUP_DIR` | 备份文件存放目录（默认：与目标文件同目录） |
