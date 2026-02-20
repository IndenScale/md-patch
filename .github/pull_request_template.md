## 描述
<!-- 描述这个 PR 做了什么 -->

## 类型
<!-- 选择适用的类型 -->
- [ ] Bug 修复
- [ ] 新功能
- [ ] 性能优化
- [ ] 文档更新
- [ ] 代码重构
- [ ] 测试

## 检查清单
<!-- 在合并前确认以下事项 -->
- [ ] 代码通过 `cargo test` 所有测试
- [ ] 代码通过 `cargo clippy` 检查
- [ ] 代码格式通过 `cargo fmt` 检查
- [ ] 集成测试通过 (GitHub Actions)
- [ ] 关键特性未被破坏：
  - [ ] 幂等性 (`append` 操作重复执行不重复添加)
  - [ ] 安全机制 (destructive 操作需要 fingerprint 或 --force)
  - [ ] 退出码 (1=一般错误, 2=heading未找到, 3=fingerprint不匹配, 4=歧义heading)
  - [ ] 嵌套 heading 路径支持

## 测试
<!-- 如何测试这些更改 -->
```bash
# 运行所有测试
cargo test

# 运行集成测试
cargo test --test integration_test

# 手动测试关键特性
./target/release/mdp patch -f test.md -H "## API" --op append -c "New" --force
```

## 相关 Issue
<!-- 如果有相关 issue，请引用 -->
Fixes #
