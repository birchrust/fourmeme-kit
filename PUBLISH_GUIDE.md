# 📦 Fourmeme Crates 发布指南

## 前置准备

### 1. 登录 crates.io
```bash
cargo login <your-api-token>
```
API Token 从这里获取：https://crates.io/settings/tokens

### 2. 修改 edition 版本
将所有 Cargo.toml 中的 `edition = "2024"` 改为 `edition = "2021"`

### 3. 添加 description 字段
在 workspace.package 中添加：
```toml
[workspace.package]
description = "A suite of tools for BSC meme token trading"
```

## 发布顺序（按依赖关系）

### 第一批：基础库（无外部内部依赖）
```bash
cargo publish -p types
cargo publish -p abi
cargo publish -p logging
```

### 第二批：中间层
```bash
cargo publish -p rpc
cargo publish -p bloxroute
cargo publish -p transaction-stream
cargo publish -p price-query
```

### 第三批：高级功能
```bash
cargo publish -p sender
cargo publish -p pancake-v2
cargo publish -p price-track
cargo publish -p telegram
```

### 第四批：主应用
```bash
cargo publish -p fourmeme
```

## 发布前检查

每个 crate 发布前运行：
```bash
# 检查是否可以打包
cargo package -p <crate-name>

# 检查是否可以发布（不实际发布）
cargo publish -p <crate-name> --dry-run
```

## 注意事项

1. **crate 名称唯一性**：确保 crates.io 上没有同名 crate
2. **版本号规则**：遵循语义化版本 (Semantic Versioning)
3. **不可撤销**：发布后无法删除，只能 yank（隐藏）
4. **等待时间**：每次发布后等待几分钟再发布依赖它的下一个 crate

## 检查 crate 名称是否可用

```bash
# 在浏览器中检查
# https://crates.io/crates/<crate-name>
# 如果显示 404，说明名称可用
```

## 更新版本号

使用 cargo-edit 工具：
```bash
cargo install cargo-edit
cargo set-version -p <crate-name> 0.1.1
```
