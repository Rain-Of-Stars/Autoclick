import { useMemo, useState } from "react";
import type { PreviewMessage, RuntimeControllerSnapshot } from "../../lib/contracts";
import { formatDecision, statusLabelMap, usePreviewUrl } from "../../lib/presentation";
import { useRuntimeStore } from "../../stores/runtimeStore";

interface FrozenFrame {
  preview: PreviewMessage;
  snapshot: RuntimeControllerSnapshot | null;
  capturedAt: string;
}

function PreviewViewport(props: {
  label: string;
  preview: PreviewMessage | null;
  snapshot: RuntimeControllerSnapshot | null;
  emptyText: string;
}) {
  const previewUrl = usePreviewUrl(props.preview);
  const match = props.snapshot?.bestMatch ?? null;
  const previewWidth = props.preview?.preview.width ?? 0;
  const previewHeight = props.preview?.preview.height ?? 0;
  const matchBox =
    match && previewWidth > 0 && previewHeight > 0
      ? {
          left: `${(match.x / previewWidth) * 100}%`,
          top: `${(match.y / previewHeight) * 100}%`,
          width: `${(match.width / previewWidth) * 100}%`,
          height: `${(match.height / previewHeight) * 100}%`
        }
      : null;

  return (
    <div className="flex min-h-0 flex-col border border-white/10 bg-black/20">
      <div className="flex shrink-0 items-center justify-between gap-3 border-b border-white/10 px-3 py-2">
        <div className="min-w-0">
          <p className="desk-eyebrow">{props.label}</p>
          <p className="mt-1 truncate text-[11px] text-slate-500">
            {props.preview ? `${props.preview.preview.width} × ${props.preview.preview.height}` : "无可用帧"}
          </p>
        </div>
        <div className="min-w-0 text-right text-[11px] text-slate-500">
          <p>令牌</p>
          <p className="mt-1 max-w-[180px] truncate text-slate-300">{props.preview?.token ?? "--"}</p>
        </div>
      </div>

      <div className="relative min-h-0 flex-1 bg-black/30 p-3">
        {previewUrl ? (
          <div
            className="relative mx-auto h-full max-h-full w-full"
            style={{ aspectRatio: `${previewWidth} / ${previewHeight}` }}
          >
            <img
              alt={props.label}
              className="absolute inset-0 h-full w-full object-fill"
              src={previewUrl}
            />
            <div className="pointer-events-none absolute inset-x-0 top-0 flex items-center justify-between border-b border-white/10 bg-black/55 px-3 py-1.5 text-[10px] uppercase tracking-[0.22em] text-slate-400">
              <span>{props.snapshot?.decision ? formatDecision(props.snapshot.decision) : "无决策"}</span>
              <span>{match ? `命中 ${match.templateName}` : "无命中"}</span>
            </div>
            {matchBox ? (
              <div
                className="pointer-events-none absolute border border-warning shadow-[0_0_0_1px_rgba(201,151,66,0.18)]"
                style={matchBox}
              />
            ) : null}
            <div className="pointer-events-none absolute inset-x-0 bottom-0 flex items-center justify-between border-t border-white/10 bg-black/55 px-3 py-1.5 text-[11px] text-slate-300">
              <span>
                点击点:{" "}
                {props.snapshot?.lastClick
                  ? `${props.snapshot.lastClick.clientX}, ${props.snapshot.lastClick.clientY}`
                  : "--"}
              </span>
              <span>分数: {match ? match.score.toFixed(3) : "--"}</span>
            </div>
          </div>
        ) : (
          <div className="flex h-full min-h-[320px] items-center justify-center text-sm text-slate-500">
            {props.emptyText}
          </div>
        )}
      </div>
    </div>
  );
}

