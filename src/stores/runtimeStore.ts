import { create } from "zustand";
import type { PreviewMessage, RuntimeControllerSnapshot, RuntimeStatus } from "../lib/contracts";
import { tauriClient } from "../lib/tauriClient";

let refreshTask: Promise<void> | null = null;
let previewRefreshTask: Promise<void> | null = null;

const IDLE_RUNTIME_REFRESH_INTERVAL_MS = 1500;
const ACTIVE_RUNTIME_REFRESH_INTERVAL_MS = 500;
const STARTING_RUNTIME_REFRESH_INTERVAL_MS = 120;
const STOPPING_RUNTIME_REFRESH_INTERVAL_MS = 150;
const IDLE_PREVIEW_REFRESH_INTERVAL_MS = 900;
const ACTIVE_PREVIEW_REFRESH_INTERVAL_MS = 300;
const STARTING_PREVIEW_REFRESH_INTERVAL_MS = 100;

interface RuntimeStoreState {
  snapshot: RuntimeControllerSnapshot | null;
  preview: PreviewMessage | null;
  loading: boolean;
  error: string | null;
  refresh: () => Promise<void>;
  refreshPreview: () => Promise<void>;
  start: () => Promise<void>;
  stop: () => Promise<void>;
  restart: () => Promise<void>;
  clearError: () => void;
}

export function shouldPollPreview(status: RuntimeStatus) {
  return (
    status === "Starting" ||
    status === "Running" ||
    status === "CoolingDown" ||
    status === "Recovering"
  );
}

function shouldPreservePreview(status: RuntimeStatus) {
  return status === "Running" || status === "CoolingDown" || status === "Recovering";
}

export function runtimeRefreshIntervalMs(status: RuntimeStatus) {
  if (status === "Stopping") {
    return STOPPING_RUNTIME_REFRESH_INTERVAL_MS;
  }
  if (status === "Starting") {
    return STARTING_RUNTIME_REFRESH_INTERVAL_MS;
  }
  if (shouldPreservePreview(status)) {
    return ACTIVE_RUNTIME_REFRESH_INTERVAL_MS;
  }
  return IDLE_RUNTIME_REFRESH_INTERVAL_MS;
}

export function previewRefreshIntervalMs(status: RuntimeStatus) {
  if (status === "Starting") {
    return STARTING_PREVIEW_REFRESH_INTERVAL_MS;
  }
  if (shouldPreservePreview(status)) {
    return ACTIVE_PREVIEW_REFRESH_INTERVAL_MS;
  }
  return IDLE_PREVIEW_REFRESH_INTERVAL_MS;
}

export function resolveRuntimePreview(
  snapshot: RuntimeControllerSnapshot,
  previousPreview: PreviewMessage | null
) {
  if (snapshot.preview) {
    return snapshot.preview;
  }
  return shouldPreservePreview(snapshot.status) ? previousPreview : null;
}

export function canStartRuntime(status: RuntimeStatus) {
  return status === "Idle" || status === "Faulted" || status === "Stopping";
}

export function canRestartRuntime(status: RuntimeStatus) {
  return (
    status === "Running" ||
    status === "CoolingDown" ||
    status === "Recovering" ||
    status === "Stopping" ||
    status === "Faulted"
  );
}

export function canStopRuntime(status: RuntimeStatus) {
  return (
    status === "Starting" ||
    status === "Running" ||
    status === "CoolingDown" ||
    status === "Recovering" ||
    status === "Faulted"
  );
}

function currentStatus(state: RuntimeStoreState) {
  return state.snapshot?.status ?? "Idle";
}

export const useRuntimeStore = create<RuntimeStoreState>((set, get) => ({
  snapshot: null,
  preview: null,
  loading: false,
  error: null,
  refresh: async () => {
    if (refreshTask) {
      await refreshTask;
      return;
    }

    refreshTask = (async () => {
      try {
        const snapshot = await tauriClient.getRuntimeStatus();
        const previousPreview = get().preview;
        set({
          snapshot,
          preview: resolveRuntimePreview(snapshot, previousPreview),
          error: null
        });
      } catch (error) {
        set({ error: error instanceof Error ? error.message : "读取运行状态失败" });
      } finally {
        refreshTask = null;
      }
    })();

    await refreshTask;
  },
  refreshPreview: async () => {
    if (previewRefreshTask) {
      await previewRefreshTask;
      return;
    }

    previewRefreshTask = (async () => {
      try {
        const preview = await tauriClient.getPreviewSnapshot();
        const snapshot = get().snapshot;
        if (preview) {
          set({ preview, error: null });
        } else if (snapshot?.preview) {
          set({ preview: snapshot.preview, error: null });
        } else if (!shouldPollPreview(snapshot?.status ?? "Idle")) {
          set({ preview: null });
        }
      } catch (error) {
        set({ error: error instanceof Error ? error.message : "读取预览失败" });
      } finally {
        previewRefreshTask = null;
      }
    })();

    await previewRefreshTask;
  },
  start: async () => {
    const status = currentStatus(get());
    if (get().loading || !canStartRuntime(status)) {
      return;
    }
    set({ loading: true, error: null, preview: null });
    try {
      // 停止中的再次开始本质上是“等待旧链路退出后重启”
      const snapshot =
        status === "Stopping"
          ? await tauriClient.restartRuntime()
          : await tauriClient.startRuntime();
      set({ snapshot, preview: snapshot.preview ?? null, loading: false });
      if (shouldPollPreview(snapshot.status)) {
        await get().refreshPreview();
      }
    } catch (error) {
      set({
        loading: false,
        error: error instanceof Error ? error.message : "启动扫描失败"
      });
    }
  },
  stop: async () => {
    if (get().loading || !canStopRuntime(currentStatus(get()))) {
      return;
    }
    set({ loading: true, error: null });
    try {
      const snapshot = await tauriClient.stopRuntime();
      set({ snapshot, preview: null, loading: false });
      // 后端 stop 已尝试等待线程退出，若仍为 Stopping 则快速轮询
      if (snapshot.status !== "Idle" && snapshot.status !== "Faulted") {
        for (let i = 0; i < 20; i++) {
          await new Promise<void>((resolve) => {
            setTimeout(resolve, 100);
          });
          try {
            const updated = await tauriClient.getRuntimeStatus();
            const prev = get().preview;
            set({
              snapshot: updated,
              preview: resolveRuntimePreview(updated, prev),
              error: null
            });
            if (updated.status === "Idle" || updated.status === "Faulted") {
              break;
            }
          } catch {
            // 忽略轮询异常，继续等待
          }
        }
      }
    } catch (error) {
      set({
        loading: false,
        error: error instanceof Error ? error.message : "停止扫描失败"
      });
    }
  },
  restart: async () => {
    if (get().loading || !canRestartRuntime(currentStatus(get()))) {
      return;
    }
    set({ loading: true, error: null, preview: null });
    try {
      const snapshot = await tauriClient.restartRuntime();
      set({ snapshot, preview: snapshot.preview ?? null, loading: false });
      if (shouldPollPreview(snapshot.status)) {
        await get().refreshPreview();
      }
    } catch (error) {
      set({
        loading: false,
        error: error instanceof Error ? error.message : "重启扫描失败"
      });
    }
  },
  clearError: () => set({ error: null })
}));
