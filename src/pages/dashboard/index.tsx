import { useConfigStore } from "../../stores/configStore";
import { useRuntimeStore } from "../../stores/runtimeStore";
import { formatDecision, formatPercent, statusLabelMap } from "../../lib/presentation";

function MetricCard({ label, value, detail }: { label: string; value: string; detail: string }) {
  return (
    <article className="desk-metric">
      <p className="desk-field-label !mb-0">{label}</p>
      <p className="desk-metric-value">{value}</p>
      <p className="desk-metric-detail">{detail}</p>
    </article>
  );
}

export default function DashboardPage() {
  const snapshot = useRuntimeStore((state) => state.snapshot);
  const config = useConfigStore((state) => state.config);
  const locatedTarget = useConfigStore((state) => state.locatedTarget);
  const templates = useConfigStore((state) => state.templates);

  return (
    <section className="desk-page">
      <header className="desk-page-header">
        <div className="desk-page-header-main">
          <div>
            <p className="desk-eyebrow">Dashboard</p>
            <h1 className="desk-title">运行总览</h1>
          </div>
          <div className="desk-statline">
            <span className="flex items-center gap-2">
              <span className={`w-2 h-2 rounded-full ${snapshot?.status === "Running" ? "bg-success" : snapshot?.status === "Faulted" ? "bg-danger" : "bg-warning"}`} />
              {statusLabelMap[snapshot?.status ?? "Idle"]}
            </span>
            <span>决策: {formatDecision(snapshot?.decision)}</span>
            <span>模板: {templates.length}</span>
            <span>捕获源: {config?.capture.source ?? "--"}</span>
          </div>
        </div>
      </header>

      <div className="desk-page-body grid gap-6 xl:grid-cols-[1.5fr_1fr]">
        <div className="flex flex-col gap-6 min-h-0">
          <div className="grid gap-4 md:grid-cols-2 xl:grid-cols-4">
            <MetricCard
              label="捕获 FPS"
              value={(snapshot?.metrics.runtime.performance.captureFps ?? 0).toFixed(1)}
              detail={`帧间隔 ${(snapshot?.metrics.runtime.performance.frameIntervalMs ?? 0).toFixed(1)} ms`}
            />
            <MetricCard
              label="检测延迟"
              value={`${(snapshot?.metrics.runtime.performance.detectLatencyMs ?? 0).toFixed(1)} ms`}
              detail="匹配与命中判定"
            />
            <MetricCard
              label="端到端耗时"
              value={`${(snapshot?.metrics.runtime.performance.endToEndLatencyMs ?? 0).toFixed(1)} ms`}
              detail={`预览编码 ${(snapshot?.metrics.runtime.performance.previewLatencyMs ?? 0).toFixed(1)} ms`}
            />
            <MetricCard
              label="最近分数"
              value={formatPercent(snapshot?.metrics.runtime.performance.lastScore ?? 0)}
              detail={`恢复次数 ${snapshot?.metrics.recoveryCount ?? 0}`}
            />
          </div>

          <article className="desk-panel flex flex-col flex-1 min-h-0 p-6">
            <div className="flex items-center justify-between gap-3 mb-6">
              <h2 className="text-lg font-medium text-white/90">运行摘要</h2>
              <span className="desk-chip">{statusLabelMap[snapshot?.status ?? "Idle"]}</span>
            </div>
            <div className="grid gap-4 md:grid-cols-3">
              <div className="desk-field">
                <p className="desk-field-label">最近命中</p>
                <p className="mt-1 text-xl font-light text-white">
                  {snapshot?.bestMatch?.templateName ?? "暂无命中"}
                </p>
                <p className="mt-2 text-[13px] text-muted">
                  坐标 {snapshot?.bestMatch ? `${snapshot.bestMatch.x}, ${snapshot.bestMatch.y}` : "--"}
                </p>
              </div>
              <div className="desk-field">
                <p className="desk-field-label">点击结果</p>
                <p className="mt-1 text-xl font-light text-white">
                  {snapshot?.lastClick?.method ?? "暂无点击"}
                </p>
                <p className="mt-2 text-[13px] text-muted">
                  客户端 {snapshot?.lastClick ? `${snapshot.lastClick.clientX}, ${snapshot.lastClick.clientY}` : "--"}
                </p>
              </div>
              <div className="desk-field">
                <p className="desk-field-label">自动恢复</p>
                <p className="mt-1 text-xl font-light text-white">
                  {config?.recovery.enableAutoRecovery ? "开启" : "关闭"}
                </p>
                <p className="mt-2 text-[13px] text-muted">
                  上限 {config?.recovery.maxRecoveryAttempts ?? "--"} 次
                </p>
              </div>
            </div>
          </article>
        </div>

        <div className="flex flex-col min-h-0 gap-6">
          <article className="desk-panel flex flex-col p-6">
            <div className="flex items-center justify-between gap-3 mb-6">
              <h2 className="text-lg font-medium text-white/90">
                {locatedTarget?.window.title ?? "未定位窗口"}
              </h2>
              <span className="desk-chip">目标窗口</span>
            </div>
            <div className="grid gap-4">
              <div className="desk-field">
                <p className="desk-field-label">定位原因</p>
                <p className="text-[13px] text-white/80">
                  {locatedTarget?.reason ?? "尚未执行目标定位"}
                </p>
              </div>
              <div className="grid gap-4 md:grid-cols-2">
                <div className="desk-field">
                  <p className="desk-field-label">当前 ROI</p>
                  <p className="mt-1 text-xl font-light text-white">
                    {config ? `${config.capture.roi.width} × ${config.capture.roi.height}` : "--"}
                  </p>
                  <p className="mt-2 text-[13px] text-muted">
                    起点 {config ? `${config.capture.roi.x}, ${config.capture.roi.y}` : "--"}
                  </p>
                </div>
                <div className="desk-field">
                  <p className="desk-field-label">输入策略</p>
                  <p className="mt-1 text-xl font-light text-white">{config?.input.method ?? "--"}</p>
                  <p className="mt-2 text-[13px] text-muted">
                    窗口校验 {config?.input.verifyWindowBeforeClick ? "开启" : "关闭"}
                  </p>
                </div>
              </div>
            </div>
          </article>

          <article className="desk-panel flex min-h-0 flex-col flex-1 p-6">
            <div className="flex items-center justify-between gap-3 mb-6">
              <h2 className="text-lg font-medium text-white/90">最近问题</h2>
              <span className="desk-chip">缓冲 {snapshot?.metrics.bufferDrops ?? 0}</span>
            </div>
            <div className="desk-scroll flex-1">
              <div className="rounded-lg border border-danger/20 bg-danger/10 p-4">
                <p className="text-[13px] leading-relaxed text-danger">
                  {snapshot?.lastError ?? snapshot?.metrics.runtime.lastError ?? "当前没有运行时错误。"}
                </p>
              </div>
              <div className="mt-4 desk-field">
                <p className="desk-field-label">运行链路</p>
                <div className="mt-2 flex flex-col gap-1.5 text-[13px] text-muted">
                  <p>决策: {formatDecision(snapshot?.decision)}</p>
                  <p>恢复次数: {snapshot?.metrics.recoveryCount ?? 0}</p>
                  <p>内存估算: {((snapshot?.metrics.memoryBytesEstimate ?? 0) / 1024 / 1024).toFixed(1)} MB</p>
                </div>
              </div>
            </div>
          </article>
        </div>
      </div>
    </section>
  );
}

