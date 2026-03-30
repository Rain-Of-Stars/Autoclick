import type {
  AppConfig,
  AppPaths,
  BootstrapInfo,
  DiagnosticsOverview,
  EncodedPreview,
  LegacyImportReport,
  LocatorCandidate,
  MonitorInfo,
  RuntimeControllerSnapshot,
  TargetCaptureResult,
  TargetProfile,
  TemplateRef,
  TemplatePreview,
  UpdateCheckResult,
  UpdaterStatus,
  WindowInfo
} from "./contracts";

type InvokeArgs = Record<string, unknown> | undefined;

const clone = <T>(value: T): T => {
  if (typeof structuredClone === "function") {
    return structuredClone(value);
  }
  return JSON.parse(JSON.stringify(value)) as T;
};

const blankPreview: EncodedPreview = {
  frameId: 0,
  width: 0,
  height: 0,
  mimeType: "image/png",
  bytes: []
};

const mockCapturePreview: EncodedPreview = {
  frameId: 1,
  width: 1,
  height: 1,
  mimeType: "image/png",
  bytes: [
    137, 80, 78, 71, 13, 10, 26, 10, 0, 0, 0, 13, 73, 72, 68, 82, 0, 0, 0, 1, 0, 0, 0, 1,
    8, 4, 0, 0, 0, 181, 28, 12, 2, 0, 0, 0, 11, 73, 68, 65, 84, 120, 218, 99, 252, 255, 31,
    0, 3, 3, 2, 0, 239, 156, 23, 219, 0, 0, 0, 0, 73, 69, 78, 68, 174, 66, 96, 130
  ]
};

const mockWindows: WindowInfo[] = [
  {
    hwnd: 12041,
    title: "示例编辑器",
    className: "ExampleEditorWindow",
    pid: 4201,
    exePath: "apps/ExampleEditor/Editor.exe",
    isMinimized: false,
    isVisible: true,
    rect: { left: 120, top: 72, right: 1460, bottom: 940 }
  },
  {
    hwnd: 23881,
    title: "安卓模拟器",
    className: "AndroidEmulatorFrame",
    pid: 5208,
    exePath: "apps/AndroidEmulator/Emulator.exe",
    isMinimized: false,
    isVisible: true,
    rect: { left: 1620, top: 40, right: 2480, bottom: 1010 }
  }
];

const mockMonitors: MonitorInfo[] = [
  {
    handle: 1,
    name: "\\\\.\\DISPLAY1",
    isPrimary: true,
    rect: { left: 0, top: 0, right: 2560, bottom: 1440 },
    workRect: { left: 0, top: 0, right: 2560, bottom: 1400 },
    dpi: 144,
    scaleFactor: 1.5
  }
];

