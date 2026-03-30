import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import type { RuntimeControllerSnapshot, RuntimeStatus } from "../src/lib/contracts";
import { tauriClient } from "../src/lib/tauriClient";
import {
  canRestartRuntime,
  canStartRuntime,
  previewRefreshIntervalMs,
  resolveRuntimePreview,
  runtimeRefreshIntervalMs,
  shouldPollPreview,
  useRuntimeStore
} from "../src/stores/runtimeStore";

function buildSnapshot(status: RuntimeStatus): RuntimeControllerSnapshot {
  return {
    status,
    metrics: {
      runtime: {
        status,
        performance: {
          captureFps: 0,
          frameIntervalMs: 0,
          detectLatencyMs: 0,
          previewLatencyMs: 0,
          endToEndLatencyMs: 0,
          clickCount: 0,
          lastScore: 0,
          uptimeSecs: 0
        },
        capture: {
          frameWidth: 0,
          frameHeight: 0,
          drops: 0,
          activeSource: null
        },
        recovery: {
          attempts: 0,
          lastReason: null,
          nextRetryInMs: null
        },
        preview: {
          enabled: false,
          frameToken: null,
          width: 0,
          height: 0
        },
        lastError: null
      },
      recoveryCount: 0,
      bufferDrops: 0,
      memoryBytesEstimate: 0
    },
    preview: null,
    activeTarget: null,
    bestMatch: null,
    decision: null,
    lastClick: null,
    lastError: null
  };
}

function buildPreview(token = "preview-1") {
  return {
    token,
    preview: {
      frameId: 1,
      width: 8,
      height: 8,
      mimeType: "image/png",
      bytes: [137, 80, 78, 71]
    }
  };
}

beforeEach(() => {
  useRuntimeStore.setState({
    snapshot: buildSnapshot("Idle"),
    preview: null,
    loading: false,
    error: null
  });
});

afterEach(async () => {
  vi.restoreAllMocks();
  await tauriClient.stopRuntime();
});

describe("runtime store stability", () => {
  it("deduplicates concurrent runtime refresh calls", async () => {
    let resolveRefresh: any = null;
    const getRuntimeStatus = vi
      .spyOn(tauriClient, "getRuntimeStatus")
      .mockImplementation(
        () =>
          new Promise((resolve) => {
            resolveRefresh = resolve;
          })
      );

    const first = useRuntimeStore.getState().refresh();
    const second = useRuntimeStore.getState().refresh();

    expect(getRuntimeStatus).toHaveBeenCalledTimes(1);

    resolveRefresh?.(buildSnapshot("Running"));
    await Promise.all([first, second]);

    expect(useRuntimeStore.getState().snapshot?.status).toBe("Running");
  });

  it("ignores duplicate start requests while runtime is starting", async () => {
    useRuntimeStore.setState({
      snapshot: buildSnapshot("Starting"),
      loading: false,
      error: null,
      preview: null
    });

    const startRuntime = vi.spyOn(tauriClient, "startRuntime");

    await useRuntimeStore.getState().start();

    expect(startRuntime).not.toHaveBeenCalled();
  });

  it("uses restart path when start is clicked during stopping", async () => {
    useRuntimeStore.setState({
      snapshot: buildSnapshot("Stopping"),
      loading: false,
      error: null,
      preview: null
    });

    const restartRuntime = vi
      .spyOn(tauriClient, "restartRuntime")
      .mockResolvedValue(buildSnapshot("Running"));
    const startRuntime = vi.spyOn(tauriClient, "startRuntime");

    await useRuntimeStore.getState().start();

    expect(restartRuntime).toHaveBeenCalledTimes(1);
    expect(startRuntime).not.toHaveBeenCalled();
    expect(useRuntimeStore.getState().snapshot?.status).toBe("Running");
  });

  it("hydrates preview from runtime snapshot refresh", async () => {
    const preview = buildPreview();
    vi.spyOn(tauriClient, "getRuntimeStatus").mockResolvedValue({
      ...buildSnapshot("Running"),
      preview
    });

    await useRuntimeStore.getState().refresh();

    expect(useRuntimeStore.getState().preview).toEqual(preview);
  });

  it("loads mock preview immediately after starting runtime", async () => {
    await tauriClient.stopRuntime();

    await useRuntimeStore.getState().start();

    const state = useRuntimeStore.getState();
    expect(state.snapshot?.status).toBe("Running");
    expect(state.preview?.preview.width).toBeGreaterThan(0);
    expect(state.preview?.preview.height).toBeGreaterThan(0);
    expect(state.preview?.preview.mimeType).toBe("image/svg+xml");
    expect(state.preview?.token).toMatch(/^preview-\d+$/);
  });

  it("advances mock preview token across preview refresh calls", async () => {
    await tauriClient.stopRuntime();

    await useRuntimeStore.getState().start();
    const firstToken = useRuntimeStore.getState().preview?.token;

    await useRuntimeStore.getState().refreshPreview();
    const secondToken = useRuntimeStore.getState().preview?.token;

    expect(firstToken).toBeTruthy();
    expect(secondToken).toBeTruthy();
    expect(secondToken).not.toBe(firstToken);
  });

  it("keeps preview only while runtime is actively producing frames", () => {
    const preview = buildPreview();

    expect(resolveRuntimePreview(buildSnapshot("Running"), preview)).toEqual(preview);
    expect(resolveRuntimePreview(buildSnapshot("Idle"), preview)).toBeNull();
  });

  it("uses fast polling during startup and stopping transitions", () => {
    expect(shouldPollPreview("Starting")).toBe(true);
    expect(canStartRuntime("Stopping")).toBe(true);
    expect(canRestartRuntime("Stopping")).toBe(true);
    expect(runtimeRefreshIntervalMs("Stopping")).toBeLessThan(runtimeRefreshIntervalMs("Idle"));
    expect(previewRefreshIntervalMs("Starting")).toBeLessThan(previewRefreshIntervalMs("Idle"));
  });
});
