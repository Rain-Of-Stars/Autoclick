import { create } from "zustand";
import type {
  AppConfig,
  AppPaths,
  BootstrapInfo,
  DiagnosticsOverview,
  LegacyImportReport,
  LocatorCandidate,
  MonitorInfo,
  TargetCaptureResult,
  TemplateRef,
  WindowInfo
} from "../lib/contracts";
import { tauriClient } from "../lib/tauriClient";

interface ConfigStoreState {
  config: AppConfig | null;
  paths: AppPaths | null;
  bootstrap: BootstrapInfo | null;
  modules: string[];
  templates: TemplateRef[];
  windows: WindowInfo[];
  monitors: MonitorInfo[];
  diagnostics: DiagnosticsOverview | null;
  locatedTarget: LocatorCandidate | null;
  targetCapture: TargetCaptureResult | null;
  importReport: LegacyImportReport | null;
  loading: boolean;
  saving: boolean;
  error: string | null;
  loadInitial: () => Promise<void>;
  saveConfig: (config: AppConfig) => Promise<void>;
  refreshTemplates: () => Promise<void>;
  refreshWindows: () => Promise<void>;
  refreshDiagnostics: () => Promise<void>;
  importTemplate: (filePath: string, tags: string[]) => Promise<void>;
  importPastedTemplate: (
    name: string,
    tags: string[],
    bytes: number[]
  ) => Promise<TemplateRef | null>;
  removeTemplate: (templateId: string) => Promise<void>;
  renameTemplate: (templateId: string, name: string) => Promise<void>;
  pickTargetWindow: (hwnd: number) => Promise<void>;
  locateTarget: () => Promise<void>;
  testTargetCapture: (hwnd?: number) => Promise<void>;
  exportDiagnostics: () => Promise<string | null>;
  dryRunLegacyImport: (legacyRoot?: string) => Promise<void>;
  runLegacyImport: (legacyRoot?: string) => Promise<void>;
  clearError: () => void;
}

