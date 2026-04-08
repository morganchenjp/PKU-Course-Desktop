# 贡献指南

感谢您对 PKU Course Desktop 项目的关注！

## 开发环境设置

1. Fork 并克隆仓库
2. 安装依赖（见 README.md）
3. 创建功能分支：`git checkout -b feature/your-feature`
4. 提交更改：`git commit -am 'Add some feature'`
5. 推送分支：`git push origin feature/your-feature`
6. 提交 Pull Request

## 代码规范

### 前端 (TypeScript/Svelte)

- 使用 2 空格缩进
- 使用单引号
- 最大行长度 100
- 使用 TypeScript 严格模式

### 后端 (Rust)

- 使用 `cargo fmt` 格式化代码
- 使用 `cargo clippy` 检查代码
- 遵循 Rust 命名规范

## 提交信息规范

使用语义化提交信息：

- `feat:` 新功能
- `fix:` 修复问题
- `docs:` 文档更新
- `style:` 代码格式（不影响功能）
- `refactor:` 代码重构
- `test:` 测试相关
- `chore:` 构建过程或辅助工具的变动

示例：
```
feat: 添加批量下载功能
fix: 修复 m3u8 转码失败的问题
docs: 更新 README 安装说明
```

## 报告问题

请使用 GitHub Issues 报告问题，并提供：

1. 操作系统和版本
2. 应用版本
3. 问题描述
4. 复现步骤
5. 期望行为
6. 实际行为
7. 截图（如适用）

## 功能请求

欢迎提出功能建议！请在 GitHub Issues 中描述：

1. 功能描述
2. 使用场景
3. 可能的实现方案（可选）

## 许可证

通过贡献代码，您同意您的贡献将在 GPL-3.0 许可证下发布。