const mockConfig: AppConfig = {
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
    scales: [1, 1.25, 0.8],
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
    hwnd: mockWindows[1].hwnd,
    processName: "emulator.exe",
    processPath: mockWindows[1].exePath,
    titleContains: "安卓模拟器",
    className: "AndroidEmulatorFrame",
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

const mockTemplates: TemplateRef[] = [
  {
    id: "tpl-ldplayer-ok",
    name: "确认按钮",
    hash: "hash-confirm",
    sourcePath: "tests/fixtures/legacy_project/templates/confirm.png",
    storedPath: "data/templates/hash-confirm.png",
    width: 64,
    height: 32,
    tags: ["战斗", "按钮"],
    createdAt: new Date().toISOString()
  },
  {
    id: "tpl-ldplayer-battle",
    name: "开始战斗",
    hash: "hash-battle",
    sourcePath: "tests/fixtures/legacy_project/templates/battle.png",
    storedPath: "data/templates/hash-battle.png",
    width: 92,
    height: 44,
    tags: ["战斗"],
    createdAt: new Date().toISOString()
  }
];
mockConfig.templates = clone(mockTemplates);

const mockRuntime: RuntimeControllerSnapshot = {
  status: "Idle",
  metrics: {
    runtime: {
      status: "Idle",
      performance: {
        captureFps: 0,
        frameIntervalMs: 0,
        detectLatencyMs: 0,
        previewLatencyMs: 0,
        endToEndLatencyMs: 0,
        clickCount: 0,
        lastScore: 0,
        uptimeSecs: 0
      },
      capture: {
        frameWidth: 0,
        frameHeight: 0,
        drops: 0,
        activeSource: null
      },
      recovery: {
        attempts: 0,
        lastReason: null,
        nextRetryInMs: null
      },
      preview: {
        enabled: false,
        frameToken: null,
        width: 0,
        height: 0
      },
      lastError: null
    },
    recoveryCount: 0,
    bufferDrops: 0,
    memoryBytesEstimate: 0
  },
  preview: null,
  activeTarget: null,
  bestMatch: null,
  decision: null,
  lastClick: null,
  lastError: null
};

let mockPreviewSequence = 0;

const mockRuntimeProducesPreview = (status: RuntimeControllerSnapshot["status"]) =>
  status === "Starting" ||
  status === "Running" ||
  status === "CoolingDown" ||
  status === "Recovering";

const buildMockRuntimePreview = (frameId: number) => {
  const accentOffset = 120 + ((frameId * 37) % 360);
  const scanlineY = 140 + ((frameId * 29) % 360);
  const svg = `
<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 1280 720">
  <defs>
    <linearGradient id="bg" x1="0" y1="0" x2="1" y2="1">
      <stop offset="0%" stop-color="#081018" />
      <stop offset="100%" stop-color="#13263b" />
    </linearGradient>
    <linearGradient id="accent" x1="0" y1="0" x2="1" y2="0">
      <stop offset="0%" stop-color="#4a8bc7" stop-opacity="0.18" />
      <stop offset="100%" stop-color="#7fd4ff" stop-opacity="0.72" />
    </linearGradient>
  </defs>
  <rect width="1280" height="720" fill="url(#bg)" />
  <rect x="96" y="96" width="1088" height="528" rx="20" fill="#0b1622" stroke="#4a8bc7" stroke-opacity="0.32" />
  <rect x="${accentOffset}" y="146" width="320" height="18" rx="9" fill="url(#accent)" />
  <rect x="146" y="${scanlineY}" width="988" height="4" rx="2" fill="#90d7ff" fill-opacity="0.55" />
  <rect x="146" y="208" width="340" height="222" rx="18" fill="#132738" />
  <rect x="516" y="208" width="618" height="120" rx="18" fill="#101f2e" />
  <rect x="516" y="354" width="300" height="220" rx="18" fill="#132738" />
  <rect x="834" y="354" width="300" height="220" rx="18" fill="#132738" />
  <text x="146" y="160" fill="#dbeafe" font-size="34" font-family="Segoe UI, Arial, sans-serif">Live Preview Mock Feed</text>
  <text x="146" y="206" fill="#7dd3fc" font-size="18" font-family="Consolas, monospace">FRAME ${frameId.toString().padStart(4, "0")}</text>
  <text x="176" y="278" fill="#f8fafc" font-size="22" font-family="Segoe UI, Arial, sans-serif">Capture viewport</text>
  <text x="176" y="316" fill="#94a3b8" font-size="16" font-family="Segoe UI, Arial, sans-serif">Simulated desktop preview for browser dev mode</text>
  <text x="548" y="264" fill="#f8fafc" font-size="22" font-family="Segoe UI, Arial, sans-serif">Runtime metrics</text>
  <text x="548" y="306" fill="#94a3b8" font-size="16" font-family="Consolas, monospace">fps=29.8 latency=8.9ms source=${mockConfig.capture.source}</text>
  <text x="548" y="416" fill="#f8fafc" font-size="20" font-family="Segoe UI, Arial, sans-serif">Match inspector</text>
  <text x="548" y="452" fill="#94a3b8" font-size="15" font-family="Consolas, monospace">template=confirm score=0.982</text>
  <text x="866" y="416" fill="#f8fafc" font-size="20" font-family="Segoe UI, Arial, sans-serif">Click channel</text>
  <text x="866" y="452" fill="#94a3b8" font-size="15" font-family="Consolas, monospace">method=Message status=armed</text>
  <rect x="214" y="340" width="132" height="54" rx="12" fill="none" stroke="#fbbf24" stroke-width="4" />
</svg>`.trim();

  return {
    token: `preview-${frameId}`,
    preview: {
      frameId,
      width: 1280,
      height: 720,
      mimeType: "image/svg+xml",
      bytes: Array.from(new TextEncoder().encode(svg))
    }
  };
};

const syncMockRuntimePreviewMetrics = () => {
  if (!mockRuntime.preview) {
    mockRuntime.metrics.runtime.preview = {
      enabled: false,
      frameToken: null,
      width: 0,
      height: 0
    };
    return;
  }

  mockRuntime.metrics.runtime.preview = {
    enabled: true,
    frameToken: mockRuntime.preview.token,
    width: mockRuntime.preview.preview.width,
    height: mockRuntime.preview.preview.height
  };
};

const updateMockRuntimePreview = () => {
  if (!mockRuntimeProducesPreview(mockRuntime.status)) {
    mockRuntime.preview = null;
    syncMockRuntimePreviewMetrics();
    return null;
  }

  mockPreviewSequence += 1;
  mockRuntime.preview = buildMockRuntimePreview(mockPreviewSequence);
  mockRuntime.metrics.runtime.capture.activeSource =
    mockConfig.capture.source === "Window" ? "window" : "monitor";
  syncMockRuntimePreviewMetrics();
  return mockRuntime.preview;
};

const resetMockRuntimePreview = () => {
  mockRuntime.preview = null;
  syncMockRuntimePreviewMetrics();
};

const mockPaths: AppPaths = {
  dataDir: "mock-data/data",
  cacheDir: "mock-data/cache",
  logDir: "mock-data/logs",
  templatesDir: "mock-data/data/templates",
  debugDir: "mock-data/cache/debug",
  dbPath: "mock-data/data/autoclick.db"
};

const mockBootstrap: BootstrapInfo = {
  appName: "Autoclick Tauri 2",
  version: "6.0.0",
  runtimePath: "rust-workspace"
};

const mockLogs = [
  {
    name: "autoclick.log",
    path: `${mockPaths.logDir}/autoclick.log`,
    sizeBytes: 32768
  }
];

const mockInvoke = async <T>(command: string, args?: InvokeArgs): Promise<T> => {
  switch (command) {
    case "get_updater_status":
      return {
        configured: false,
        pubkeyConfigured: false,
        installMode: "passive",
        reason: "尚未配置发布通道，更新功能处于占位状态。"
      } as T;
    case "check_for_updates":
      return {
        configured: false,
        checked: false,
        updateAvailable: false,
        currentVersion: "6.0.0",
        latestVersion: null,
        body: null,
        date: null,
        reason: "尚未配置发布通道，更新功能处于占位状态。"
      } as T;
    case "get_bootstrap_info":
      return clone(mockBootstrap) as T;
    case "list_workspace_modules":
      return [
        "autoclick-domain",
        "autoclick-platform-win",
        "autoclick-storage",
        "autoclick-capture",
        "autoclick-detect",
        "autoclick-input",
        "autoclick-runtime",
        "autoclick-diagnostics"
      ] as T;
    case "get_config":
      return clone(mockConfig) as T;
    case "save_config":
      Object.assign(mockConfig, clone((args?.config as AppConfig) ?? mockConfig));
      return clone(mockConfig) as T;
    case "get_app_paths":
      return clone(mockPaths) as T;
    case "list_templates":
      return clone(mockTemplates) as T;
    case "get_template_preview": {
      const request = args?.request as { templateId: string };
      const template =
        mockTemplates.find((item) => item.id === request.templateId) ??
        mockTemplates[0] ?? {
          width: 64,
          height: 64
        };
      return {
        mimeType: "image/png",
        bytes: clone(mockCapturePreview.bytes),
        width: template.width,
        height: template.height
      } as T;
    }
    case "import_template": {
      const request = args?.request as { filePath: string; tags: string[] };
      const name = request.filePath.split(/[\\/]/).pop()?.replace(/\.[^.]+$/, "") || "template";
      const created: TemplateRef = {
        id: crypto.randomUUID(),
        name,
        hash: crypto.randomUUID().replaceAll("-", ""),
        sourcePath: request.filePath,
        storedPath: `${mockPaths.templatesDir}/${name}.png`,
        width: 64,
        height: 64,
        tags: request.tags,
        createdAt: new Date().toISOString()
      };
      mockTemplates.unshift(created);
      mockConfig.templates = clone(mockTemplates);
      return clone(created) as T;
    }
    case "import_pasted_template": {
      const request = args?.request as {
        name: string;
        tags: string[];
        bytes: number[];
      };
      const created: TemplateRef = {
        id: crypto.randomUUID(),
        name: request.name.trim() || "粘贴模板",
        hash: crypto.randomUUID().replaceAll("-", ""),
        sourcePath: "clipboard://image",
        storedPath: `${mockPaths.templatesDir}/${Date.now()}-clipboard.png`,
        width: request.bytes.length > 0 ? 64 : 0,
        height: request.bytes.length > 0 ? 64 : 0,
        tags: request.tags,
        createdAt: new Date().toISOString()
      };
      mockTemplates.unshift(created);
      mockConfig.templates = clone(mockTemplates);
      return clone(created) as T;
    }
    case "remove_template": {
      const templateId = args?.templateId as string;
      const index = mockTemplates.findIndex((item) => item.id === templateId);
      if (index >= 0) {
        mockTemplates.splice(index, 1);
      }
      mockConfig.templates = clone(mockTemplates);
      return clone(mockTemplates) as T;
    }
    case "rename_template": {
      const request = args?.request as { templateId: string; name: string };
      const item = mockTemplates.find((template) => template.id === request.templateId);
      if (item) {
        item.name = request.name;
      }
      return clone(item ?? mockTemplates[0]) as T;
    }
    case "list_target_windows":
      return clone(mockWindows) as T;
    case "list_monitors":
      return clone(mockMonitors) as T;
    case "locate_target": {
      const matched = mockWindows.find((window) => window.hwnd === mockConfig.target.hwnd);
      const located: LocatorCandidate | null = matched
        ? {
            window: matched,
            reliability: 95,
            reason: "hwnd 精确匹配"
          }
        : null;
      return clone(located) as T;
    }
    case "pick_target_window": {
      const request = args?.request as { hwnd: number };
      const matched = mockWindows.find((window) => window.hwnd === request.hwnd) ?? mockWindows[0];
      mockConfig.target = {
        ...mockConfig.target,
        hwnd: matched.hwnd,
        titleContains: matched.title,
        className: matched.className,
        processPath: matched.exePath,
        processName: matched.exePath?.split("/").pop() ?? null
      };
      if (mockRuntimeProducesPreview(mockRuntime.status)) {
        mockRuntime.activeTarget = {
          window: matched,
          reliability: 95,
          reason: "hwnd 精确匹配"
        };
      }
      return clone(mockConfig.target) as T;
    }
    case "test_target_capture": {
      const request = args?.request as { hwnd?: number };
      const matched =
        mockWindows.find((window) => window.hwnd === (request?.hwnd ?? mockConfig.target.hwnd)) ??
        null;
      const result: TargetCaptureResult = {
        window: matched,
        preview: mockCapturePreview
      };
      return clone(result) as T;
    }
    case "get_runtime_status":
      updateMockRuntimePreview();
      return clone(mockRuntime) as T;
    case "start_runtime":
      mockRuntime.status = "Running";
      mockRuntime.metrics.runtime.status = "Running";
      mockRuntime.metrics.runtime.performance.captureFps = 29.8;
      mockRuntime.metrics.runtime.performance.frameIntervalMs = 33.5;
      mockRuntime.metrics.runtime.performance.detectLatencyMs = 7.2;
      mockRuntime.metrics.runtime.performance.previewLatencyMs = 1.4;
      mockRuntime.metrics.runtime.performance.endToEndLatencyMs = 8.9;
      mockRuntime.metrics.runtime.capture.frameWidth = 1280;
      mockRuntime.metrics.runtime.capture.frameHeight = 720;
      mockRuntime.activeTarget = {
        window: mockWindows[1],
        reliability: 95,
        reason: "hwnd 精确匹配"
      };
      updateMockRuntimePreview();
      return clone(mockRuntime) as T;
    case "stop_runtime":
      mockRuntime.status = "Idle";
      mockRuntime.metrics.runtime.status = "Idle";
      mockRuntime.metrics.runtime.capture.activeSource = null;
      resetMockRuntimePreview();
      return clone(mockRuntime) as T;
    case "restart_runtime":
      mockRuntime.status = "Running";
      mockRuntime.metrics.runtime.status = "Running";
      updateMockRuntimePreview();
      return clone(mockRuntime) as T;
    case "get_preview_snapshot":
      updateMockRuntimePreview();
      return clone(mockRuntime.preview) as T;
    case "get_diagnostics_overview":
      return {
        paths: clone(mockPaths),
        runtime: clone(mockRuntime),
        logs: clone(mockLogs)
      } as T;
    case "export_diagnostics_bundle":
      return `${mockPaths.cacheDir}/diagnostics/diagnostics-mock.zip` as T;
    case "dry_run_legacy_import_command":
      return {
        configImported: false,
        templatesImported: 0,
        warnings: [
          `mock 环境未读取真实旧工程，默认会使用 ${(args?.request as { legacyRoot?: string } | undefined)?.legacyRoot ?? "tests/fixtures/legacy_project"}。`
        ]
      } as T;
    case "run_legacy_import_command":
      return {
        configImported: true,
        templatesImported: mockTemplates.length,
        warnings: []
      } as T;
    default:
      throw new Error(`未实现的命令: ${command}`);
  }
};

export { mockInvoke };

