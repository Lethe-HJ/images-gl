# 项目命令说明

## 🚀 主要命令

### `yarn review`

**代码质量检查** - 用于 commit 钩子和 CI/CD

- 检查前端代码质量 (ESLint)
- 检查前端代码格式 (Prettier)
- 检查 Rust 代码编译 (cargo check)
- 检查 Rust 代码质量 (cargo clippy)

### `yarn fix`

**自动修复代码问题**

- 自动修复前端代码问题 (ESLint --fix)
- 自动格式化前端代码 (Prettier)
- 自动修复 Rust 代码问题 (cargo clippy --fix)
- 自动格式化 Rust 代码 (cargo fmt)

## 🔧 子命令

### 前端相关

- `yarn review:frontend` - 检查前端代码质量和格式
- `yarn fix:frontend` - 修复前端代码问题和格式

### Rust 相关

- `yarn review:rust` - 检查 Rust 代码编译和质量
- `yarn fix:rust` - 修复 Rust 代码问题和格式

## 📋 使用场景

### 开发时

```bash
# 检查代码质量
yarn review

# 自动修复问题
yarn fix
```

### Commit 前

```bash
# 确保代码质量
yarn review
```

### CI/CD

```bash
# 验证代码质量
yarn review
```

## ⚠️ 注意事项

- `review` 命令**不会修改代码**，仅用于检查
- `fix` 命令会**自动修改代码**，请确保已提交当前更改
- 如果 `fix` 命令无法自动修复某些问题，会显示具体的错误信息
- 建议在提交前运行 `review` 命令确保代码质量
