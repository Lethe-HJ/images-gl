# Git Commit Message 指南

## 📋 基本格式

```
<type>(<scope>): <subject>

<body>

<footer>
```

## 🏷️ 支持的类型

- **feat**: 新功能 (feature)
- **fix**: 修复bug
- **docs**: 文档更新
- **style**: 代码格式调整
- **refactor**: 代码重构
- **perf**: 性能优化
- **test**: 测试相关
- **chore**: 构建过程或辅助工具
- **ci**: CI/CD相关
- **build**: 构建相关
- **revert**: 回滚

## 🎯 支持的范围

### 前端相关

- `ui`, `component`, `view`, `style`, `router`

### 后端相关

- `api`, `service`, `model`, `config`

### 项目特定

- `image`, `chunk`, `cache`, `webgl`, `tauri`

### 通用

- `deps`, `ci`, `build`, `docs`, `test`

## 📝 示例

### ✅ 好的例子

```
feat(image): add chunk-based image loading
fix(cache): resolve memory leak in chunk processing
refactor(tauri): split large index.rs into modules
docs(api): add Tauri command documentation
chore(deps): update image processing dependencies
```

### ❌ 不好的例子

```
feat: add feature
fix: bug fix
update: some changes
```

## 🚀 提交前检查

在提交前，系统会自动运行以下检查：

1. **前端代码格式** (Prettier)
2. **前端代码质量** (ESLint)
3. **Rust代码格式** (cargo fmt)
4. **Rust代码编译** (cargo check)
5. **Rust代码质量** (cargo clippy)

如果检查失败，请运行相应的修复命令：

```bash
# 修复所有问题
yarn check:all

# 或分别修复
yarn format      # 修复前端格式
yarn lint        # 修复前端代码质量
cargo fmt           # 修复Rust格式
cargo check         # 检查Rust编译
cargo clippy        # 检查Rust代码质量
```

## 🔧 快速修复命令

```bash
# 一键修复所有格式问题
yarn format && cd src-tauri && cargo fmt

# 一键检查所有问题
yarn check:all
```

## 📚 更多信息

- [Conventional Commits](https://www.conventionalcommits.org/)
- [Angular Commit Guidelines](https://github.com/angular/angular/blob/main/CONTRIBUTING.md#commit)
