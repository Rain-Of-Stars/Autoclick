import { fireEvent, render, screen } from "@testing-library/react";
import { beforeEach, describe, expect, it } from "vitest";
import { MemoryRouter, Route, Routes } from "react-router-dom";
import { Sidebar } from "../src/components/layout/Sidebar";
import { TopBar } from "../src/components/layout/TopBar";
import type { AppConfig } from "../src/lib/contracts";
import DashboardPage from "../src/pages/dashboard";
import DiagnosticsPage from "../src/pages/diagnostics";
import LivePreviewPage from "../src/pages/live-preview";
import SystemSettingsPage from "../src/pages/system-settings";
import TaskConfigPage from "../src/pages/task-config";
import TargetWindowPage from "../src/pages/target-window";
import TemplatesPage from "../src/pages/templates";
import { useConfigStore } from "../src/stores/configStore";
import { useRuntimeStore } from "../src/stores/runtimeStore";

const tinyPreviewBytes = [
  137, 80, 78, 71, 13, 10, 26, 10, 0, 0, 0, 13, 73, 72, 68, 82, 0, 0, 0, 1, 0, 0, 0,
  1, 8, 4, 0, 0, 0, 181, 28, 12, 2, 0, 0, 0, 11, 73, 68, 65, 84, 120, 218, 99, 252,
  255, 31, 0, 3, 3, 2, 0, 239, 156, 23, 219, 0, 0, 0, 0, 73, 69, 78, 68, 174, 66, 96,
  130
];

const baseConfig: AppConfig = {
  schemaVersion: 1,
  capture: {
    source: "Window",
    monitor: { id: "primary", name: "主显示器", isPrimary: true },
    roi: { x: 0, y: 0, width: 0, height: 0 },
    targetFps: 30,
    timeoutMs: 1000,
    includeCursor: false,
    restoreMinimizedNoactivate: true,
    restoreMinimizedAfterCapture: false,
    windowBorderRequired: true,
    screenBorderRequired: false
  },
  detection: {
    threshold: 0.88,
    grayscale: true,
    multiScale: false,
    scales: [1],
    minDetections: 1,
    cooldownMs: 5000,
    earlyExit: true
  },
  input: {
    method: "Message",
    verifyWindowBeforeClick: true,
    clickOffsetX: 0,
    clickOffsetY: 0
  },
  recovery: {
    enableAutoRecovery: true,
    maxRecoveryAttempts: 5,
    recoveryCooldownSecs: 10,
    autoUpdateTargetByProcess: false,
    autoUpdateIntervalMs: 5000
  },
  target: {
    hwnd: 1,
    processName: "demo.exe",
    processPath: "apps/demo.exe",
    titleContains: "Demo",
    className: "DemoWindow",
    allowPartialMatch: true,
    strategies: {
      processName: true,
      processPath: true,
      windowTitle: true,
      className: true,
      fuzzyMatch: true
    }
  },
  ui: {
    enableLogging: true,
    enableNotifications: true,
    autoStartScan: false,
    debugMode: false,
    saveDebugImages: false,
    debugImageDir: "debug_images"
  },
  templates: [],
  runtimeStatus: "Idle"
};

