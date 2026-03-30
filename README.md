<div align="center">

# Autoclick

**基于 Tauri 2 + Rust + React 的 Windows 桌面自动点击工具**

[![Rust](https://img.shields.io/badge/Rust-stable-orange?logo=rust)](https://www.rust-lang.org/)
[![Tauri](https://img.shields.io/badge/Tauri-2-blue?logo=tauri)](https://v2.tauri.app/)
[![React](https://img.shields.io/badge/React-18-61dafb?logo=react)](https://react.dev/)
[![Platform](https://img.shields.io/badge/Platform-Windows-0078d4?logo=windows)](https://www.microsoft.com/windows)
[![License](https://img.shields.io/badge/License-MIT-green)](./LICENSE)

</div>

---

## 项目简介

Autoclick Tauri 2 是一款仅面向 Windows 的桌面自动化工具，围绕“捕获目标窗口 / 屏幕区域 → 图像模板匹配 → 自动点击 → 实时预览与诊断”这一完整链路构建。核心运行链路由 Rust 原生实现，前端使用 React + TypeScript 提供桌面控制台界面，用于配置参数、管理模板、观察实时状态和导出诊断信息。

如果你是第一次接触这个项目，建议先读完本文的“快速开始”和“首次使用流程”，再根据需要进入`docs/user_guide.md`查看更细的操作说明。

## 核心能力

- **模板匹配驱动**：导入或粘贴模板图片，在目标窗口或显示器区域内执行匹配。
- **多模板管理**：支持模板列表、标签筛选、重命名、删除和预览查看。
- **窗口定位与测试捕获**：枚举目标窗口、绑定句柄、测试当前捕获结果。
- **实时预览与冻结对比**：查看当前捕获帧、命中框、点击点，并支持冻结帧对比。
- **运行时状态控制**：支持开始扫描、停止扫描和重启链路。
- **日志与诊断导出**：一键导出诊断包，便于复盘问题。
- **旧版工程迁移**：支持从旧版 Python 工程执行 dry-run 或正式导入。
- **自动更新占位**：预留 Tauri Updater 能力，生产环境可通过配置启用。

## 技术栈

| 层级 | 技术 |
| --- | --- |
| 桌面框架 | Tauri 2 |
| 后端 | Rust workspace |
| 前端 | React 18 · TypeScript 5 · Tailwind CSS 3 |
| 状态管理 | Zustand |
| 路由 | React Router 6 |
| 构建 | Vite 7 · pnpm |
| 测试 | Vitest · cargo test · Criterion |

## 环境要求

| 依赖 | 要求 |
| --- | --- |
| 操作系统 | Windows 10 / 11 |
| Rust | stable（由`rust-toolchain.toml`约束） |
| Node.js | 22 及以上 |
| pnpm | 10 及以上 |
| C++ 工具链 | Visual Studio Build Tools + Windows SDK |

> Tauri 2 的系统依赖说明可参考[Tauri 官方文档](https://v2.tauri.app/start/prerequisites/)。

## 快速开始

### 1. 克隆并安装依赖

```bash
git clone https://github.com/Rain-Of-Stars/Autoclick.git
cd Autoclick
pnpm install
```

### 2. 做一次基础检查

```bash
cargo check --workspace
pnpm typecheck
pnpm test
```

### 3. 启动桌面开发环境

```bash
pnpm tauri dev
```

应用启动后会自动加载配置、模板、窗口列表、监视器信息、运行时快照和诊断概览。左侧导航共 7 个页面，顶部工具栏可直接开始扫描、重启链路或停止扫描。

## 首次使用流程

如果你是第一次启动应用，推荐按照下面顺序完成最小可用配置：

### 第一步：导入模板

进入“模板库”页面，选择以下任意方式准备模板：

- 在输入框中填写模板文件路径后导入。
- 直接从剪贴板粘贴图片生成模板。
- 为模板填写标签，方便后续筛选和批量管理。

建议先准备 1 到 3 张最关键的模板图，并确认预览正常显示。

### 第二步：选择目标窗口

进入“目标窗口”页面：

- 搜索目标窗口标题、类名或进程路径。
- 点击“设为目标”绑定句柄。
- 点击“测试捕获”确认当前窗口能被正确抓取。
- 如配置为窗口模式但尚未绑定目标，顶部“开始扫描”会阻止启动并提示先完成选择。

### 第三步：调整任务配置

进入“任务配置”页面，重点检查四类参数：

- **捕获**：捕获源、目标 FPS、ROI 区域、是否恢复最小化窗口。
- **检测**：阈值、灰度、多尺度、连续命中和冷却时间。
- **点击**：注入方式、点击偏移、点击前窗口校验。
- **恢复**：自动恢复开关、重试次数、恢复间隔。

如果只是验证链路是否通，可先保持默认值，仅按实际窗口大小修正 ROI。

### 第四步：开始扫描并观察状态

回到顶部工具栏，使用以下按钮控制运行时：

- `开始扫描`：启动捕获、检测和点击主链路。
- `重启链路`：适用于恢复异常或需要重新建链。
- `停止扫描`：终止当前扫描流程。

底部状态栏会持续显示状态、捕获 FPS、帧间隔、最近分数、缓冲占用和当前目标窗口。

### 第五步：查看实时预览

进入“实时预览”页面可观察：

- 当前捕获帧尺寸与预览令牌。
- 命中框位置、匹配模板名、点击点与分数。
- 暂停预览、冻结当前帧、开启冻结对比、清空冻结结果。

这一步非常适合排查“模板不命中”或“命中位置偏移”的问题。

### 第六步：导出诊断包

如果运行异常，进入“日志与诊断”页面：

- 查看告警队列和链路指标。
- 筛选日志文件。
- 点击“导出诊断包”保存当前配置、日志、路径和运行信息。
- 若你正在从旧版 Python 工程迁移，可在本页先做 dry-run，再执行正式导入。

## 页面说明

| 页面 | 作用 |
| --- | --- |
| 总览 | 查看运行状态、最近命中、点击结果、恢复次数和错误摘要 |
| 任务配置 | 配置捕获、检测、点击和恢复策略 |
| 模板库 | 管理模板导入、标签、预览与批量操作 |
| 目标窗口 | 枚举窗口、绑定目标、测试捕获 |
| 实时预览 | 观察匹配框、点击点、帧冻结与对比 |
| 日志诊断 | 查看告警、导出诊断包、执行旧版迁移 |
| 系统设置 | 查看运行路径、数据目录、日志目录和更新配置 |

## 常用命令

### 开发与验证

```bash
pnpm tauri dev
pnpm dev
pnpm typecheck
pnpm test
pnpm build
pnpm check
cargo test --workspace
cargo bench --workspace
```

### 质量检查

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
.\scripts\release_check.ps1
```

### Justfile 快捷入口

```bash
just check
just lint
just test
just build
just release-check
```

## 文档导航

- `docs/user_guide.md`：面向用户的详细上手说明书。
- `docs/troubleshooting.md`：常见故障与排查建议。
- `docs/migration_from_python.md`：旧版 Python 工程迁移说明。
- `docs/architecture.md`：项目架构与模块职责。
- `docs/perf_budget.md`：性能预算与相关说明。

## 项目结构

```text
├── docs/                   # 项目文档
├── scripts/                # 发布检查等脚本
├── src/                    # React 前端
│   ├── app/                # 路由与应用壳层
│   ├── components/         # 布局与通用组件
│   ├── pages/              # 7 个功能页面
│   ├── stores/             # Zustand 状态管理
│   └── lib/                # 类型契约、展示与 Tauri 客户端
├── src-tauri/              # Tauri 桌面壳 + Rust workspace
│   ├── src/                # 命令层、托盘、应用状态、运行时控制
│   ├── crates/             # 核心能力 crate
│   └── tests/              # Tauri / Windows 集成测试
├── tests/                  # 前端测试夹具与冒烟测试
└── target/                 # Rust / Tauri 构建输出
```

## 构建产物

执行`pnpm tauri build`后，安装包默认输出到：

```text
target/release/bundle/
├── nsis/
└── msi/
```

如果只做前端生产构建，可使用`pnpm build`，产物位于`dist/`。

## 使用注意事项

- 当前项目仅支持 Windows，不提供 macOS 或 Linux 运行能力。
- 使用窗口捕获模式时，必须先绑定目标窗口句柄。
- “关闭窗口”在桌面模式下通常配合托盘使用，建议先确认托盘行为后再结束应用。
- 调试图目录仅支持应用数据目录下的相对路径，不支持绝对路径或上级目录。
- 更新检查在未注入发布通道配置时会显示为占位状态，这属于预期行为。

## 当前提交前检查结果

本次提交前已完成以下基线验证，结果均通过：

- `pnpm test`
- `pnpm lint`
- `pnpm build`
- `cargo test --workspace`
- `pnpm check`

如需执行更完整的发版前检查，请运行`.\scripts\release_check.ps1`。

## 许可证

[MIT](./LICENSE)
