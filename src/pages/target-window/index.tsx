import { useMemo, useState } from "react";
import { useEncodedPreviewUrl } from "../../lib/presentation";
import { useConfigStore } from "../../stores/configStore";

export default function TargetWindowPage() {
  const windows = useConfigStore((state) => state.windows);
  const locatedTarget = useConfigStore((state) => state.locatedTarget);
  const targetCapture = useConfigStore((state) => state.targetCapture);
  const pickTargetWindow = useConfigStore((state) => state.pickTargetWindow);
  const testTargetCapture = useConfigStore((state) => state.testTargetCapture);
  const locateTarget = useConfigStore((state) => state.locateTarget);
  const [keyword, setKeyword] = useState("");
  const selectedHwnd = locatedTarget?.window.hwnd ?? null;

  const filteredWindows = useMemo(() => {
    const lowerKeyword = keyword.trim().toLowerCase();
    if (!lowerKeyword) {
      return windows;
    }
    return windows.filter((window) =>
      [window.title, window.className, window.exePath ?? ""]
        .join(" ")
        .toLowerCase()
        .includes(lowerKeyword)
    );
  }, [keyword, windows]);

  const capturePreview = useEncodedPreviewUrl(targetCapture?.preview ?? null, targetCapture);
  const tableGridClass =
    "grid grid-cols-[minmax(0,1.2fr)_minmax(120px,0.55fr)_72px_110px_150px] gap-3";

  return (
    <section className="desk-page">
      <header className="desk-page-header">
        <div className="desk-page-header-main">
          <div className="min-w-0">
            <p className="desk-eyebrow">Window Explorer</p>
            <div className="mt-2 flex flex-wrap items-center gap-3">
              <h1 className="desk-title">目标窗口</h1>
              <span className="desk-chip">窗口 {windows.length}</span>
              <span className="desk-chip">筛选 {filteredWindows.length}</span>
              <span className="desk-chip">当前句柄 {selectedHwnd ?? "--"}</span>
            </div>
          </div>
          <div className="desk-toolbar justify-end">
            <input
              className="desk-input w-[240px]"
              placeholder="筛选标题、类名、进程"
              value={keyword}
              onChange={(event) => setKeyword(event.target.value)}
            />
            <button type="button" className="desk-button desk-button-neutral" onClick={() => void locateTarget()}>
              重新定位
            </button>
          </div>
        </div>
        <div className="desk-statline shrink-0">
          <span>定位原因: {locatedTarget?.reason ?? "--"}</span>
          <span>可靠度: {locatedTarget?.reliability ?? "--"}</span>
          <span>测试捕获: {targetCapture ? "已生成" : "未生成"}</span>
        </div>
      </header>

      <div className="desk-page-body grid gap-4 xl:grid-cols-[minmax(0,1.15fr)_360px]">
        <article className="desk-panel flex min-h-0 flex-col p-3">
          <div className="shrink-0 border-b border-white/10 pb-2">
            <div className="grid gap-2 text-[11px] text-slate-400 xl:grid-cols-4">
              <div className="border border-white/10 bg-black/10 px-3 py-2">
                <p className="desk-field-label">可见窗口</p>
                <p className="mt-1 text-sm font-semibold text-slate-100">
                  {windows.filter((window) => window.isVisible).length}
                </p>
              </div>
              <div className="border border-white/10 bg-black/10 px-3 py-2">
                <p className="desk-field-label">最小化窗口</p>
                <p className="mt-1 text-sm font-semibold text-slate-100">
                  {windows.filter((window) => window.isMinimized).length}
                </p>
              </div>
              <div className="border border-white/10 bg-black/10 px-3 py-2">
                <p className="desk-field-label">当前目标</p>
                <p className="mt-1 truncate text-sm font-semibold text-slate-100">
                  {locatedTarget?.window.title ? `已选 ${locatedTarget.window.title}` : "未定位"}
                </p>
              </div>
              <div className="border border-white/10 bg-black/10 px-3 py-2">
                <p className="desk-field-label">筛选关键字</p>
                <p className="mt-1 truncate text-sm font-semibold text-slate-100">
                  {keyword || "未启用"}
                </p>
              </div>
            </div>
          </div>

          <div className="desk-table mt-3 min-h-0 flex-1 overflow-hidden">
            <div className={`desk-table-head ${tableGridClass} px-3 py-2`}>
              <span>窗口</span>
              <span>类名</span>
              <span>PID</span>
              <span>句柄</span>
              <span>操作</span>
            </div>
            <div className="desk-scroll desk-scrollbar-visible flex-1 overflow-x-hidden">
              {filteredWindows.map((window) => (
                <div
                  key={window.hwnd}
                  className={[
                    `desk-table-row ${tableGridClass} px-3 py-3 text-xs text-slate-300`,
                    selectedHwnd === window.hwnd
                      ? "desk-selected-row"
                      : ""
                  ].join(" ")}
                >
                  <div className="min-w-0">
                    <p className="truncate text-sm font-medium text-slate-100">{window.title || "无标题窗口"}</p>
                    <div className="mt-1 flex flex-wrap items-center gap-1.5 text-[10px] text-slate-500">
                      <span className="min-w-0 truncate">{window.exePath ?? "未知进程"}</span>
                      {window.isVisible ? <span className="desk-chip h-5 px-2 text-[9px]">可见</span> : null}
                      {window.isMinimized ? <span className="desk-chip h-5 px-2 text-[9px]">最小化</span> : null}
                    </div>
                  </div>
                  <span className="truncate" title={window.className || "--"}>{window.className || "--"}</span>
                  <span>{window.pid}</span>
                  <span>{window.hwnd}</span>
                  <div className="grid gap-2">
                    <button
                      type="button"
                      className="desk-button desk-button-primary h-7 w-full justify-center px-2.5 text-[10px] tracking-[0.08em]"
                      onClick={() => void pickTargetWindow(window.hwnd)}
                    >
                      设为目标
                    </button>
                    <button
                      type="button"
                      className="desk-button desk-button-neutral h-7 w-full justify-center px-2.5 text-[10px] tracking-[0.08em]"
                      onClick={() => void testTargetCapture(window.hwnd)}
                    >
                      测试捕获
                    </button>
                  </div>
                </div>
              ))}
            </div>
          </div>
        </article>

        <aside className="desk-panel-strong flex min-h-0 flex-col p-4">
          <div className="shrink-0 border-b border-white/10 pb-3">
            <p className="desk-eyebrow">Selection</p>
            <h2 className="mt-2 text-lg font-semibold text-slate-100">
              {locatedTarget?.window.title ?? "尚未选择目标"}
            </h2>
          </div>
          <div className="desk-scroll mt-4 space-y-3">
            <div className="desk-field">
              <p className="desk-field-label">定位摘要</p>
              <div className="mt-2 grid gap-2 text-xs leading-6 text-slate-300">
                <p>定位原因: {locatedTarget?.reason ?? "--"}</p>
                <p>可靠度: {locatedTarget?.reliability ?? "--"}</p>
                <p>句柄: {locatedTarget?.window.hwnd ?? "--"}</p>
                <p>类名: {locatedTarget?.window.className ?? "--"}</p>
              </div>
            </div>
            <div className="desk-field">
              <p className="desk-field-label">进程信息</p>
              <div className="mt-2 grid gap-2 text-xs leading-6 text-slate-300">
                <p>PID: {locatedTarget?.window.pid ?? "--"}</p>
                <p>可见: {locatedTarget?.window.isVisible ? "是" : "否"}</p>
                <p>最小化: {locatedTarget?.window.isMinimized ? "是" : "否"}</p>
                <p>路径: {locatedTarget?.window.exePath ?? "--"}</p>
              </div>
            </div>
            <div className="border border-white/10 bg-black/20">
              {capturePreview ? (
                <img
                  alt="目标窗口测试捕获"
                  className="block h-[220px] w-full object-contain"
                  src={capturePreview}
                />
              ) : (
                <div className="flex h-[220px] items-center justify-center text-sm text-slate-500">
                  还没有测试捕获结果
                </div>
              )}
            </div>
            <div className="desk-field">
              <p className="desk-field-label">快速操作</p>
              <div className="mt-3 grid gap-2">
                <button type="button" className="desk-button desk-button-primary justify-center" onClick={() => selectedHwnd && void testTargetCapture(selectedHwnd)}>
                  测试当前目标
                </button>
                <button type="button" className="desk-button desk-button-neutral justify-center" onClick={() => void locateTarget()}>
                  按当前配置重新定位
                </button>
              </div>
            </div>
          </div>
        </aside>
      </div>
    </section>
  );
}