export default function LivePreviewPage() {
  const snapshot = useRuntimeStore((state) => state.snapshot);
  const preview = useRuntimeStore((state) => state.preview);
  const [paused, setPaused] = useState(false);
  const [compareMode, setCompareMode] = useState(false);
  const [frozenFrame, setFrozenFrame] = useState<FrozenFrame | null>(null);

  const effectivePreview = preview ?? snapshot?.preview ?? null;
  const livePreview = paused ? null : effectivePreview;

  // 根据运行状态生成更准确的空帧提示
  const liveEmptyText = useMemo(() => {
    if (paused) return "预览已暂停，扫描主链路不会停止。";
    const status = snapshot?.status;
    if (status === "Starting") return "正在启动捕获会话，请稍候…";
    if (status === "Recovering") {
      const reason = snapshot?.metrics.runtime.recovery.lastReason;
      return `正在自动恢复捕获${reason ? `（${reason}）` : ""}…`;
    }
    if (status === "Running" || status === "CoolingDown") {
      const fps = snapshot?.metrics.runtime.performance.captureFps ?? 0;
      if (fps === 0) {
        const err = snapshot?.lastError;
        return err
          ? `捕获帧未到达：${err}`
          : "等待第一帧… 如长时间无响应请检查目标窗口是否可见且未最小化。";
      }
    }
    if (status === "Faulted") {
      return snapshot?.lastError ?? "运行时故障，请查看日志诊断页面。";
    }
    return "当前没有可显示的预览帧。";
  }, [paused, snapshot]);

  const previewPanels = useMemo(() => {
    if (compareMode && frozenFrame) {
      return [
        {
          key: "live",
          label: "Live Feed",
          preview: livePreview,
          snapshot,
          emptyText: liveEmptyText
        },
        {
          key: "frozen",
          label: "Frozen Frame",
          preview: frozenFrame.preview,
          snapshot: frozenFrame.snapshot,
          emptyText: "当前没有冻结快照。"
        }
      ];
    }

    return [
      {
        key: "live",
        label: frozenFrame && !livePreview ? "Frozen Frame" : "Live Feed",
        preview: livePreview ?? frozenFrame?.preview ?? null,
        snapshot: livePreview ? snapshot : frozenFrame?.snapshot ?? snapshot,
        emptyText: liveEmptyText
      }
    ];
  }, [compareMode, frozenFrame, livePreview, liveEmptyText, snapshot]);

  return (
    <section className="desk-page">
      <header className="desk-page-header">
        <div className="desk-page-header-main">
          <div className="min-w-0">
            <p className="desk-eyebrow">Live Preview</p>
            <div className="mt-2 flex flex-wrap items-center gap-3">
              <h1 className="desk-title">实时预览</h1>
              <span className="desk-chip">状态 {statusLabelMap[snapshot?.status ?? "Idle"]}</span>
              <span className="desk-chip">
                帧间隔 {(snapshot?.metrics.runtime.performance.frameIntervalMs ?? 0).toFixed(1)} ms
              </span>
              <span className="desk-chip">
                匹配 {snapshot?.bestMatch?.templateName ?? "暂无"}
              </span>
            </div>
          </div>
          <div className="desk-toolbar justify-end">
            <button
              type="button"
              className="desk-button desk-button-neutral"
              onClick={() => setPaused((value) => !value)}
            >
              {paused ? "恢复预览" : "暂停预览"}
            </button>
            <button
              type="button"
              className="desk-button desk-button-primary disabled:cursor-not-allowed disabled:opacity-50"
              disabled={!effectivePreview}
              onClick={() => {
                if (effectivePreview) {
                  setFrozenFrame({
                    preview: effectivePreview,
                    snapshot,
                    capturedAt: new Date().toLocaleString("zh-CN")
                  });
                }
              }}
            >
              冻结当前帧
            </button>
            <button
              type="button"
              className="desk-button desk-button-neutral disabled:cursor-not-allowed disabled:opacity-50"
              disabled={!frozenFrame}
              onClick={() => setCompareMode((value) => !value)}
            >
              {compareMode ? "关闭对比" : "冻结对比"}
            </button>
            <button
              type="button"
              className="desk-button desk-button-danger disabled:cursor-not-allowed disabled:opacity-50"
              disabled={!frozenFrame}
              onClick={() => {
                setFrozenFrame(null);
                setCompareMode(false);
              }}
            >
              清空冻结
            </button>
          </div>
        </div>
        <div className="desk-statline shrink-0">
          <span>预览通道: {livePreview?.preview.width ?? 0} × {livePreview?.preview.height ?? 0}</span>
          <span>检测耗时: {(snapshot?.metrics.runtime.performance.detectLatencyMs ?? 0).toFixed(1)} ms</span>
          <span>预览编码: {(snapshot?.metrics.runtime.performance.previewLatencyMs ?? 0).toFixed(1)} ms</span>
          <span>端到端: {(snapshot?.metrics.runtime.performance.endToEndLatencyMs ?? 0).toFixed(1)} ms</span>
        </div>
      </header>

      <div className="desk-page-body grid gap-4 xl:grid-cols-[minmax(0,1.45fr)_320px]">
        <article className="desk-panel flex min-h-0 flex-col p-3">
          <div className="shrink-0 border-b border-white/10 pb-2">
            <div className="grid gap-2 text-[11px] text-slate-400 xl:grid-cols-4">
              <div className="border border-white/10 bg-black/10 px-3 py-2">
                <p className="desk-field-label">匹配结果</p>
                <p className="mt-1 text-sm font-semibold text-slate-100">
                  {snapshot?.bestMatch?.templateName ?? "暂无"}
                </p>
              </div>
              <div className="border border-white/10 bg-black/10 px-3 py-2">
                <p className="desk-field-label">匹配分数</p>
                <p className="mt-1 text-sm font-semibold text-slate-100">
                  {snapshot?.bestMatch?.score?.toFixed(3) ?? "--"}
                </p>
              </div>
              <div className="border border-white/10 bg-black/10 px-3 py-2">
                <p className="desk-field-label">点击点</p>
                <p className="mt-1 text-sm font-semibold text-slate-100">
                  {snapshot?.lastClick
                    ? `${snapshot.lastClick.clientX}, ${snapshot.lastClick.clientY}`
                    : "--"}
                </p>
              </div>
              <div className="border border-white/10 bg-black/10 px-3 py-2">
                <p className="desk-field-label">冻结状态</p>
                <p className="mt-1 text-sm font-semibold text-slate-100">
                  {frozenFrame ? "已冻结" : "未冻结"}
                </p>
              </div>
            </div>
          </div>

          <div className={`mt-3 grid min-h-0 flex-1 gap-3 ${compareMode && frozenFrame ? "xl:grid-cols-2" : ""}`}>
            {previewPanels.map((panel) => (
              <PreviewViewport
                key={panel.key}
                emptyText={panel.emptyText}
                label={panel.label}
                preview={panel.preview}
                snapshot={panel.snapshot}
              />
            ))}
          </div>
        </article>

        <aside className="desk-panel-strong flex min-h-0 flex-col p-4">
          <div className="shrink-0 border-b border-white/10 pb-3">
            <p className="desk-eyebrow">Inspector</p>
            <h2 className="mt-2 text-lg font-semibold text-slate-100">调试检查器</h2>
          </div>
          <div className="desk-scroll mt-4 space-y-3">
            <div className="desk-field">
              <p className="desk-field-label">决策</p>
              <p className="mt-2 text-base font-semibold text-slate-100">
                {formatDecision(snapshot?.decision)}
              </p>
            </div>
            <div className="desk-field">
              <p className="desk-field-label">命中框</p>
              <div className="mt-2 grid gap-2 text-xs leading-6 text-slate-300">
                <p>
                  坐标:{" "}
                  {snapshot?.bestMatch
                    ? `${snapshot.bestMatch.x}, ${snapshot.bestMatch.y}`
                    : "--"}
                </p>
                <p>
                  尺寸:{" "}
                  {snapshot?.bestMatch
                    ? `${snapshot.bestMatch.width} × ${snapshot.bestMatch.height}`
                    : "--"}
                </p>
                <p>尺度: {snapshot?.bestMatch?.scale?.toFixed(2) ?? "--"}</p>
              </div>
            </div>
            <div className="desk-field">
              <p className="desk-field-label">捕获摘要</p>
              <div className="mt-2 grid gap-2 text-xs leading-6 text-slate-300">
                <p>来源: {snapshot?.metrics.runtime.capture.activeSource ?? "未记录"}</p>
                <p>
                  尺寸: {snapshot?.metrics.runtime.capture.frameWidth ?? 0} × {snapshot?.metrics.runtime.capture.frameHeight ?? 0}
                </p>
                <p>丢帧: {snapshot?.metrics.runtime.capture.drops ?? 0}</p>
                <p>点击次数: {snapshot?.metrics.runtime.performance.clickCount ?? 0}</p>
              </div>
            </div>
            <div className="desk-field">
              <p className="desk-field-label">冻结快照</p>
              <div className="mt-2 grid gap-2 text-xs leading-6 text-slate-300">
                <p>时间: {frozenFrame?.capturedAt ?? "--"}</p>
                <p>令牌: {frozenFrame?.preview.token ?? "尚未生成"}</p>
                <p>对比模式: {compareMode ? "开启" : "关闭"}</p>
              </div>
            </div>
            <div className="desk-field">
              <p className="desk-field-label">预览令牌</p>
              <p className="mt-2 break-all text-xs leading-6 text-slate-300">
                {effectivePreview?.token ?? "尚未生成"}
              </p>
            </div>
          </div>
        </aside>
      </div>
    </section>
  );
}