beforeEach(() => {
  useConfigStore.setState({
    config: structuredClone(baseConfig),
    paths: {
      dataDir: "mock-data/data",
      cacheDir: "mock-data/cache",
      logDir: "mock-data/logs",
      templatesDir: "mock-data/templates",
      debugDir: "mock-data/debug",
      dbPath: "mock-data/data/autoclick.db"
    },
    bootstrap: {
      appName: "Autoclick Tauri 2",
      version: "6.0.0",
      runtimePath: "rust-workspace"
    },
    modules: ["autoclick-runtime", "autoclick-storage"],
    templates: [
      {
        id: "tpl-1",
        name: "确认按钮",
        hash: "hash-1",
        sourcePath: "confirm.png",
        storedPath: "templates/confirm.png",
        width: 32,
        height: 16,
        tags: ["按钮"],
        createdAt: new Date().toISOString()
      }
    ],
    windows: [
      {
        hwnd: 1,
        title: "Demo Window",
        className: "DemoWindow",
        pid: 1234,
        exePath: "apps/demo.exe",
        isMinimized: false,
        isVisible: true,
        rect: { left: 0, top: 0, right: 800, bottom: 600 }
      }
    ],
    monitors: [],
    diagnostics: {
      paths: {
        dataDir: "mock-data/data",
        cacheDir: "mock-data/cache",
        logDir: "mock-data/logs",
        templatesDir: "mock-data/templates",
        debugDir: "mock-data/debug",
        dbPath: "mock-data/data/autoclick.db"
      },
      runtime: {
        status: "Running",
        metrics: {
          runtime: {
            status: "Running",
            performance: {
              captureFps: 30,
              frameIntervalMs: 33.3,
              detectLatencyMs: 7,
              previewLatencyMs: 1.4,
              endToEndLatencyMs: 8.8,
              clickCount: 1,
              lastScore: 0.97,
              uptimeSecs: 12
            },
            capture: {
              frameWidth: 1280,
              frameHeight: 720,
              drops: 0,
              activeSource: "window"
            },
            recovery: {
              attempts: 0,
              lastReason: null,
              nextRetryInMs: null
            },
            preview: {
              enabled: true,
              frameToken: "preview-1",
              width: 640,
              height: 360
            },
            lastError: null
          },
          recoveryCount: 0,
          bufferDrops: 0,
          memoryBytesEstimate: 4096
        },
        preview: null,
        activeTarget: null,
        bestMatch: null,
        decision: "NoMatch",
        lastClick: null,
        lastError: null
      },
      logs: [
        {
          name: "autoclick.log",
          path: "mock-data/logs/autoclick.log",
          sizeBytes: 1024
        }
      ]
    },
    locatedTarget: {
      window: {
        hwnd: 1,
        title: "Demo Window",
        className: "DemoWindow",
        pid: 1234,
        exePath: "apps/demo.exe",
        isMinimized: false,
        isVisible: true,
        rect: { left: 0, top: 0, right: 800, bottom: 600 }
      },
      reliability: 95,
      reason: "hwnd 精确匹配"
    },
    targetCapture: null,
    importReport: null,
    loading: false,
    saving: false,
    error: null
  });

  useRuntimeStore.setState({
    snapshot: {
      status: "Running",
      metrics: {
        runtime: {
          status: "Running",
          performance: {
            captureFps: 30,
            frameIntervalMs: 33.3,
            detectLatencyMs: 7,
            previewLatencyMs: 1.4,
            endToEndLatencyMs: 8.8,
            clickCount: 2,
            lastScore: 0.98,
            uptimeSecs: 20
          },
          capture: {
            frameWidth: 1280,
            frameHeight: 720,
            drops: 0,
            activeSource: "window"
          },
          recovery: {
            attempts: 0,
            lastReason: null,
            nextRetryInMs: null
          },
          preview: {
            enabled: true,
            frameToken: "preview-1",
            width: 640,
            height: 360
          },
          lastError: null
        },
        recoveryCount: 0,
        bufferDrops: 0,
        memoryBytesEstimate: 4096
      },
      preview: null,
      activeTarget: null,
      bestMatch: {
        templateId: "tpl-1",
        templateName: "确认按钮",
        score: 0.98,
        x: 100,
        y: 120,
        width: 32,
        height: 16,
        scale: 1
      },
      decision: "NoMatch",
      lastClick: null,
      lastError: null
    },
    preview: null,
    loading: false,
    error: null
  });
});

