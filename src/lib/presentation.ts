import { useEffect, useState } from "react";
import type {
  EncodedPreview,
  PreviewMessage,
  RuntimeDecision,
  RuntimeStatus
} from "./contracts";

export const statusLabelMap: Record<RuntimeStatus, string> = {
  Idle: "空闲",
  Starting: "启动中",
  Running: "运行中",
  CoolingDown: "冷却中",
  Recovering: "恢复中",
  Stopping: "停止中",
  Faulted: "故障"
};

export const formatDecision = (decision: RuntimeDecision | null | undefined) => {
  if (!decision) {
    return "暂无决策";
  }
  if (typeof decision === "string") {
    return {
      NoMatch: "未命中",
      BelowThreshold: "低于阈值"
    }[decision] ?? decision;
  }
  if ("Pending" in decision) {
    return `连续命中 ${decision.Pending} 帧`;
  }
  if ("CoolingDown" in decision) {
    return `冷却剩余 ${decision.CoolingDown} ms`;
  }
  if ("ShouldClick" in decision) {
    return `已触发点击: ${decision.ShouldClick.templateName}`;
  }
  return "未知决策";
};

function bytesToDataUrl(bytes: number[], mimeType: string) {
  if (bytes.length === 0) {
    return null;
  }
  // 分块拼接避免 O(n²) 字符串拼接，提升大帧性能
  const chunks: string[] = [];
  const chunkSize = 8192;
  for (let i = 0; i < bytes.length; i += chunkSize) {
    const slice = bytes.slice(i, i + chunkSize);
    chunks.push(String.fromCharCode(...slice));
  }
  return `data:${mimeType};base64,${btoa(chunks.join(""))}`;
}

function buildPreviewUrl(preview: EncodedPreview | null) {
  if (!preview || preview.bytes.length === 0) {
    return null;
  }
  if (typeof URL !== "undefined" && typeof URL.createObjectURL === "function") {
    return URL.createObjectURL(
      new Blob([Uint8Array.from(preview.bytes)], { type: preview.mimeType })
    );
  }
  return bytesToDataUrl(preview.bytes, preview.mimeType);
}

export function revokePreviewUrl(url: string | null | undefined) {
  if (
    url?.startsWith("blob:") &&
    typeof URL !== "undefined" &&
    typeof URL.revokeObjectURL === "function"
  ) {
    URL.revokeObjectURL(url);
  }
}

export function useEncodedPreviewUrl(preview: EncodedPreview | null, cacheKey: unknown) {
  const [url, setUrl] = useState<string | null>(null);

  useEffect(() => {
    const nextUrl = buildPreviewUrl(preview);
    setUrl(nextUrl);
    return () => {
      revokePreviewUrl(nextUrl);
    };
  }, [cacheKey]);

  return url;
}

export function usePreviewUrl(preview: PreviewMessage | null) {
  return useEncodedPreviewUrl(preview?.preview ?? null, preview?.token ?? null);
}

export const previewToDataUrl = (preview: PreviewMessage | null) => {
  const encoded = preview?.preview ?? null;
  if (!encoded) {
    return null;
  }
  return bytesToDataUrl(encoded.bytes, encoded.mimeType);
};

export const formatPercent = (value: number) => `${(value * 100).toFixed(1)}%`;

export const formatBytes = (value: number) => {
  if (value <= 0) {
    return "0 B";
  }
  if (value >= 1024 * 1024) {
    return `${(value / (1024 * 1024)).toFixed(1)} MB`;
  }
  if (value >= 1024) {
    return `${(value / 1024).toFixed(1)} KB`;
  }
  return `${value} B`;
};
