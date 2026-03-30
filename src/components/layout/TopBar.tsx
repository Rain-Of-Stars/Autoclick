import type { MouseEvent as ReactMouseEvent, ReactNode } from "react";
import { useEffect, useMemo, useState } from "react";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { useNavigate } from "react-router-dom";
import {
  canRestartRuntime,
  canStartRuntime,
  canStopRuntime,
  useRuntimeStore
} from "../../stores/runtimeStore";
import type { AppConfig } from "../../lib/contracts";
import { useConfigStore } from "../../stores/configStore";
import { statusLabelMap } from "../../lib/presentation";

const isDesktopWindow =
  typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;

function requiresTargetWindowSelection(config: AppConfig | null) {
  return config?.capture.source === "Window" && !config.target.hwnd;
}

export function TopBar() {
  const navigate = useNavigate();
  const config = useConfigStore((state) => state.config);
  const refreshWindows = useConfigStore((state) => state.refreshWindows);
  const snapshot = useRuntimeStore((state) => state.snapshot);
  const loading = useRuntimeStore((state) => state.loading);
  const start = useRuntimeStore((state) => state.start);
  const stop = useRuntimeStore((state) => state.stop);
  const restart = useRuntimeStore((state) => state.restart);
  const [maximized, setMaximized] = useState(false);
  const [targetPromptOpen, setTargetPromptOpen] = useState(false);

  const status = snapshot?.status ?? "Idle";
  const theme = useMemo(() => {
    if (status === "Running") {
      return "border-success/40 bg-success/10 text-success";
    }
    if (status === "Faulted") {
      return "border-danger/40 bg-danger/10 text-danger";
    }
    if (
      status === "Starting" ||
      status === "Stopping" ||
      status === "Recovering" ||
      status === "CoolingDown"
    ) {
      return "border-warning/40 bg-warning/10 text-warning";
    }
    return "border-white/10 bg-white/5 text-muted";
  }, [status]);

  useEffect(() => {
    if (!isDesktopWindow) {
      return;
    }

    let disposed = false;
    let unlistenResize: (() => void) | undefined;
    const appWindow = getCurrentWindow();

    void appWindow
      .isMaximized()
      .then((value) => {
        if (!disposed) {
          setMaximized(value);
        }
      })
      .catch(() => {
        if (!disposed) {
          setMaximized(false);
        }
      });

    void appWindow.onResized(async () => {
      if (!disposed) {
        setMaximized(await appWindow.isMaximized());
      }
    }).then((unlisten) => {
      unlistenResize = unlisten;
    });

    return () => {
      disposed = true;
      unlistenResize?.();
    };
  }, []);

  const handleHeaderMouseDown = async (event: ReactMouseEvent<HTMLElement>) => {
    if (!isDesktopWindow || event.button !== 0) {
      return;
    }
    const target = event.target as HTMLElement | null;
    if (
      target?.closest(
        "button, input, textarea, select, a, [role='button'], [data-no-drag='true']"
      )
    ) {
      return;
    }
    await getCurrentWindow().startDragging();
  };

  const handleToggleMaximize = async () => {
    if (!isDesktopWindow) {
      return;
    }
    const appWindow = getCurrentWindow();
    await appWindow.toggleMaximize();
    setMaximized(await appWindow.isMaximized());
  };

  const handleMinimize = async () => {
    if (!isDesktopWindow) {
      return;
    }
    await getCurrentWindow().minimize();
  };

  const handleHide = async () => {
    if (!isDesktopWindow) {
      return;
    }
    await getCurrentWindow().close();
  };

  const openTargetWindowPicker = () => {
    setTargetPromptOpen(false);
    void refreshWindows();
    navigate("/target-window");
  };

  const handleStart = async () => {
    if (requiresTargetWindowSelection(config)) {
      setTargetPromptOpen(true);
      return;
    }
    await start();
  };

  const handleRestart = async () => {
    if (status === "Faulted" && requiresTargetWindowSelection(config)) {
      setTargetPromptOpen(true);
      return;
    }
    await restart();
  };

  return (
    <header
      className="relative shrink-0 border-b border-white/10 bg-slate-950/95 px-4 py-1.5"
      onMouseDown={(event) => {
        void handleHeaderMouseDown(event);
      }}
      onDoubleClick={(event) => {
        const target = event.target as HTMLElement | null;
        if (
          target?.closest(
            "button, input, textarea, select, a, [role='button'], [data-no-drag='true']"
          )
        ) {
          return;
        }
        void handleToggleMaximize();
      }}
    >
      <div className="flex items-center gap-4">
        {/* 应用名 + 状态 */}
        <div data-tauri-drag-region className="flex min-w-0 flex-1 items-center gap-3">
          <h2 className="shrink-0 text-sm font-semibold tracking-wide text-slate-200">
            Autoclick
          </h2>
          <span className={`desk-chip ${theme}`}>
            {statusLabelMap[status]}
          </span>
        </div>
        {/* 操作按钮 */}
        <div data-no-drag="true" className="flex shrink-0 items-center gap-1.5">
          <button
            type="button"
            className="desk-button desk-button-success disabled:cursor-not-allowed disabled:opacity-50"
            disabled={loading || !canStartRuntime(status)}
            onClick={() => void handleStart()}
          >
            开始扫描
          </button>
          <button
            type="button"
            className="desk-button desk-button-warning disabled:cursor-not-allowed disabled:opacity-50"
            disabled={loading || !canRestartRuntime(status)}
            onClick={() => void handleRestart()}
          >
            重启链路
          </button>
          <button
            type="button"
            className="desk-button desk-button-danger disabled:cursor-not-allowed disabled:opacity-50"
            disabled={loading || !canStopRuntime(status)}
            onClick={() => void stop()}
          >
            停止扫描
          </button>
        </div>
        {/* 窗口控制 */}
        {isDesktopWindow ? (
          <div
            data-no-drag="true"
            className="flex shrink-0 items-center border border-white/10 bg-black/20"
          >
            <WindowButton label="最小化窗口" onClick={() => void handleMinimize()}>
              <svg viewBox="0 0 16 16" className="h-4 w-4" fill="none" aria-hidden="true">
                <path d="M4 8.5H12" stroke="currentColor" strokeWidth="1.4" strokeLinecap="round" />
              </svg>
            </WindowButton>
            <WindowButton label={maximized ? "还原窗口" : "最大化窗口"} onClick={() => void handleToggleMaximize()}>
              <svg viewBox="0 0 16 16" className="h-4 w-4" fill="none" aria-hidden="true">
                {maximized ? (
                  <>
                    <path d="M5.5 4.5H10.5V9.5H5.5Z" stroke="currentColor" strokeWidth="1.2" />
                    <path d="M7 6H12V11H7" stroke="currentColor" strokeWidth="1.2" />
                  </>
                ) : (
                  <path d="M4.5 4.5H11.5V11.5H4.5Z" stroke="currentColor" strokeWidth="1.2" />
                )}
              </svg>
            </WindowButton>
            <WindowButton label="关闭窗口并隐藏到托盘" danger onClick={() => void handleHide()}>
              <svg viewBox="0 0 16 16" className="h-4 w-4" fill="none" aria-hidden="true">
                <path d="M5 5L11 11" stroke="currentColor" strokeWidth="1.4" strokeLinecap="round" />
                <path d="M11 5L5 11" stroke="currentColor" strokeWidth="1.4" strokeLinecap="round" />
              </svg>
            </WindowButton>
          </div>
        ) : null}
      </div>
      {targetPromptOpen ? (
        <div
          aria-modal="true"
          className="fixed inset-0 z-50 flex items-center justify-center bg-black/72 px-4"
          data-no-drag="true"
          role="dialog"
          onDoubleClick={(event) => event.stopPropagation()}
          onMouseDown={(event) => event.stopPropagation()}
        >
          <div className="w-full max-w-md border border-warning/30 bg-[#09101a] p-5 shadow-[0_24px_80px_rgba(0,0,0,0.55)]">
            <p className="desk-eyebrow text-warning">Scan Guard</p>
            <h3 className="mt-2 text-lg font-semibold text-slate-100">开始扫描前需要先选择目标窗口</h3>
            <p className="mt-3 text-sm leading-6 text-slate-300">
              当前捕获源处于窗口模式，但还没有绑定目标窗口。继续开始扫描会导致运行链路无法建立，请先前往目标窗口页面完成选择。
            </p>
            <div className="mt-5 flex justify-end gap-2">
              <button
                type="button"
                className="desk-button desk-button-neutral"
                onClick={() => setTargetPromptOpen(false)}
              >
                暂不处理
              </button>
              <button
                type="button"
                className="desk-button desk-button-primary"
                onClick={openTargetWindowPicker}
              >
                去选择窗口
              </button>
            </div>
          </div>
        </div>
      ) : null}
    </header>
  );
}

type WindowButtonProps = {
  children: ReactNode;
  danger?: boolean;
  label: string;
  onClick: () => void;
};

function WindowButton({ children, danger = false, label, onClick }: WindowButtonProps) {
  return (
    <button
      type="button"
      aria-label={label}
      title={label}
      className={[
        "flex h-9 w-10 items-center justify-center border-r border-white/10 text-slate-400 transition last:border-r-0",
        danger
          ? "hover:bg-danger/15 hover:text-danger"
          : "hover:bg-white/[0.08] hover:text-slate-100"
      ].join(" ")}
      onClick={onClick}
    >
      {children}
    </button>
  );
}