describe("frontend smoke", () => {
  it("renders dashboard metrics", () => {
    render(<DashboardPage />);
    expect(screen.getByText("运行总览")).toBeInTheDocument();
    expect(screen.getByText("确认按钮")).toBeInTheDocument();
    expect(screen.getByText(/模板.*1/)).toBeInTheDocument();
  });

  it("renders task config sections", () => {
    render(<TaskConfigPage />);
    expect(screen.getByText("任务配置")).toBeInTheDocument();
    expect(screen.getByText("捕获")).toBeInTheDocument();
    expect(screen.getByText("检测")).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "保存配置" })).toBeInTheDocument();
  });

  it("renders live preview fallback and diagnostics controls", () => {
    render(<LivePreviewPage />);
    expect(screen.getByText("实时预览")).toBeInTheDocument();
    expect(screen.getByText("当前没有可显示的预览帧。")).toBeInTheDocument();
  });

  it("renders live preview without visual grid overlay", () => {
    useRuntimeStore.setState((state) => ({
      ...state,
      preview: {
        token: "preview-1",
        preview: {
          frameId: 1,
          width: 4,
          height: 4,
          mimeType: "image/png",
          bytes: tinyPreviewBytes
        }
      }
    }));

    const { container } = render(<LivePreviewPage />);

    expect(screen.getByAltText("Live Feed")).toBeInTheDocument();
    expect(container.querySelector(".desk-overlay-grid")).toBeNull();
  });

  it("renders live preview from snapshot when preview store is still empty", () => {
    useRuntimeStore.setState((state) => ({
      ...state,
      snapshot: state.snapshot
        ? {
            ...state.snapshot,
            preview: {
              token: "preview-from-snapshot",
              preview: {
                frameId: 2,
                width: 4,
                height: 4,
                mimeType: "image/png",
                bytes: tinyPreviewBytes
              }
            }
          }
        : state.snapshot,
      preview: null
    }));

    render(<LivePreviewPage />);

    expect(screen.getByAltText("Live Feed")).toBeInTheDocument();
    expect(screen.getAllByText("preview-from-snapshot")).toHaveLength(2);
  });

  it("renders diagnostics log table", () => {
    render(<DiagnosticsPage />);
    expect(screen.getByText("日志与诊断")).toBeInTheDocument();
    expect(screen.getByText("autoclick.log")).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "导出诊断包" })).toBeInTheDocument();
  });

  it("renders template library entries", async () => {
    render(<TemplatesPage />);
    expect(screen.getByText("资产模板库")).toBeInTheDocument();
    expect(screen.getByDisplayValue("确认按钮")).toBeInTheDocument();
    expect(screen.getByText("导入特征")).toBeInTheDocument();
    expect(screen.getByText("剪贴板快捷入库焦点区域")).toBeInTheDocument();
    expect(screen.getByText(/Ctrl\+V/)).toBeInTheDocument();
    expect(await screen.findByAltText("Template Stage")).toBeInTheDocument();
  });

  it("renders selected template visual preview", async () => {
    render(<TemplatesPage />);
    const preview = await screen.findByAltText("Template Stage");
    expect(preview).toBeInTheDocument();
    expect(preview).toHaveClass("h-full");
    expect(preview).toHaveClass("w-full");
    expect(screen.queryByText("源文件路径")).not.toBeInTheDocument();
    expect(screen.queryByText("应用特征链串")).not.toBeInTheDocument();
    expect(screen.queryByText("引擎存储集落")).not.toBeInTheDocument();
  });

  it("renders target window table and actions", () => {
    const { container } = render(<TargetWindowPage />);
    expect(screen.getByText("目标窗口")).toBeInTheDocument();
    expect(screen.getAllByText("Demo Window")).toHaveLength(2);
    expect(screen.getByRole("button", { name: "设为目标" })).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "测试捕获" })).toBeInTheDocument();
    expect(container.querySelector(".desk-scrollbar-visible")).toBeTruthy();
  });

  it("renders sidebar navigation", () => {
    render(
      <MemoryRouter
        future={{ v7_startTransition: true, v7_relativeSplatPath: true }}
      >
        <Sidebar />
      </MemoryRouter>
    );
    expect(screen.getByText("总览")).toBeInTheDocument();
    expect(screen.getByText("模板库")).toBeInTheDocument();
    expect(screen.getByText("日志诊断")).toBeInTheDocument();
  });

  it("disables start while runtime is starting", () => {
    const snapshot = useRuntimeStore.getState().snapshot;
    useRuntimeStore.setState({
      snapshot: snapshot
        ? {
            ...snapshot,
            status: "Starting",
            metrics: {
              ...snapshot.metrics,
              runtime: {
                ...snapshot.metrics.runtime,
                status: "Starting"
              }
            }
          }
        : snapshot
    });
    render(
      <MemoryRouter
        future={{ v7_startTransition: true, v7_relativeSplatPath: true }}
      >
        <TopBar />
      </MemoryRouter>
    );
    expect(screen.getByRole("button", { name: "开始扫描" })).toBeDisabled();
    expect(screen.getByRole("button", { name: "停止扫描" })).toBeEnabled();
  });

  it("prompts for target window selection before starting scan", () => {
    useConfigStore.setState((state) => ({
      ...state,
      config: state.config
        ? {
            ...state.config,
            target: {
              ...state.config.target,
              hwnd: null,
              processName: null,
              processPath: null,
              titleContains: null,
              className: null
            }
          }
        : state.config
    }));
    useRuntimeStore.setState((state) => ({
      ...state,
      snapshot: state.snapshot
        ? {
            ...state.snapshot,
            status: "Idle",
            metrics: {
              ...state.snapshot.metrics,
              runtime: {
                ...state.snapshot.metrics.runtime,
                status: "Idle"
              }
            }
          }
        : state.snapshot
    }));

    render(
      <MemoryRouter
        future={{ v7_startTransition: true, v7_relativeSplatPath: true }}
        initialEntries={["/"]}
      >
        <Routes>
          <Route path="/" element={<TopBar />} />
          <Route path="/target-window" element={<div>目标窗口页</div>} />
        </Routes>
      </MemoryRouter>
    );

    fireEvent.click(screen.getByRole("button", { name: "开始扫描" }));
    expect(screen.getByRole("dialog")).toBeInTheDocument();
    expect(screen.getByText("开始扫描前需要先选择目标窗口")).toBeInTheDocument();

    fireEvent.click(screen.getByRole("button", { name: "去选择窗口" }));
    expect(screen.getByText("目标窗口页")).toBeInTheDocument();
  });

  it("renders system settings updater section", async () => {
    render(<SystemSettingsPage />);
    expect(screen.getByText("系统设置")).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "检查更新" })).toBeInTheDocument();
    expect(await screen.findByText(/更新功能处于占位状态/)).toBeInTheDocument();
  });
});
