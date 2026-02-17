# Autoclick

基于 Windows 的托盘自动化工具，聚焦「窗口/屏幕捕获 + 模板匹配 + 自动点击」。

项目使用 PySide6 构建 UI，采用多线程/多进程解耦，目标是保证扫描性能的同时尽量不阻塞界面。


## 主要能力

- 支持 **窗口捕获** 与 **显示器捕获**（WGC）。
- 支持 OpenCV 模板匹配（灰度、多尺度、阈值、ROI）。
- 托盘常驻，支持一键开始/停止扫描、打开设置。
- 支持按进程名自动更新 HWND。
- 内置 GUI 响应性监控与 UI 更新节流。
- 支持高 DPI / 多屏场景下的缩放与坐标处理。


## 运行环境

- 操作系统：Windows 10/11
- Python：3.12（建议使用 conda 环境 `use`）
- 关键依赖：PySide6、OpenCV、numpy、windows-capture、aiohttp、qasync


## 快速开始

### 方式一（推荐：conda）

```bat
conda env create -f environment.yml -n use
conda activate use
python -m pip install -r requirements.txt
```

### 方式二（已有 Python 环境）

```bat
python -m pip install -r requirements.txt
```


## 启动应用

```bat
python main_auto_approve_refactored.py
```

启动后应用驻留系统托盘，可通过托盘菜单进行开始/停止扫描、打开设置等操作。


## 存储与配置

- 配置主存储：`app.db`（SQLite）
- 配置读写入口：`auto_approve/config_manager.py`
- 说明：
  - 配置以 JSON 结构存储在 SQLite 的 `config` 表中；
  - `config.json` 仅用于历史兼容/迁移，不作为主存储。


## 项目结构（核心）

```text
.
├── main_auto_approve_refactored.py   # 主入口（托盘、调度、状态更新）
├── auto_approve/                     # UI、配置、性能、DPI、窗口管理
├── capture/                          # 捕获相关（WGC、缓存、预览）
├── workers/                          # IO/CPU/异步任务与扫描进程
├── storage/                          # SQLite 存储层
├── tests/                            # 自动化测试
├── requirements.txt
└── environment.yml
```


## 开发与测试

### 语法检查

```bat
python -m py_compile main_auto_approve_refactored.py
```

### 运行测试（示例）

```bat
python -m pytest tests/test_settings_dialog_construct.py -q --maxfail=1
python -m pytest tests/test_wgc_preview_save_callback.py -q --maxfail=1
```

> 测试文件统一放在 `tests/` 目录。


## 最近修复（UI）

- 修复保存后 `control` 调度链断裂导致的“自动启动无效”问题。
- 修复 WGC 预览保存回调中的变量作用域异常。
- 修复设置对话框测试异常路径下进度框未关闭导致的 UI 阻塞。
- 缓解 HWND 列表刷新卡顿（后台枚举 + 分块填充）。
- 修复主屏切换后 DPI 基准未刷新问题。
- 优化按钮 hover 事件处理，避免覆盖原事件链。


## 常见问题

- **启动后无反应**  
  请先确认在 Windows 环境，并已正确安装 `windows-capture` 依赖。

- **匹配效果不稳定**  
  优先检查模板图与目标分辨率/缩放是否一致，再调整 `threshold` 与 `multi_scale`。

- **多屏坐标偏差**  
  检查 DPI 缩放比例与配置中的坐标转换相关选项。


## 免责声明

本项目仅用于学习与研究，请在合法合规场景下使用。
