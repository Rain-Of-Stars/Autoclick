import { useEffect, useMemo, useState } from "react";
import type { AppConfig } from "../../lib/contracts";
import { useConfigStore } from "../../stores/configStore";

type SectionId = "capture" | "detect" | "input" | "recovery";

type SectionEntry = {
  id: SectionId;
  eyebrow: string;
  title: string;
  navLabel: string;
  summary: string;
};

type FieldProps = {
  label: string;
  children: React.ReactNode;
  hint?: string;
};

type ToggleFieldProps = {
  label: string;
  checked: boolean;
  hint?: string;
  onChange: (checked: boolean) => void;
};

const sectionCatalog: SectionEntry[] = [
  {
    id: "capture",
    eyebrow: "Capture",
    title: "捕获",
    navLabel: "捕获",
    summary: "控制采样来源、ROI、恢复最小化窗口与边界要求。"
  },
  {
    id: "detect",
    eyebrow: "Detect",
    title: "检测",
    navLabel: "检测",
    summary: "定义阈值、灰度、多尺度、连续命中与冷却。"
  },
  {
    id: "input",
    eyebrow: "Input",
    title: "点击",
    navLabel: "点击",
    summary: "控制注入方式、前置校验和点击偏移。"
  },
  {
    id: "recovery",
    eyebrow: "Recovery",
    title: "恢复",
    navLabel: "恢复",
    summary: "约束自动恢复、重试间隔和按进程重新定位。"
  }
];

const inputClassName = "desk-input";

function Field({ label, children, hint }: FieldProps) {
  return (
    <label className="desk-field block space-y-2">
      <span className="desk-field-label block">{label}</span>
      {children}
      {hint ? <p className="desk-field-hint">{hint}</p> : null}
    </label>
  );
}

function ToggleField({ label, checked, hint, onChange }: ToggleFieldProps) {
  return (
    <label className="desk-field flex items-start justify-between gap-3">
      <div className="min-w-0">
        <span className="desk-field-label block">{label}</span>
        {hint ? <p className="desk-field-hint mt-1">{hint}</p> : null}
      </div>
      <input
        className="mt-1 h-4 w-4 shrink-0 accent-[rgb(70,128,186)]"
        type="checkbox"
        checked={checked}
        onChange={(event) => onChange(event.target.checked)}
      />
    </label>
  );
}

function SectionTab(props: {
  active: boolean;
  entry: SectionEntry;
  onClick: () => void;
}) {
  return (
    <button
      type="button"
      className={[
        "border px-3 py-3 text-left transition",
        props.active
          ? "desk-accent-surface"
          : "border-white/10 bg-black/10 text-slate-400 hover:border-white/20 hover:bg-white/[0.03] hover:text-slate-200"
      ].join(" ")}
      onClick={props.onClick}
    >
      <p className="text-[10px] uppercase tracking-[0.28em] text-slate-500">{props.entry.eyebrow}</p>
      <div className="mt-2 flex items-center justify-between gap-3">
        <span className="text-sm font-medium">{props.entry.navLabel}</span>
        <span className="desk-badge border-white/10 text-[10px] text-slate-500">{props.entry.eyebrow}</span>
      </div>
    </button>
  );
}

function buildScaleText(values: number[]) {
  return values.join(", ");
}

