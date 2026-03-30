import { useEffect, useMemo, useState } from "react";
import type { AppConfig, UpdateCheckResult, UpdaterStatus } from "../../lib/contracts";
import { tauriClient } from "../../lib/tauriClient";
import { useConfigStore } from "../../stores/configStore";

export default function SystemSettingsPage() {
  const config = useConfigStore((state) => state.config);
  const paths = useConfigStore((state) => state.paths);
  const bootstrap = useConfigStore((state) => state.bootstrap);
  const modules = useConfigStore((state) => state.modules);
  const saveConfig = useConfigStore((state) => state.saveConfig);
  const [draft, setDraft] = useState<AppConfig | null>(null);
  const [updaterStatus, setUpdaterStatus] = useState<UpdaterStatus | null>(null);
  const [updateResult, setUpdateResult] = useState<UpdateCheckResult | null>(null);
  const [checkingUpdate, setCheckingUpdate] = useState(false);

  useEffect(() => {
    if (config) {
      setDraft(config);
    }
  }, [config]);

  useEffect(() => {
    void tauriClient
      .getUpdaterStatus()
      .then((status) => setUpdaterStatus(status))
      .catch(() => setUpdaterStatus(null));
  }, []);

  const toggleItems = useMemo(
    () => [
      ["enableLogging", "启用日志"],
      ["enableNotifications", "启用通知"],
      ["autoStartScan", "启动后自动扫描"],
      ["debugMode", "调试模式"],
      ["saveDebugImages", "保存调试图"]
    ] as const,
    []
  );

  if (!draft) {
    return <div className="desk-panel p-6 text-slate-400">加载中…</div>;
  }

  const updateHint =
    updateResult?.reason ??
    updaterStatus?.reason ??
    "更新功能处于占位状态，生产环境通过环境变量注入更新地址和公钥。";

  return (
    <section className="desk-page">
      <header className="desk-page-header">
        <div className="desk-page-header-main">
          <div className="min-w-0">
            <p className="desk-eyebrow">System Settings</p>
            <div className="mt-2 flex flex-wrap items-center gap-3">
              <h1 className="desk-title">系统设置</h1>
              <span className="desk-chip">版本 {bootstrap?.version ?? "--"}</span>
              <span className="desk-chip">模块 {modules.length}</span>
              <span className="desk-chip">更新 {updaterStatus?.configured ? "已配置" : "占位"}</span>
            </div>
          </div>
          <div className="desk-toolbar justify-end">
            <button
              type="button"
              className="desk-button desk-button-primary"
              onClick={() => void saveConfig(draft)}
            >
              保存系统设置
            </button>
          </div>
        </div>
        <div className="desk-statline shrink-0 break-all select-text">
          <span>应用名: {bootstrap?.appName ?? "--"}</span>
          <span>运行路径: {bootstrap?.runtimePath ?? "--"}</span>
          <span>日志目录: {paths?.logDir ?? "--"}</span>
        </div>
      </header>

      <div className="desk-page-body grid gap-4 xl:grid-cols-[320px_minmax(0,1fr)_320px]">
        <article className="desk-panel flex min-h-0 flex-col p-4">
          <div className="shrink-0 border-b border-white/10 pb-3">
            <p className="desk-eyebrow">UI Preferences</p>
            <h2 className="mt-2 text-lg font-semibold text-slate-100">界面与调试偏好</h2>
          </div>
          <div className="desk-scroll mt-4 space-y-3">
            {toggleItems.map(([key, label]) => (
              <label
                key={key}
                className="flex items-center justify-between border border-white/10 bg-black/10 px-3 py-2.5"
              >
                <span className="text-sm text-slate-100">{label}</span>
                <input
                  className="h-4 w-4 accent-[rgb(70,128,186)]"
                  type="checkbox"
                  checked={draft.ui[key as keyof AppConfig["ui"]] as boolean}
                  onChange={(event) =>
                    setDraft({
                      ...draft,
                      ui: {
                        ...draft.ui,
                        [key]: event.target.checked
                      }
                    })
                  }
                />
              </label>
            ))}
            <div className="desk-field">
              <p className="desk-field-label">调试图目录</p>
              <input
                className="desk-input mt-2"
                placeholder="例如 debug_images/session-1"
                value={draft.ui.debugImageDir}
                onChange={(event) =>
                  setDraft({
                    ...draft,
                    ui: { ...draft.ui, debugImageDir: event.target.value }
                  })
                }
              />
              <p className="mt-2 text-[11px] leading-5 text-slate-500">
                仅允许应用数据目录下的相对路径，不支持绝对路径或上级目录。
              </p>
            </div>
          </div>
        </article>

        <article className="desk-panel-strong flex min-h-0 flex-col p-4">
          <div className="shrink-0 border-b border-white/10 pb-3">
            <p className="desk-eyebrow">Environment</p>
            <h2 className="mt-2 text-lg font-semibold text-slate-100">环境与路径</h2>
          </div>
          <div className="desk-scroll mt-4 space-y-3">
            <div className="grid gap-2 text-[11px] text-slate-400 xl:grid-cols-3">
              <div className="border border-white/10 bg-black/10 px-3 py-2">
                <p className="desk-field-label">版本</p>
                <p className="mt-1 text-sm font-semibold text-slate-100">{bootstrap?.version ?? "--"}</p>
              </div>
              <div className="border border-white/10 bg-black/10 px-3 py-2">
                <p className="desk-field-label">模块数</p>
                <p className="mt-1 text-sm font-semibold text-slate-100">{modules.length}</p>
              </div>
              <div className="border border-white/10 bg-black/10 px-3 py-2">
                <p className="desk-field-label">自动扫描</p>
                <p className="mt-1 text-sm font-semibold text-slate-100">
                  {draft.ui.autoStartScan ? "开启" : "关闭"}
                </p>
              </div>
            </div>

            <div className="desk-field flex flex-col overflow-hidden">
              <p className="desk-field-label shrink-0">运行信息</p>
              <div className="mt-2 flex flex-col gap-2 text-xs leading-6 text-slate-300 break-all select-text">
                <p>应用名: {bootstrap?.appName ?? "--"}</p>
                <p>运行路径: {bootstrap?.runtimePath ?? "--"}</p>
                <p>数据目录: {paths?.dataDir ?? "--"}</p>
                <p>缓存目录: {paths?.cacheDir ?? "--"}</p>
                <p>日志目录: {paths?.logDir ?? "--"}</p>
                <p>模板目录: {paths?.templatesDir ?? "--"}</p>
                <p>数据库: {paths?.dbPath ?? "--"}</p>
              </div>
            </div>

            <div className="desk-field">
              <p className="desk-field-label">Workspace Modules</p>
              <div className="mt-2 flex flex-wrap gap-1.5">
                {modules.map((item) => (
                  <span key={item} className="desk-chip h-5 px-2 text-[9px]">
                    {item}
                  </span>
                ))}
              </div>
            </div>
          </div>
        </article>

        <aside className="desk-panel flex min-h-0 flex-col p-4">
          <div className="shrink-0 border-b border-white/10 pb-3">
            <p className="desk-eyebrow">Updater</p>
            <h2 className="mt-2 text-lg font-semibold text-slate-100">更新与发布通道</h2>
          </div>
          <div className="desk-scroll mt-4 space-y-3">
            <div className="desk-field">
              <p className="desk-field-label">配置状态</p>
              <div className="mt-2 grid gap-2 text-xs leading-6 text-slate-300">
                <p>安装模式: {updaterStatus?.installMode ?? "passive"}</p>
                <p>更新状态: {updaterStatus?.configured ? "已配置" : "占位"}</p>
                <p>公钥状态: {updaterStatus?.pubkeyConfigured ? "已配置" : "占位"}</p>
              </div>
            </div>

            <div className="desk-field">
              <div className="flex flex-wrap items-center justify-between gap-3">
                <div>
                  <p className="desk-field-label">检查更新</p>
                  <p className="mt-1 text-[11px] leading-5 text-slate-500">{updateHint}</p>
                </div>
                <button
                  type="button"
                  className="desk-button desk-button-neutral disabled:cursor-not-allowed disabled:opacity-50"
                  disabled={checkingUpdate}
                  onClick={() => {
                    setCheckingUpdate(true);
                    void tauriClient
                      .checkForUpdates()
                      .then((result) => setUpdateResult(result))
                      .catch((error) =>
                        setUpdateResult({
                          configured: false,
                          checked: false,
                          updateAvailable: false,
                          currentVersion: bootstrap?.version ?? "6.0.0",
                          latestVersion: null,
                          body: null,
                          date: null,
                          reason: error instanceof Error ? error.message : "检查更新失败"
                        })
                      )
                      .finally(() => setCheckingUpdate(false));
                  }}
                >
                  {checkingUpdate ? "检查中…" : "检查更新"}
                </button>
              </div>
              <div className="mt-3 grid gap-2 text-xs leading-6 text-slate-300">
                <p>当前版本: {updateResult?.currentVersion ?? bootstrap?.version ?? "--"}</p>
                <p>最新版本: {updateResult?.latestVersion ?? "--"}</p>
                <p>是否有更新: {updateResult?.updateAvailable ? "是" : "否"}</p>
              </div>
            </div>
          </div>
        </aside>
      </div>
    </section>
  );
}
