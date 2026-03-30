import { useConfigStore } from "../../stores/configStore";
import { useRuntimeStore } from "../../stores/runtimeStore";
import { formatBytes, statusLabelMap } from "../../lib/presentation";

export function StatusBar() {
  const snapshot = useRuntimeStore((state) => state.snapshot);
  const locatedTarget = useConfigStore((state) => state.locatedTarget);

  return (
    <footer className="grid shrink-0 grid-cols-2 gap-x-4 gap-y-2 border-t border-white/10 bg-black/20 px-4 py-2 text-[12px] text-slate-400 xl:grid-cols-6">
      <div>
        <p className="text-[10px] uppercase tracking-[0.24em]">状态</p>
        <p className="mt-1 text-slate-100">{statusLabelMap[snapshot?.status ?? "Idle"]}</p>
      </div>
      <div>
        <p className="text-[10px] uppercase tracking-[0.24em]">捕获 FPS</p>
        <p className="mt-1 text-slate-100">{snapshot?.metrics.runtime.performance.captureFps.toFixed(1) ?? "--"}</p>
      </div>
      <div>
        <p className="text-[10px] uppercase tracking-[0.24em]">帧间隔</p>
        <p className="mt-1 text-slate-100">{snapshot?.metrics.runtime.performance.frameIntervalMs.toFixed(1) ?? "--"} ms</p>
      </div>
      <div>
        <p className="text-[10px] uppercase tracking-[0.24em]">最近分数</p>
        <p className="mt-1 text-slate-100">{snapshot?.metrics.runtime.performance.lastScore.toFixed(3) ?? "--"}</p>
      </div>
      <div>
        <p className="text-[10px] uppercase tracking-[0.24em]">缓冲占用</p>
        <p className="mt-1 text-slate-100">{formatBytes(snapshot?.metrics.memoryBytesEstimate ?? 0)}</p>
      </div>
      <div>
        <p className="text-[10px] uppercase tracking-[0.24em]">目标窗口</p>
        <p className="mt-1 truncate text-slate-100">{locatedTarget?.window.title ?? "未定位"}</p>
      </div>
    </footer>
  );
}
