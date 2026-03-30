import type {
  AppConfig,
  AppPaths,
  BootstrapInfo,
  DiagnosticsOverview,
  LegacyImportReport,
  LocatorCandidate,
  MonitorInfo,
  PreviewMessage,
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

const isTauriRuntime = () =>
  typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;

const invokeCommand = async <T>(command: string, args?: InvokeArgs): Promise<T> => {
  if (!isTauriRuntime()) {
    const { mockInvoke } = await import("./tauriClient.mock");
    return mockInvoke<T>(command, args);
  }
  const { invoke } = await import("@tauri-apps/api/core");
  return invoke<T>(command, args);
};

export const tauriClient = {
  getUpdaterStatus: () => invokeCommand<UpdaterStatus>("get_updater_status"),
  checkForUpdates: () => invokeCommand<UpdateCheckResult>("check_for_updates"),
  getBootstrapInfo: () => invokeCommand<BootstrapInfo>("get_bootstrap_info"),
  listWorkspaceModules: () => invokeCommand<string[]>("list_workspace_modules"),
  getConfig: () => invokeCommand<AppConfig>("get_config"),
  saveConfig: (config: AppConfig) => invokeCommand<AppConfig>("save_config", { config }),
  getAppPaths: () => invokeCommand<AppPaths>("get_app_paths"),
  listTemplates: () => invokeCommand<TemplateRef[]>("list_templates"),
  getTemplatePreview: (templateId: string) =>
    invokeCommand<TemplatePreview>("get_template_preview", { request: { templateId } }),
  importTemplate: (filePath: string, tags: string[]) =>
    invokeCommand<TemplateRef>("import_template", { request: { filePath, tags } }),
  importPastedTemplate: (name: string, tags: string[], bytes: number[]) =>
    invokeCommand<TemplateRef>("import_pasted_template", {
      request: { name, tags, bytes }
    }),
  removeTemplate: (templateId: string) =>
    invokeCommand<TemplateRef[]>("remove_template", { templateId }),
  renameTemplate: (templateId: string, name: string) =>
    invokeCommand<TemplateRef>("rename_template", { request: { templateId, name } }),
  listTargetWindows: () => invokeCommand<WindowInfo[]>("list_target_windows"),
  listMonitors: () => invokeCommand<MonitorInfo[]>("list_monitors"),
  locateTarget: () => invokeCommand<LocatorCandidate | null>("locate_target"),
  pickTargetWindow: (hwnd: number) =>
    invokeCommand<TargetProfile>("pick_target_window", { request: { hwnd } }),
  testTargetCapture: (hwnd?: number) =>
    invokeCommand<TargetCaptureResult>("test_target_capture", { request: { hwnd } }),
  getRuntimeStatus: () => invokeCommand<RuntimeControllerSnapshot>("get_runtime_status"),
  startRuntime: () => invokeCommand<RuntimeControllerSnapshot>("start_runtime"),
  stopRuntime: () => invokeCommand<RuntimeControllerSnapshot>("stop_runtime"),
  restartRuntime: () => invokeCommand<RuntimeControllerSnapshot>("restart_runtime"),
  getPreviewSnapshot: () => invokeCommand<PreviewMessage | null>("get_preview_snapshot"),
  getDiagnosticsOverview: () => invokeCommand<DiagnosticsOverview>("get_diagnostics_overview"),
  exportDiagnosticsBundle: () => invokeCommand<string>("export_diagnostics_bundle"),
  dryRunLegacyImport: (legacyRoot?: string) =>
    invokeCommand<LegacyImportReport>("dry_run_legacy_import_command", {
      request: { legacyRoot }
    }),
  runLegacyImport: (legacyRoot?: string) =>
    invokeCommand<LegacyImportReport>("run_legacy_import_command", {
      request: { legacyRoot }
    })
};
