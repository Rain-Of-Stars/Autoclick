import { Outlet } from "react-router-dom";
import { useEffect } from "react";
import { Sidebar } from "./Sidebar";
import { TopBar } from "./TopBar";
import { StatusBar } from "./StatusBar";
import { useConfigStore } from "../../stores/configStore";
import {
  previewRefreshIntervalMs,
  runtimeRefreshIntervalMs,
  shouldPollPreview,
  useRuntimeStore
} from "../../stores/runtimeStore";

function isDesktopRuntime() {
  return typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;
}

export function shouldSuspendBackgroundPolling(hidden: boolean, desktopRuntime: boolean) {
  return hidden && !desktopRuntime;
}

export function AppShell() {
  const loadInitial = useConfigStore((state) => state.loadInitial);
  const configError = useConfigStore((state) => state.error);
  const clearConfigError = useConfigStore((state) => state.clearError);
  const runtimeError = useRuntimeStore((state) => state.error);
  const clearRuntimeError = useRuntimeStore((state) => state.clearError);
  const refresh = useRuntimeStore((state) => state.refresh);
  const refreshPreview = useRuntimeStore((state) => state.refreshPreview);
  const runtimeStatus = useRuntimeStore((state) => state.snapshot?.status ?? "Idle");

  useEffect(() => {
    void loadInitial();
    void refresh();
  }, [loadInitial, refresh]);

  useEffect(() => {
    const desktopRuntime = isDesktopRuntime();
    const runtimeTimer = window.setInterval(() => {
      if (shouldSuspendBackgroundPolling(document.hidden, desktopRuntime)) {
        return;
      }
      void refresh();
    }, runtimeRefreshIntervalMs(runtimeStatus));
    return () => {
      window.clearInterval(runtimeTimer);
    };
  }, [refresh, runtimeStatus]);

  useEffect(() => {
    if (
      shouldSuspendBackgroundPolling(document.hidden, isDesktopRuntime()) ||
      !shouldPollPreview(runtimeStatus)
    ) {
      return;
    }
    void refreshPreview();
  }, [refreshPreview, runtimeStatus]);

  useEffect(() => {
    if (!shouldPollPreview(runtimeStatus)) {
      return;
    }

    const desktopRuntime = isDesktopRuntime();

    const previewTimer = window.setInterval(() => {
      if (shouldSuspendBackgroundPolling(document.hidden, desktopRuntime)) {
        return;
      }
      void refreshPreview();
    }, previewRefreshIntervalMs(runtimeStatus));

    return () => {
      window.clearInterval(previewTimer);
    };
  }, [refreshPreview, runtimeStatus]);

  const errorMessage = runtimeError ?? configError;

  return (
    <div className="h-screen overflow-hidden bg-surface text-ink selection:bg-accent/30">
      <div className="relative mx-auto flex h-screen w-full bg-black/40">
        <div className="flex w-full overflow-hidden bg-gradient-to-br from-[#080b0f] via-[#0b1018] to-[#040608] relative">
          <div className="pointer-events-none absolute inset-0 opacity-[0.015] bg-[url('data:image/svg+xml;base64,PHN2ZyB4bWxucz0iaHR0cDovL3d3dy53My5vcmcvMjAwMC9zdmciIHdpZHRoPSI4IiBoZWlnaHQ9IjgiPjxjaXJjbGUgY3g9IjEiIGN5PSIxIiByPSIxIiBmaWxsPSIjZmZmIi8+PC9zdmc+')] mix-blend-screen" />
          <div className="pointer-events-none absolute inset-0 bg-gradient-to-t from-transparent via-accent/[0.02] to-transparent bg-[length:100%_200%] animate-[scan_8s_ease-in-out_infinite]" />
          
          <Sidebar />
          <div className="flex min-w-0 flex-1 flex-col overflow-hidden relative z-10 shadow-[-10px_0_30px_rgba(0,0,0,0.5)]">
            <TopBar />
            {errorMessage ? (
              <div className="mx-4 mt-3 flex shrink-0 items-center justify-between rounded-lg border border-danger/30 bg-danger/10 px-3 py-2 text-[13px] text-danger">
                <span>{errorMessage}</span>
                <button
                  type="button"
                  className="desk-button desk-button-danger h-7 px-2.5 text-[11px] uppercase tracking-[0.18em]"
                  onClick={() => {
                    clearRuntimeError();
                    clearConfigError();
                  }}
                >
                  清除
                </button>
              </div>
            ) : null}
            <main className="min-h-0 flex-1 overflow-hidden">
              <Outlet />
            </main>
            <StatusBar />
          </div>
        </div>
      </div>
    </div>
  );
}
