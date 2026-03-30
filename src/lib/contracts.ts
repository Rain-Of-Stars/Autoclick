export type RuntimeStatus =
  | "Idle"
  | "Starting"
  | "Running"
  | "CoolingDown"
  | "Recovering"
  | "Stopping"
  | "Faulted";

export interface Roi {
  x: number;
  y: number;
  width: number;
  height: number;
}

export interface MonitorRef {
  id: string;
  name: string;
  isPrimary: boolean;
}

export interface CaptureProfile {
  source: "Window" | "Monitor";
  monitor: MonitorRef;
  roi: Roi;
  targetFps: number;
  timeoutMs: number;
  includeCursor: boolean;
  restoreMinimizedNoactivate: boolean;
  restoreMinimizedAfterCapture: boolean;
  windowBorderRequired: boolean;
  screenBorderRequired: boolean;
}

export interface DetectionProfile {
  threshold: number;
  grayscale: boolean;
  multiScale: boolean;
  scales: number[];
  minDetections: number;
  cooldownMs: number;
  earlyExit: boolean;
}

export interface InputPolicy {
  method: "Message" | "Simulate";
  verifyWindowBeforeClick: boolean;
  clickOffsetX: number;
  clickOffsetY: number;
}

export interface RecoveryPolicy {
  enableAutoRecovery: boolean;
  maxRecoveryAttempts: number;
  recoveryCooldownSecs: number;
  autoUpdateTargetByProcess: boolean;
  autoUpdateIntervalMs: number;
}

export interface FinderStrategies {
  processName: boolean;
  processPath: boolean;
  windowTitle: boolean;
  className: boolean;
  fuzzyMatch: boolean;
}

export interface TargetProfile {
  hwnd: number | null;
  processName: string | null;
  processPath: string | null;
  titleContains: string | null;
  className: string | null;
  allowPartialMatch: boolean;
  strategies: FinderStrategies;
}

export interface UiPrefs {
  enableLogging: boolean;
  enableNotifications: boolean;
  autoStartScan: boolean;
  debugMode: boolean;
  saveDebugImages: boolean;
  debugImageDir: string;
}

export interface TemplateRef {
  id: string;
  name: string;
  hash: string;
  sourcePath: string | null;
  storedPath: string | null;
  width: number;
  height: number;
  tags: string[];
  createdAt: string;
}

export interface TemplatePreview {
  mimeType: string;
  bytes: number[];
  width: number;
  height: number;
}

export interface AppConfig {
  schemaVersion: number;
  capture: CaptureProfile;
  detection: DetectionProfile;
  input: InputPolicy;
  recovery: RecoveryPolicy;
  target: TargetProfile;
  ui: UiPrefs;
  templates: TemplateRef[];
  runtimeStatus: RuntimeStatus;
}

export interface AppPaths {
  dataDir: string;
  cacheDir: string;
  logDir: string;
  templatesDir: string;
  debugDir: string;
  dbPath: string;
}

export interface BootstrapInfo {
  appName: string;
  version: string;
  runtimePath: string;
}

export interface WindowRect {
  left: number;
  top: number;
  right: number;
  bottom: number;
}

export interface WindowInfo {
  hwnd: number;
  title: string;
  className: string;
  pid: number;
  exePath: string | null;
  isMinimized: boolean;
  isVisible: boolean;
  rect: WindowRect;
}

export interface MonitorInfo {
  handle: number;
  name: string;
  isPrimary: boolean;
  rect: WindowRect;
  workRect: WindowRect;
  dpi: number;
  scaleFactor: number;
}

export interface LocatorCandidate {
  window: WindowInfo;
  reliability: number;
  reason: string;
}

export interface MatchResult {
  templateId: string;
  templateName: string;
  score: number;
  x: number;
  y: number;
  width: number;
  height: number;
  scale: number;
}

export type RuntimeDecision =
  | "NoMatch"
  | "BelowThreshold"
  | { Pending: number }
  | { CoolingDown: number }
  | { ShouldClick: MatchResult };

export interface PerformanceSnapshot {
  captureFps: number;
  frameIntervalMs: number;
  detectLatencyMs: number;
  previewLatencyMs: number;
  endToEndLatencyMs: number;
  clickCount: number;
  lastScore: number;
  uptimeSecs: number;
}

export interface CaptureSnapshot {
  frameWidth: number;
  frameHeight: number;
  drops: number;
  activeSource: string | null;
}

export interface RecoverySnapshot {
  attempts: number;
  lastReason: string | null;
  nextRetryInMs: number | null;
}

export interface PreviewSnapshot {
  enabled: boolean;
  frameToken: string | null;
  width: number;
  height: number;
}

export interface RuntimeSnapshot {
  status: RuntimeStatus;
  performance: PerformanceSnapshot;
  capture: CaptureSnapshot;
  recovery: RecoverySnapshot;
  preview: PreviewSnapshot;
  lastError: string | null;
}

export interface RuntimeMetricsSnapshot {
  runtime: RuntimeSnapshot;
  recoveryCount: number;
  bufferDrops: number;
  memoryBytesEstimate: number;
}

export interface ClickReport {
  method: "Message" | "Simulate";
  dispatchHwnd: number;
  screenX: number;
  screenY: number;
  clientX: number;
  clientY: number;
  restoredFromMinimized: boolean;
}

export interface EncodedPreview {
  frameId: number;
  width: number;
  height: number;
  mimeType: string;
  bytes: number[];
}

export interface PreviewMessage {
  token: string;
  preview: EncodedPreview;
}

export interface RuntimeControllerSnapshot {
  status: RuntimeStatus;
  metrics: RuntimeMetricsSnapshot;
  preview: PreviewMessage | null;
  activeTarget: LocatorCandidate | null;
  bestMatch: MatchResult | null;
  decision: RuntimeDecision | null;
  lastClick: ClickReport | null;
  lastError: string | null;
}

export interface LogFileEntry {
  name: string;
  path: string;
  sizeBytes: number;
}

export interface DiagnosticsOverview {
  paths: AppPaths;
  runtime: RuntimeControllerSnapshot;
  logs: LogFileEntry[];
}

export interface LegacyImportReport {
  configImported: boolean;
  templatesImported: number;
  warnings: string[];
}

export interface TargetCaptureResult {
  window: WindowInfo | null;
  preview: EncodedPreview;
}

export interface UpdaterStatus {
  configured: boolean;
  pubkeyConfigured: boolean;
  installMode: string;
  reason: string | null;
}

export interface UpdateCheckResult {
  configured: boolean;
  checked: boolean;
  updateAvailable: boolean;
  currentVersion: string;
  latestVersion: string | null;
  body: string | null;
  date: string | null;
  reason: string | null;
}
