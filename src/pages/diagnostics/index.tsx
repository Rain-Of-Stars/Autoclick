import { useMemo, useState } from "react";

import { formatBytes, statusLabelMap } from "../../lib/presentation";
import { useConfigStore } from "../../stores/configStore";
import { useRuntimeStore } from "../../stores/runtimeStore";

type AlertLevel = "critical" | "warning" | "info";

interface AlertItem {
  id: string;
  level: AlertLevel;
  title: string;
  detail: string;
  hint: string;
}

function levelText(level: AlertLevel) {
  return {
    critical: "Critical",
    warning: "Warning",
    info: "Info"
  }[level];
}

function levelClass(level: AlertLevel) {
  return {
    critical: "desk-alert desk-alert-critical",
    warning: "desk-alert desk-alert-warning",
    info: "desk-alert desk-alert-info"
  }[level];
}

export default function DiagnosticsPage() {
  const diagnostics = useConfigStore((state) => state.diagnostics);
  const refreshDiagnostics = useConfigStore((state) => state.refreshDiagnostics);
  const exportDiagnostics = useConfigStore((state) => state.exportDiagnostics);
  const importReport = useConfigStore((state) => state.importReport);
  const dryRunLegacyImport = useConfigStore((state) => state.dryRunLegacyImport);
  const runLegacyImport = useConfigStore((state) => state.runLegacyImport);
  const runtimeSnapshot = useRuntimeStore((state) => state.snapshot);
  const [legacyRoot, setLegacyRoot] = useState("tests/fixtures/legacy_project");
  const [logKeyword, setLogKeyword] = useState("");
  const [logScale, setLogScale] = useState("all");
  const [bundlePath, setBundlePath] = useState<string | null>(null);

  const activeRuntime = runtimeSnapshot ?? diagnostics?.runtime ?? null;
  const logs = diagnostics?.logs ?? [];
  const totalLogBytes = useMemo(
    () => logs.reduce((sum, log) => sum + log.sizeBytes, 0),
    [logs]
  );

  const alerts = useMemo<AlertItem[]>(() => {
    const items: AlertItem[] = [];
    const lastError = activeRuntime?.lastError ?? activeRuntime?.metrics.runtime.lastError ?? null;
    if (lastError) {
      items.push({
        id: "runtime-error",
        level: "critical",
        title: "运行时故障",
        detail: lastError,
        hint: "先检查目标窗口、模板库和捕获链路，再看恢复计数是否持续增长。"
      });
    }
    if (activeRuntime?.status === "Faulted") {
      items.push({
        id: "runtime-faulted",
        level: "critical",
        title: "状态机处于故障态",
        detail: "扫描主链路已经脱离稳定运行区间，当前预览与点击决策都不可信。",
        hint: "优先执行重新链路，并保留当前日志包。"
      });
    }
    if ((activeRuntime?.metrics.bufferDrops ?? 0) > 0) {
      items.push({
        id: "buffer-drops",
        level: "warning",
        title: "发现缓冲丢帧",
        detail: `累计丢帧 ${activeRuntime?.metrics.bufferDrops ?? 0} 次，说明捕获或预览消费存在背压。`,
        hint: "降低目标帧率或缩小ROI，优先排查高频预览与图像编码负载。"
      });
    }
    if ((activeRuntime?.metrics.recoveryCount ?? 0) > 0) {
      items.push({
        id: "recovery-count",
        level: "warning",
        title: "恢复链路已触发",
        detail: `当前会话已恢复 ${(activeRuntime?.metrics.recoveryCount ?? 0).toString()} 次。`,
        hint: "目标窗口稳定性不足时，应优先校验窗口可见性和自动恢复配置。"
      });
    }
    if ((activeRuntime?.metrics.runtime.performance.endToEndLatencyMs ?? 0) >= 50) {
      items.push({
        id: "latency-high",
        level: "warning",
        title: "端到端时延偏高",
        detail: `当前端到端耗时 ${(activeRuntime?.metrics.runtime.performance.endToEndLatencyMs ?? 0).toFixed(1)} ms。`,
        hint: "优先确认检测阈值、多尺度和预览编码是否过重。"
      });
    }
    if (items.length === 0) {
      items.push({
        id: "stable",
        level: "info",
        title: "当前无高优先级异常",
        detail: "运行态指标处于可接受范围，诊断面板未发现立即阻断扫描的错误。",
        hint: "如要继续排查性能，可优先观察帧间隔和预览编码耗时。"
      });
    }
    return items;
  }, [activeRuntime]);

  const severityCounts = useMemo(
    () => ({
      critical: alerts.filter((item) => item.level === "critical").length,
      warning: alerts.filter((item) => item.level === "warning").length,
      info: alerts.filter((item) => item.level === "info").length
    }),
    [alerts]
  );

  const filteredLogs = useMemo(() => {
    const normalizedKeyword = logKeyword.trim().toLowerCase();
    return logs.filter((log) => {
      const matchesKeyword =
        !normalizedKeyword ||
        `${log.name} ${log.path}`.toLowerCase().includes(normalizedKeyword);
      const matchesScale =
        logScale === "all" ||
        (logScale === "large" && log.sizeBytes >= 1024 * 1024) ||
        (logScale === "small" && log.sizeBytes < 1024 * 1024);
      return matchesKeyword && matchesScale;
    });
  }, [logKeyword, logScale, logs]);

  const logTableGridClass = "grid grid-cols-[minmax(0,170px)_84px_minmax(0,1fr)] gap-3";

  return (
    <section className="desk-page">
      <header className="desk-page-header">
        <div className="desk-page-header-main">
          <div className="min-w-0">
            <p className="desk-eyebrow">Diagnostics</p>
            <div className="mt-2 flex flex-wrap items-center gap-3">
              <h1 className="desk-title">日志与诊断</h1>
              <span className="desk-chip">Critical {severityCounts.critical}</span>
              <span className="desk-chip">Warning {severityCounts.warning}</span>
              <span className="desk-chip">Info {severityCounts.info}</span>
            </div>
          </div>
          <div className="desk-toolbar justify-end">
            <button type="button" className="desk-button desk-button-neutral" onClick={() => void refreshDiagnostics()}>
              刷新
            </button>
            <button
              type="button"
              className="desk-button desk-button-primary"
              onClick={async () => {
                const path = await exportDiagnostics();
                setBundlePath(path);
              }}
            >
              导出诊断包
            </button>
          </div>
        </div>
        <div className="desk-statline shrink-0">
          <span>状态: {statusLabelMap[activeRuntime?.status ?? "Idle"]}</span>
          <span>日志占用: {formatBytes(totalLogBytes)}</span>
          <span>最近导出: {bundlePath ?? "尚未导出"}</span>
        </div>
      </header>

      <div className="desk-page-body grid gap-4 xl:grid-cols-[280px_minmax(0,0.9fr)_minmax(380px,1.1fr)]">
        <article className="desk-panel flex min-h-0 min-w-0 flex-col p-4">
          <div className="shrink-0 border-b border-white/10 pb-3">
            <p className="desk-eyebrow">Alert Queue</p>
            <h2 className="mt-2 text-lg font-semibold text-slate-100">告警队列</h2>
          </div>
          <div className="desk-scroll mt-4 space-y-3">
            {alerts.map((alert) => (
              <div key={alert.id} className={levelClass(alert.level)}>
                <div className="flex flex-wrap items-start justify-between gap-3">
                  <div>
                    <p className="text-[10px] uppercase tracking-[0.24em] text-current/80">
                      {levelText(alert.level)}
                    </p>
                    <p className="mt-2 text-base font-semibold text-slate-100">{alert.title}</p>
                  </div>
                  <span className="desk-badge border-current/40 text-current">{alert.id}</span>
                </div>
                <p className="mt-2 text-sm leading-6 text-slate-200">{alert.detail}</p>
                <p className="mt-2 text-[11px] leading-5 text-slate-300/90">{alert.hint}</p>
              </div>
            ))}
          </div>
        </article>

        <div className="grid min-h-0 min-w-0 gap-4 xl:grid-rows-[minmax(0,1fr)_280px]">
          <article className="desk-panel flex min-h-0 min-w-0 flex-col p-4">
            <div className="shrink-0 border-b border-white/10 pb-3">
              <p className="desk-eyebrow">Runtime Pipeline</p>
              <h2 className="mt-2 text-lg font-semibold text-slate-100">链路指标</h2>
            </div>
            <div className="desk-scroll mt-4 space-y-3">
              <div className="grid gap-2 text-[11px] text-slate-400 xl:grid-cols-2">
                <div className="border border-white/10 bg-black/10 px-3 py-2">
                  <p className="desk-field-label">帧间隔</p>
                  <p className="mt-1 text-sm font-semibold text-slate-100">
                    {(activeRuntime?.metrics.runtime.performance.frameIntervalMs ?? 0).toFixed(1)} ms
                  </p>
                </div>
                <div className="border border-white/10 bg-black/10 px-3 py-2">
                  <p className="desk-field-label">端到端</p>
                  <p className="mt-1 text-sm font-semibold text-slate-100">
                    {(activeRuntime?.metrics.runtime.performance.endToEndLatencyMs ?? 0).toFixed(1)} ms
                  </p>
                </div>
                <div className="border border-white/10 bg-black/10 px-3 py-2">
                  <p className="desk-field-label">检测耗时</p>
                  <p className="mt-1 text-sm font-semibold text-slate-100">
                    {(activeRuntime?.metrics.runtime.performance.detectLatencyMs ?? 0).toFixed(1)} ms
                  </p>
                </div>
                <div className="border border-white/10 bg-black/10 px-3 py-2">
                  <p className="desk-field-label">预览编码</p>
                  <p className="mt-1 text-sm font-semibold text-slate-100">
                    {(activeRuntime?.metrics.runtime.performance.previewLatencyMs ?? 0).toFixed(1)} ms
                  </p>
                </div>
                <div className="border border-white/10 bg-black/10 px-3 py-2">
                  <p className="desk-field-label">恢复次数</p>
                  <p className="mt-1 text-sm font-semibold text-slate-100">
                    {activeRuntime?.metrics.recoveryCount ?? 0}
                  </p>
                </div>
                <div className="border border-white/10 bg-black/10 px-3 py-2">
                  <p className="desk-field-label">缓冲丢帧</p>
                  <p className="mt-1 text-sm font-semibold text-slate-100">
                    {activeRuntime?.metrics.bufferDrops ?? 0}
                  </p>
                </div>
              </div>
              <div className="desk-field">
                <p className="desk-field-label">当前目标</p>
                <p className="mt-2 truncate text-sm leading-6 text-slate-100" title={activeRuntime?.activeTarget?.window.title ?? "未定位"}>
                  {activeRuntime?.activeTarget?.window.title ?? "未定位"}
                </p>
                <p className="mt-1 truncate text-[11px] leading-5 text-slate-500" title={activeRuntime?.activeTarget?.reason ?? "尚未生成定位摘要。"}>
                  {activeRuntime?.activeTarget?.reason ?? "尚未生成定位摘要。"}
                </p>
              </div>
              <div className="desk-field">
                <p className="desk-field-label">诊断路径</p>
                <div className="mt-2 space-y-1.5 text-[11px] leading-5 text-slate-400">
                  <p className="truncate" title={diagnostics?.paths.logDir ?? "--"}>日志目录: {diagnostics?.paths.logDir ?? "--"}</p>
                  <p className="truncate" title={diagnostics?.paths.cacheDir ?? "--"}>缓存目录: {diagnostics?.paths.cacheDir ?? "--"}</p>
                  <p className="truncate" title={diagnostics?.paths.dbPath ?? "--"}>数据库: {diagnostics?.paths.dbPath ?? "--"}</p>
                </div>
              </div>
            </div>
          </article>

          <article className="desk-panel flex min-h-0 min-w-0 flex-col p-4">
            <div className="shrink-0 border-b border-white/10 pb-3">
              <p className="desk-eyebrow">Legacy Import</p>
              <h2 className="mt-2 text-lg font-semibold text-slate-100">导入旧工程</h2>
            </div>
            <div className="desk-scroll mt-4 space-y-3">
              <input
                className="desk-input"
                value={legacyRoot}
                onChange={(event) => setLegacyRoot(event.target.value)}
                placeholder="输入旧工程目录，例如 legacy-project"
              />
              <div className="flex flex-wrap gap-2">
                <button type="button" className="desk-button desk-button-neutral" onClick={() => void dryRunLegacyImport(legacyRoot)}>
                  Dry Run
                </button>
                <button type="button" className="desk-button desk-button-warning" onClick={() => void runLegacyImport(legacyRoot)}>
                  执行导入
                </button>
              </div>
              <div className="desk-field">
                <p className="desk-field-label">导入摘要</p>
                <div className="mt-2 grid gap-2 text-xs leading-6 text-slate-300">
                  <p>配置导入: {importReport?.configImported ? "已导入" : "未执行"}</p>
                  <p>模板导入: {importReport?.templatesImported ?? 0}</p>
                </div>
              </div>
              <div className="desk-field">
                <p className="desk-field-label">导入警告</p>
                <div className="mt-2 space-y-2 text-xs leading-6">
                  {(importReport?.warnings ?? []).length > 0 ? (
                    (importReport?.warnings ?? []).map((warning) => (
                      <p key={warning} className="text-warning">
                        {warning}
                      </p>
                    ))
                  ) : (
                    <p className="text-slate-500">当前没有导入告警。</p>
                  )}
                </div>
              </div>
            </div>
          </article>
        </div>

        <article className="desk-panel-strong flex min-h-0 min-w-0 flex-col p-4">
          <div className="shrink-0 border-b border-white/10 pb-3">
            <div className="flex flex-wrap items-end justify-between gap-3">
              <div className="min-w-0">
                <p className="desk-eyebrow">Log Files</p>
                <h2 className="mt-2 text-lg font-semibold text-slate-100">日志文件</h2>
              </div>
              <div className="grid min-w-0 gap-2 md:grid-cols-[minmax(0,1fr)_120px]">
                <input
                  className="desk-input"
                  placeholder="按文件名或路径筛选"
                  value={logKeyword}
                  onChange={(event) => setLogKeyword(event.target.value)}
                />
                <select
                  className="desk-select"
                  value={logScale}
                  onChange={(event) => setLogScale(event.target.value)}
                >
                  <option value="all">全部体量</option>
                  <option value="large">大于1 MB</option>
                  <option value="small">小于1 MB</option>
                </select>
              </div>
            </div>
          </div>

          <div className="desk-table mt-4 min-h-0 flex-1 overflow-hidden">
            <div className={`desk-table-head ${logTableGridClass} px-3 py-2`}>
              <span>文件</span>
              <span>大小</span>
              <span>路径</span>
            </div>
            <div className="desk-scroll flex-1 overflow-x-hidden">
              {filteredLogs.map((log) => (
                <div
                  key={log.path}
                  className={`desk-table-row ${logTableGridClass} px-3 py-3 text-xs text-slate-400`}
                >
                  <div className="min-w-0">
                    <p className="break-words font-medium text-slate-100">{log.name}</p>
                    <p className="mt-1 text-[10px] text-slate-500">
                      {log.sizeBytes >= 1024 * 1024 ? "Large" : "Standard"}
                    </p>
                  </div>
                  <span>{formatBytes(log.sizeBytes)}</span>
                  <span className="truncate" title={log.path}>{log.path}</span>
                </div>
              ))}
              {filteredLogs.length === 0 ? (
                <div className="flex h-full min-h-[220px] items-center justify-center px-4 text-sm text-slate-500">
                  当前筛选条件下没有日志文件。
                </div>
              ) : null}
            </div>
          </div>
        </article>
      </div>
    </section>
  );
}