export const useConfigStore = create<ConfigStoreState>((set, get) => ({
  config: null,
  paths: null,
  bootstrap: null,
  modules: [],
  templates: [],
  windows: [],
  monitors: [],
  diagnostics: null,
  locatedTarget: null,
  targetCapture: null,
  importReport: null,
  loading: false,
  saving: false,
  error: null,
  loadInitial: async () => {
    set({ loading: true, error: null });
    try {
      const [config, paths, bootstrap, modules, templates, windows, monitors, diagnostics] =
        await Promise.all([
          tauriClient.getConfig(),
          tauriClient.getAppPaths(),
          tauriClient.getBootstrapInfo(),
          tauriClient.listWorkspaceModules(),
          tauriClient.listTemplates(),
          tauriClient.listTargetWindows(),
          tauriClient.listMonitors(),
          tauriClient.getDiagnosticsOverview()
        ]);
      set({
        config,
        paths,
        bootstrap,
        modules,
        templates,
        windows,
        monitors,
        diagnostics,
        locatedTarget: diagnostics.runtime.activeTarget,
        loading: false
      });
    } catch (error) {
      set({
        loading: false,
        error: error instanceof Error ? error.message : "加载配置失败"
      });
    }
  },
  saveConfig: async (config) => {
    set({ saving: true, error: null });
    try {
      const saved = await tauriClient.saveConfig(config);
      set({ config: saved, saving: false });
    } catch (error) {
      set({
        saving: false,
        error: error instanceof Error ? error.message : "保存配置失败"
      });
    }
  },
  refreshTemplates: async () => {
    try {
      const templates = await tauriClient.listTemplates();
      set({ templates });
    } catch (error) {
      set({ error: error instanceof Error ? error.message : "读取模板失败" });
    }
  },
  refreshWindows: async () => {
    try {
      const [windows, monitors] = await Promise.all([
        tauriClient.listTargetWindows(),
        tauriClient.listMonitors()
      ]);
      set({ windows, monitors });
    } catch (error) {
      set({ error: error instanceof Error ? error.message : "读取窗口失败" });
    }
  },
  refreshDiagnostics: async () => {
    try {
      const diagnostics = await tauriClient.getDiagnosticsOverview();
      set({
        diagnostics,
        locatedTarget: diagnostics.runtime.activeTarget
      });
    } catch (error) {
      set({ error: error instanceof Error ? error.message : "读取诊断失败" });
    }
  },
  importTemplate: async (filePath, tags) => {
    set({ saving: true, error: null });
    try {
      await tauriClient.importTemplate(filePath, tags);
      const templates = await tauriClient.listTemplates();
      set({ templates, saving: false });
    } catch (error) {
      set({
        saving: false,
        error: error instanceof Error ? error.message : "导入模板失败"
      });
    }
  },
  importPastedTemplate: async (name, tags, bytes) => {
    set({ saving: true, error: null });
    try {
      const created = await tauriClient.importPastedTemplate(name, tags, bytes);
      const templates = await tauriClient.listTemplates();
      set({ templates, saving: false });
      return created;
    } catch (error) {
      set({
        saving: false,
        error: error instanceof Error ? error.message : "粘贴模板失败"
      });
      return null;
    }
  },
  removeTemplate: async (templateId) => {
    set({ saving: true, error: null });
    try {
      const templates = await tauriClient.removeTemplate(templateId);
      set({ templates, saving: false });
    } catch (error) {
      set({
        saving: false,
        error: error instanceof Error ? error.message : "删除模板失败"
      });
    }
  },
  renameTemplate: async (templateId, name) => {
    set({ saving: true, error: null });
    try {
      await tauriClient.renameTemplate(templateId, name);
      const templates = await tauriClient.listTemplates();
      set({ templates, saving: false });
    } catch (error) {
      set({
        saving: false,
        error: error instanceof Error ? error.message : "重命名模板失败"
      });
    }
  },
  pickTargetWindow: async (hwnd) => {
    set({ saving: true, error: null });
    try {
      const target = await tauriClient.pickTargetWindow(hwnd);
      set((state) => ({
        saving: false,
        config: state.config ? { ...state.config, target } : state.config
      }));
      await get().locateTarget();
    } catch (error) {
      set({
        saving: false,
        error: error instanceof Error ? error.message : "选择目标窗口失败"
      });
    }
  },
  locateTarget: async () => {
    try {
      const locatedTarget = await tauriClient.locateTarget();
      set({ locatedTarget });
    } catch (error) {
      set({ error: error instanceof Error ? error.message : "定位目标窗口失败" });
    }
  },
  testTargetCapture: async (hwnd) => {
    set({ saving: true, error: null });
    try {
      const targetCapture = await tauriClient.testTargetCapture(hwnd);
      set({ targetCapture, saving: false });
    } catch (error) {
      set({
        saving: false,
        error: error instanceof Error ? error.message : "测试捕获失败"
      });
    }
  },
  exportDiagnostics: async () => {
    try {
      const path = await tauriClient.exportDiagnosticsBundle();
      await get().refreshDiagnostics();
      return path;
    } catch (error) {
      set({ error: error instanceof Error ? error.message : "导出诊断失败" });
      return null;
    }
  },
  dryRunLegacyImport: async (legacyRoot) => {
    try {
      const importReport = await tauriClient.dryRunLegacyImport(legacyRoot);
      set({ importReport });
    } catch (error) {
      set({ error: error instanceof Error ? error.message : "旧数据 dry-run 失败" });
    }
  },
  runLegacyImport: async (legacyRoot) => {
    set({ saving: true, error: null });
    try {
      const importReport = await tauriClient.runLegacyImport(legacyRoot);
      set({ importReport, saving: false });
      await get().loadInitial();
    } catch (error) {
      set({
        saving: false,
        error: error instanceof Error ? error.message : "旧数据导入失败"
      });
    }
  },
  clearError: () => set({ error: null })
}));