export default function TaskConfigPage() {
  const config = useConfigStore((state) => state.config);
  const saveConfig = useConfigStore((state) => state.saveConfig);
  const saving = useConfigStore((state) => state.saving);
  const [draft, setDraft] = useState<AppConfig | null>(null);
  const [activeSection, setActiveSection] = useState<SectionId>("capture");
  const [scaleText, setScaleText] = useState("");

  useEffect(() => {
    if (config) {
      setDraft(config);
      setScaleText(buildScaleText(config.detection.scales));
    }
  }, [config]);

  const diffCount = useMemo(() => {
    if (!config || !draft) {
      return 0;
    }
    return JSON.stringify(config) === JSON.stringify(draft) ? 0 : 1;
  }, [config, draft]);

  const activeEntry = sectionCatalog.find((section) => section.id === activeSection) ?? sectionCatalog[0];

  if (!draft) {
    return <div className="desk-panel p-6 text-slate-400">加载配置中…</div>;
  }

  const renderSection = () => {
    if (activeSection === "capture") {
      return (
        <div className="grid gap-3 xl:grid-cols-2">
          <Field label="捕获源">
            <select
              className={inputClassName}
              value={draft.capture.source}
              onChange={(event) =>
                setDraft({
                  ...draft,
                  capture: {
                    ...draft.capture,
                    source: event.target.value as AppConfig["capture"]["source"]
                  }
                })
              }
            >
              <option value="Window">窗口</option>
              <option value="Monitor">显示器</option>
            </select>
          </Field>
          <Field label="监视器" hint="显示器源时使用该字段，窗口源时作为参考信息保留。">
            <input className={inputClassName} readOnly value={draft.capture.monitor.name} />
          </Field>
          <Field label="目标 FPS">
            <input
              className={inputClassName}
              type="number"
              value={draft.capture.targetFps}
              onChange={(event) =>
                setDraft({
                  ...draft,
                  capture: { ...draft.capture, targetFps: Number(event.target.value) }
                })
              }
            />
          </Field>
          <Field label="超时 (ms)">
            <input
              className={inputClassName}
              type="number"
              value={draft.capture.timeoutMs}
              onChange={(event) =>
                setDraft({
                  ...draft,
                  capture: { ...draft.capture, timeoutMs: Number(event.target.value) }
                })
              }
            />
          </Field>
          <Field label="ROI X">
            <input
              className={inputClassName}
              type="number"
              value={draft.capture.roi.x}
              onChange={(event) =>
                setDraft({
                  ...draft,
                  capture: {
                    ...draft.capture,
                    roi: { ...draft.capture.roi, x: Number(event.target.value) }
                  }
                })
              }
            />
          </Field>
          <Field label="ROI Y">
            <input
              className={inputClassName}
              type="number"
              value={draft.capture.roi.y}
              onChange={(event) =>
                setDraft({
                  ...draft,
                  capture: {
                    ...draft.capture,
                    roi: { ...draft.capture.roi, y: Number(event.target.value) }
                  }
                })
              }
            />
          </Field>
          <Field label="ROI 宽">
            <input
              className={inputClassName}
              type="number"
              value={draft.capture.roi.width}
              onChange={(event) =>
                setDraft({
                  ...draft,
                  capture: {
                    ...draft.capture,
                    roi: { ...draft.capture.roi, width: Number(event.target.value) }
                  }
                })
              }
            />
          </Field>
          <Field label="ROI 高">
            <input
              className={inputClassName}
              type="number"
              value={draft.capture.roi.height}
              onChange={(event) =>
                setDraft({
                  ...draft,
                  capture: {
                    ...draft.capture,
                    roi: { ...draft.capture.roi, height: Number(event.target.value) }
                  }
                })
              }
            />
          </Field>
          <ToggleField
            label="包含鼠标"
            hint="需要验证指针遮挡时开启。"
            checked={draft.capture.includeCursor}
            onChange={(checked) =>
              setDraft({
                ...draft,
                capture: { ...draft.capture, includeCursor: checked }
              })
            }
          />
          <ToggleField
            label="恢复最小化窗口"
            hint="高性能窗口捕获通常需要在捕获前恢复窗口。"
            checked={draft.capture.restoreMinimizedNoactivate}
            onChange={(checked) =>
              setDraft({
                ...draft,
                capture: { ...draft.capture, restoreMinimizedNoactivate: checked }
              })
            }
          />
          <ToggleField
            label="捕获后重新最小化"
            hint="仅适用于明确要求保持窗口后台隐藏的场景。"
            checked={draft.capture.restoreMinimizedAfterCapture}
            onChange={(checked) =>
              setDraft({
                ...draft,
                capture: { ...draft.capture, restoreMinimizedAfterCapture: checked }
              })
            }
          />
          <ToggleField
            label="要求窗口边框"
            hint="窗口定位链路需要排除无框干扰时启用。"
            checked={draft.capture.windowBorderRequired}
            onChange={(checked) =>
              setDraft({
                ...draft,
                capture: { ...draft.capture, windowBorderRequired: checked }
              })
            }
          />
          <ToggleField
            label="要求屏幕边框"
            hint="显示器源模式下用于约束可见边界。"
            checked={draft.capture.screenBorderRequired}
            onChange={(checked) =>
              setDraft({
                ...draft,
                capture: { ...draft.capture, screenBorderRequired: checked }
              })
            }
          />
        </div>
      );
    }

    if (activeSection === "detect") {
      return (
        <div className="grid gap-3 xl:grid-cols-2">
          <Field label="阈值" hint="建议优先在 0.80 到 0.95 区间内调优。">
            <input
              className={inputClassName}
              type="number"
              step="0.01"
              value={draft.detection.threshold}
              onChange={(event) =>
                setDraft({
                  ...draft,
                  detection: { ...draft.detection, threshold: Number(event.target.value) }
                })
              }
            />
          </Field>
          <Field label="连续命中帧数">
            <input
              className={inputClassName}
              type="number"
              value={draft.detection.minDetections}
              onChange={(event) =>
                setDraft({
                  ...draft,
                  detection: { ...draft.detection, minDetections: Number(event.target.value) }
                })
              }
            />
          </Field>
          <Field label="冷却时间 (ms)">
            <input
              className={inputClassName}
              type="number"
              value={draft.detection.cooldownMs}
              onChange={(event) =>
                setDraft({
                  ...draft,
                  detection: { ...draft.detection, cooldownMs: Number(event.target.value) }
                })
              }
            />
          </Field>
          <Field label="尺度列表" hint="使用逗号分隔，例如 1, 0.95, 1.05">
            <input
              className={inputClassName}
              value={scaleText}
              onChange={(event) => setScaleText(event.target.value)}
              onBlur={(event) => {
                const values = event.target.value
                  .split(",")
                  .map((value) => Number(value.trim()))
                  .filter((value) => !Number.isNaN(value) && value > 0);
                const nextValues = values.length > 0 ? values : [1];
                setScaleText(buildScaleText(nextValues));
                setDraft({
                  ...draft,
                  detection: { ...draft.detection, scales: nextValues }
                });
              }}
            />
          </Field>
          <ToggleField
            label="灰度匹配"
            hint="能减少颜色波动，但会牺牲部分细节信息。"
            checked={draft.detection.grayscale}
            onChange={(checked) =>
              setDraft({
                ...draft,
                detection: { ...draft.detection, grayscale: checked }
              })
            }
          />
          <ToggleField
            label="多尺度匹配"
            hint="开启后更稳，但检测耗时会增长。"
            checked={draft.detection.multiScale}
            onChange={(checked) =>
              setDraft({
                ...draft,
                detection: { ...draft.detection, multiScale: checked }
              })
            }
          />
          <ToggleField
            label="早停策略"
            hint="找到高可信结果后提前退出当前帧的后续检测。"
            checked={draft.detection.earlyExit}
            onChange={(checked) =>
              setDraft({
                ...draft,
                detection: { ...draft.detection, earlyExit: checked }
              })
            }
          />
        </div>
      );
    }

    if (activeSection === "input") {
      return (
        <div className="grid gap-3 xl:grid-cols-2">
          <Field label="点击策略">
            <select
              className={inputClassName}
              value={draft.input.method}
              onChange={(event) =>
                setDraft({
                  ...draft,
                  input: {
                    ...draft.input,
                    method: event.target.value as AppConfig["input"]["method"]
                  }
                })
              }
            >
              <option value="Message">PostMessage</option>
              <option value="Simulate">SendInput</option>
            </select>
          </Field>
          <ToggleField
            label="点击前校验窗口"
            hint="桌面场景建议保持开启，避免误击中错误句柄。"
            checked={draft.input.verifyWindowBeforeClick}
            onChange={(checked) =>
              setDraft({
                ...draft,
                input: { ...draft.input, verifyWindowBeforeClick: checked }
              })
            }
          />
          <Field label="X 偏移">
            <input
              className={inputClassName}
              type="number"
              value={draft.input.clickOffsetX}
              onChange={(event) =>
                setDraft({
                  ...draft,
                  input: { ...draft.input, clickOffsetX: Number(event.target.value) }
                })
              }
            />
          </Field>
          <Field label="Y 偏移">
            <input
              className={inputClassName}
              type="number"
              value={draft.input.clickOffsetY}
              onChange={(event) =>
                setDraft({
                  ...draft,
                  input: { ...draft.input, clickOffsetY: Number(event.target.value) }
                })
              }
            />
          </Field>
        </div>
      );
    }

    return (
      <div className="grid gap-3 xl:grid-cols-2">
        <ToggleField
          label="自动恢复"
          hint="故障或捕获中断后尝试自动拉回扫描链路。"
          checked={draft.recovery.enableAutoRecovery}
          onChange={(checked) =>
            setDraft({
              ...draft,
              recovery: { ...draft.recovery, enableAutoRecovery: checked }
            })
          }
        />
        <Field label="最大恢复次数">
          <input
            className={inputClassName}
            type="number"
            value={draft.recovery.maxRecoveryAttempts}
            onChange={(event) =>
              setDraft({
                ...draft,
                recovery: {
                  ...draft.recovery,
                  maxRecoveryAttempts: Number(event.target.value)
                }
              })
            }
          />
        </Field>
        <Field label="恢复冷却 (s)">
          <input
            className={inputClassName}
            type="number"
            step="0.1"
            value={draft.recovery.recoveryCooldownSecs}
            onChange={(event) =>
              setDraft({
                ...draft,
                recovery: {
                  ...draft.recovery,
                  recoveryCooldownSecs: Number(event.target.value)
                }
              })
            }
          />
        </Field>
        <Field label="自动更新间隔 (ms)">
          <input
            className={inputClassName}
            type="number"
            value={draft.recovery.autoUpdateIntervalMs}
            onChange={(event) =>
              setDraft({
                ...draft,
                recovery: {
                  ...draft.recovery,
                  autoUpdateIntervalMs: Number(event.target.value)
                }
              })
            }
          />
        </Field>
        <ToggleField
          label="按进程自动更新目标"
          hint="目标窗口句柄频繁重建时启用。"
          checked={draft.recovery.autoUpdateTargetByProcess}
          onChange={(checked) =>
            setDraft({
              ...draft,
              recovery: {
                ...draft.recovery,
                autoUpdateTargetByProcess: checked
              }
            })
          }
        />
      </div>
    );
  };

  return (
    <section className="desk-page">
      <header className="desk-page-header">
        <div className="desk-page-header-main">
          <div className="min-w-0">
            <p className="desk-eyebrow">Task Config</p>
            <div className="mt-2 flex flex-wrap items-center gap-3">
              <h1 className="desk-title">任务配置</h1>
              <span className="desk-chip">变更 {diffCount}</span>
              <span className="desk-chip">当前章节 {activeEntry.title}</span>
            </div>
          </div>
          <div className="desk-toolbar justify-end">
            <button type="button" className="desk-button desk-button-neutral" onClick={() => setDraft(config)}>
              撤销修改
            </button>
            <button
              type="button"
              className="desk-button desk-button-primary disabled:cursor-not-allowed disabled:opacity-50"
              disabled={saving}
              onClick={() => void saveConfig(draft)}
            >
              保存配置
            </button>
          </div>
        </div>
        <div className="desk-statline shrink-0">
          <span>捕获源: {draft.capture.source}</span>
          <span>阈值: {draft.detection.threshold.toFixed(2)}</span>
          <span>点击策略: {draft.input.method}</span>
          <span>自动恢复: {draft.recovery.enableAutoRecovery ? "开启" : "关闭"}</span>
        </div>
      </header>

      <div className="desk-page-body grid gap-4 xl:grid-cols-[190px_minmax(0,1fr)_300px]">
        <aside className="desk-panel flex min-h-0 flex-col p-3">
          <div className="shrink-0 border-b border-white/10 pb-3">
            <p className="desk-eyebrow">Section</p>
            <p className="mt-2 text-xs leading-5 text-slate-500">一次只编辑一个链路阶段，减少整页滚动。</p>
          </div>
          <div className="desk-scroll mt-3 grid gap-2">
            {sectionCatalog.map((entry) => (
              <SectionTab
                key={entry.id}
                active={activeSection === entry.id}
                entry={entry}
                onClick={() => setActiveSection(entry.id)}
              />
            ))}
          </div>
        </aside>

        <article className="desk-panel flex min-h-0 flex-col p-4">
          <div className="shrink-0 border-b border-white/10 pb-3">
            <p className="desk-eyebrow">{activeEntry.eyebrow}</p>
            <div className="mt-2 flex flex-wrap items-center justify-between gap-3">
              <h2 className="text-[1.4rem] font-semibold tracking-[0.02em] text-slate-100">
                {activeEntry.title}参数
              </h2>
              <span className="desk-badge border-white/10 text-slate-500">{activeEntry.eyebrow}</span>
            </div>
            <p className="mt-2 text-xs leading-6 text-slate-500">{activeEntry.summary}</p>
          </div>
          <div className="desk-scroll mt-4">{renderSection()}</div>
        </article>

        <aside className="desk-panel-strong flex min-h-0 flex-col p-4">
          <div className="shrink-0 border-b border-white/10 pb-3">
            <p className="desk-eyebrow">Summary</p>
            <h2 className="mt-2 text-lg font-semibold text-slate-100">配置摘要</h2>
          </div>
          <div className="desk-scroll mt-4 space-y-3">
            <div className="desk-field">
              <p className="desk-field-label">当前配置</p>
              <div className="mt-2 grid gap-2 text-xs leading-6 text-slate-300">
                <p>ROI: {draft.capture.roi.x}, {draft.capture.roi.y}, {draft.capture.roi.width} × {draft.capture.roi.height}</p>
                <p>尺度: {buildScaleText(draft.detection.scales)}</p>
                <p>恢复上限: {draft.recovery.maxRecoveryAttempts}</p>
                <p>自动更新目标: {draft.recovery.autoUpdateTargetByProcess ? "开启" : "关闭"}</p>
              </div>
            </div>
            <div className="desk-field">
              <p className="desk-field-label">目标筛选</p>
              <div className="mt-2 grid gap-2 text-xs leading-6 text-slate-300">
                <p>进程名: {draft.target.processName ?? "--"}</p>
                <p>标题关键字: {draft.target.titleContains ?? "--"}</p>
                <p>类名: {draft.target.className ?? "--"}</p>
                <p>模糊匹配: {draft.target.allowPartialMatch ? "允许" : "禁用"}</p>
              </div>
            </div>
            <div className="desk-field">
              <p className="desk-field-label">调优提示</p>
              <div className="mt-2 space-y-2 text-xs leading-6 text-slate-400">
                <p>1. 先在捕获页固定 ROI，再微调阈值和尺度。</p>
                <p>2. 需要高稳定性时优先提高连续命中帧数，而不是只拉高阈值。</p>
                <p>3. WGC 目标窗口最小化时，建议保持恢复最小化窗口开启。</p>
              </div>
            </div>
          </div>
        </aside>
      </div>
    </section>
  );
}
