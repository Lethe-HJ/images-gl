# Git Commit Message 指南

## 📋 基本格式

```
<type>(<scope>): <subject>

<body>

<footer>
```

## 🏷️ Type 类型

### 主要类型

- **feat**: 新功能 (feature)
- **fix**: 修复 bug
- **docs**: 文档更新
- **style**: 代码格式调整（不影响功能）
- **refactor**: 代码重构（既不是新功能也不是修复 bug）
- **perf**: 性能优化
- **test**: 测试相关
- **chore**: 构建过程或辅助工具的变动
- **ci**: CI/CD 相关变更

### 特殊类型

- **break**: 破坏性变更（BREAKING CHANGE）
- **revert**: 回滚之前的 commit

## 🎯 Scope 范围

### 前端相关

- `ui`: 用户界面
- `component`: 组件
- `view`: 视图
- `style`: 样式
- `router`: 路由

### 后端相关

- `api`: API 接口
- `service`: 服务层
- `model`: 数据模型
- `config`: 配置

### 项目特定

- `image`: 图片处理相关
- `chunk`: 分块处理
- `cache`: 缓存相关
- `webgl`: WebGL 渲染
- `tauri`: Tauri 相关

## 📝 Subject 主题

### 规则

- 使用现在时态："add feature" 而不是 "added feature"
- 首字母小写
- 结尾不加句号
- 长度不超过 50 个字符
- 使用祈使句

### 示例

```
✅ 好的例子:
feat(image): add chunk-based image loading
fix(cache): resolve memory leak in chunk processing
refactor(tauri): split large index.rs into modules

❌ 不好的例子:
feat(image): Added chunk-based image loading
fix(cache): Fixed memory leak in chunk processing
refactor(tauri): Split large index.rs into modules
```

## 📄 Body 正文

### 规则

- 解释"为什么"而不是"什么"
- 每行不超过 72 个字符
- 使用现在时态
- 空行分隔主题和正文

### 示例

```
feat(image): add chunk-based image loading

Implement chunk-based image loading system to handle large images
efficiently. This allows for progressive loading and better memory
management when dealing with high-resolution images.

- Add ChunkManager class for managing image chunks
- Implement WebGL-based chunk rendering
- Add spatial batching for optimal loading order
```

## 🔗 Footer 页脚

### 破坏性变更

```
BREAKING CHANGE: The API has changed significantly.
All existing code using the old API will need to be updated.
```

### 关闭 Issue

```
Closes #123
Fixes #456
Resolves #789
```

### 相关提交

```
Related to #123
See also #456
```

## 🎯 实际示例

### 1. 新功能

```
feat(image): add progressive chunk loading

Implement progressive loading system for large images using spatial
batching. This improves user experience by showing image content
gradually as chunks are loaded.

- Add spatial batching algorithm
- Implement chunk priority queue
- Add loading progress indicators
```

### 2. 修复 Bug

```
fix(cache): resolve memory leak in chunk processing

Fix memory leak caused by not properly cleaning up WebGL textures
when chunks are unloaded. This was causing gradual memory increase
over time.

- Add proper texture cleanup in ChunkManager
- Implement LRU cache eviction
- Add memory usage monitoring
```

### 3. 重构

```
refactor(tauri): split large index.rs into modules

Split the 880-line index.rs file into smaller, focused modules
to improve maintainability and code organization.

- Extract types to types.rs
- Move cache logic to cache.rs
- Separate preprocessing to preprocessing.rs
- Create dedicated commands.rs for Tauri commands

BREAKING CHANGE: Some internal functions are now private and
not accessible from outside the module.
```

### 4. 性能优化

```
perf(chunk): optimize pixel extraction with SIMD

Implement SIMD-optimized pixel extraction for better performance
when processing large image chunks.

- Add SIMD pixel extraction function
- Optimize memory access patterns
- Reduce CPU usage by 40% for large images
```

### 5. 文档更新

```
docs: add comprehensive API documentation

Add detailed documentation for all public APIs including examples
and usage patterns.

- Document all Tauri commands
- Add code examples for common use cases
- Include performance considerations
```

## 🚀 提交工作流

### 1. 检查更改

```bash
git status
git diff --cached
```

### 2. 添加文件

```bash
git add .
# 或
git add specific-file.rs
```

### 3. 提交

```bash
git commit -m "feat(image): add chunk-based loading system"
```

### 4. 推送

```bash
git push origin feature/chunk-loading
```

## 🔍 代码审查检查点

### 提交前检查

- [ ] 代码编译通过
- [ ] 测试通过
- [ ] 没有未使用的导入
- [ ] 代码格式正确
- [ ] 提交信息清晰

### 审查时关注

- [ ] 提交信息是否准确描述变更
- [ ] 变更是否合理
- [ ] 是否有破坏性变更
- [ ] 是否需要更新文档

## 📚 参考资源

- [Conventional Commits](https://www.conventionalcommits.org/)
- [Angular Commit Guidelines](https://github.com/angular/angular/blob/main/CONTRIBUTING.md#commit)
- [Git Commit Best Practices](https://chris.beams.io/posts/git-commit/)

## 🎯 项目特定示例

基于你的 images-gl 项目，这里是一些具体的提交示例：

```
feat(webgl): implement chunk-based image rendering
fix(chunk): resolve memory leak in texture cleanup
refactor(tauri): modularize image processing code
perf(cache): optimize chunk loading with spatial batching
docs(api): add Tauri command documentation
test(chunk): add unit tests for ChunkManager
chore(deps): update image processing dependencies
```

记住：好的提交信息是项目历史的重要组成部分，它帮助团队成员理解代码变更的原因和影响。
