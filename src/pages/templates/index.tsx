import { useEffect, useMemo, useRef, useState, type CSSProperties, type ClipboardEvent as ReactClipboardEvent } from "react";
import { tauriClient } from "../../lib/tauriClient";
import { useConfigStore } from "../../stores/configStore";

// ==========================================
// SVGs & Icons
// ==========================================
const Icons = {
  Search: () => (
    <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" strokeWidth={2} stroke="currentColor" className="w-4 h-4">
      <path strokeLinecap="round" strokeLinejoin="round" d="m21 21-5.197-5.197m0 0A7.5 7.5 0 1 0 5.196 5.196a7.5 7.5 0 0 0 10.607 10.607Z" />
    </svg>
  ),
  Refresh: () => (
    <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" strokeWidth={2} stroke="currentColor" className="w-3.5 h-3.5">
      <path strokeLinecap="round" strokeLinejoin="round" d="M16.023 9.348h4.992v-.001M2.985 19.644v-4.992m0 0h4.992m-4.993 0 3.181 3.183a8.25 8.25 0 0 0 13.803-3.7M4.031 9.865a8.25 8.25 0 0 1 13.803-3.7l3.181 3.182m0-4.991v4.99" />
    </svg>
  ),
  Trash: () => (
    <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" strokeWidth={1.5} stroke="currentColor" className="w-4 h-4">
      <path strokeLinecap="round" strokeLinejoin="round" d="m14.74 9-.346 9m-4.788 0L9.26 9m9.968-3.21c.342.052.682.107 1.022.166m-1.022-.165L18.16 19.673a2.25 2.25 0 0 1-2.244 2.077H8.084a2.25 2.25 0 0 1-2.244-2.077L4.772 5.79m14.456 0a48.108 48.108 0 0 0-3.478-.397m-12 .562c.34-.059.68-.114 1.022-.165m0 0a48.11 48.11 0 0 1 3.478-.397m7.5 0v-.916c0-1.18-.91-2.164-2.09-2.201a51.964 51.964 0 0 0-3.32 0c-1.18.037-2.09 1.022-2.09 2.201v.916m7.5 0a48.667 48.667 0 0 0-7.5 0" />
    </svg>
  ),
  Tag: () => (
    <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" strokeWidth={2} stroke="currentColor" className="w-3.5 h-3.5">
      <path strokeLinecap="round" strokeLinejoin="round" d="M9.568 3H5.25A2.25 2.25 0 0 0 3 5.25v4.318c0 .597.237 1.17.659 1.591l9.581 9.581c.699.699 1.78.872 2.607.33a18.095 18.095 0 0 0 5.223-5.223c.542-.827.369-1.908-.33-2.607L11.16 3.66A2.25 2.25 0 0 0 9.568 3Z" />
      <path strokeLinecap="round" strokeLinejoin="round" d="M6 6h.008v.008H6V6Z" />
    </svg>
  ),
  ImageSize: () => (
    <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" strokeWidth={2} stroke="currentColor" className="w-3 h-3">
      <path strokeLinecap="round" strokeLinejoin="round" d="M3.75 3.75v4.5m0-4.5h4.5m-4.5 0L9 9M3.75 20.25v-4.5m0 4.5h4.5m-4.5 0L9 15M20.25 3.75h-4.5m4.5 0v4.5m0-4.5L15 9m5.25 11.25h-4.5m4.5 0v-4.5m0 4.5L15 15" />
    </svg>
  ),
  ImportIcon: () => (
    <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" strokeWidth={2} stroke="currentColor" className="w-4 h-4">
      <path strokeLinecap="round" strokeLinejoin="round" d="M3 16.5v2.25A2.25 2.25 0 0 0 5.25 21h13.5A2.25 2.25 0 0 0 21 18.75V16.5M16.5 12 12 16.5m0 0L7.5 12m4.5 4.5V3" />
    </svg>
  ),
  Clipboard: () => (
    <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" strokeWidth={1.5} stroke="currentColor" className="w-5 h-5">
      <path strokeLinecap="round" strokeLinejoin="round" d="M9 12h3.75M9 15h3.75M9 18h3.75m3 .75H18a2.25 2.25 0 0 0 2.25-2.25V6.108c0-1.135-.845-2.098-1.976-2.192a48.424 48.424 0 0 0-1.123-.08m-5.801 0c-.065.21-.1.433-.1.664 0 .414.336.75.75.75h4.5a.75.75 0 0 0 .75-.75 2.25 2.25 0 0 0-.1-.664m-5.8 0A2.251 2.251 0 0 1 13.5 2.25H15c1.012 0 1.867.668 2.15 1.586m-5.8 0c-.376.023-.75.05-1.124.08C9.095 4.01 8.25 4.973 8.25 6.108V8.25m0 0H4.875c-.621 0-1.125.504-1.125 1.125v11.25c0 .621.504 1.125 1.125 1.125h9.75c.621 0 1.125-.504 1.125-1.125V9.375c0-.621-.504-1.125-1.125-1.125H8.25ZM6.75 12h.008v.008H6.75V12Zm0 3h.008v.008H6.75V15Zm0 3h.008v.008H6.75V18Z" />
    </svg>
  ),
  Hash: () => (
    <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" strokeWidth={2} stroke="currentColor" className="w-3 h-3">
      <path strokeLinecap="round" strokeLinejoin="round" d="M5.25 8.25h15m-16.5 7.5h15m-1.8-13.5-3.9 19.5m-2.1-19.5-3.9 19.5" />
    </svg>
  ),
  Photo: () => (
    <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" strokeWidth={1.5} stroke="currentColor" className="w-5 h-5">
      <path strokeLinecap="round" strokeLinejoin="round" d="m2.25 15.75 5.159-5.159a2.25 2.25 0 0 1 3.182 0l5.159 5.159m-1.5-1.5 1.409-1.409a2.25 2.25 0 0 1 3.182 0l2.909 2.909m-18 3.75h16.5a1.5 1.5 0 0 0 1.5-1.5V6a1.5 1.5 0 0 0-1.5-1.5H3.75A1.5 1.5 0 0 0 2.25 6v12a1.5 1.5 0 0 0 1.5 1.5Zm10.5-11.25h.008v.008h-.008V8.25Zm.375 0a.375.375 0 1 1-.75 0 .375.375 0 0 1 .75 0Z" />
    </svg>
  )
};

// ==========================================
// Shared Helpers
// ==========================================
function formatCreatedAt(value: string) {
  const date = new Date(value);
  if (Number.isNaN(date.getTime())) return value;
  return date.toLocaleString("zh-CN", {
    year: "numeric", month: "2-digit", day: "2-digit",
    hour: "2-digit", minute: "2-digit"
  });
}

function parseTags(value: string) {
  return value.split(/[，,]/).map((item) => item.trim()).filter(Boolean);
}

function buildPastedTemplateName() {
  const now = new Date();
  const parts = [
    now.getFullYear().toString(),
    `${now.getMonth() + 1}`.padStart(2, "0"),
    `${now.getDate()}`.padStart(2, "0"),
    `${now.getHours()}`.padStart(2, "0"),
    `${now.getMinutes()}`.padStart(2, "0"),
    `${now.getSeconds()}`.padStart(2, "0")
  ];
  return `粘_${parts.join("")}`;
}

function formatBytes(sizeBytes: number) {
  if (sizeBytes < 1024) return `${sizeBytes} B`;
  if (sizeBytes < 1024 * 1024) return `${(sizeBytes / 1024).toFixed(1)} KB`;
  return `${(sizeBytes / (1024 * 1024)).toFixed(2)} MB`;
}

function isEditableElement(target: EventTarget | null) {
  return (
    target instanceof HTMLInputElement ||
    target instanceof HTMLTextAreaElement ||
    target instanceof HTMLSelectElement
  );
}

function bytesToDataUrl(bytes: number[], mimeType: string) {
  if (bytes.length === 0) return null;
  const binary = Uint8Array.from(bytes);
  let text = "";
  binary.forEach((value) => { text += String.fromCharCode(value); });
  return `data:${mimeType};base64,${btoa(text)}`;
}

function createAssetUrl(bytes: number[], mimeType: string) {
  if (typeof URL !== "undefined" && typeof URL.createObjectURL === "function") {
    return URL.createObjectURL(new Blob([Uint8Array.from(bytes)], { type: mimeType }));
  }
  return bytesToDataUrl(bytes, mimeType);
}

function revokePreviewUrl(url: string | null | undefined) {
  if (url?.startsWith("blob:")) URL.revokeObjectURL(url);
}

function shouldPixelatePreview(width?: number | null, height?: number | null) {
  if (!width || !height) return false;
  return Math.max(width, height) <= 96;
}

const previewCheckerboardStyle: CSSProperties = {
  backgroundImage: [
    "linear-gradient(45deg, rgba(15, 23, 42, 0.06) 25%, transparent 25%, transparent 75%, rgba(15, 23, 42, 0.06) 75%, rgba(15, 23, 42, 0.06))",
    "linear-gradient(45deg, rgba(15, 23, 42, 0.06) 25%, transparent 25%, transparent 75%, rgba(15, 23, 42, 0.06) 75%, rgba(15, 23, 42, 0.06))"
  ].join(","),
  backgroundPosition: "0 0, 12px 12px",
  backgroundSize: "24px 24px"
};

const MAX_PASTED_TEMPLATE_BYTES = 25 * 1024 * 1024;

interface AssetPreviewCanvasProps {
  src: string;
  alt: string;
  width?: number | null;
  height?: number | null;
  containerClassName?: string;
  imageClassName?: string;
  onError?: () => void;
}

function AssetPreviewCanvas({
  src,
  alt,
  width,
  height,
  containerClassName,
  imageClassName,
  onError
}: AssetPreviewCanvasProps) {
  const pixelated = shouldPixelatePreview(width, height);

  return (
    <div
      className={[
        "relative overflow-hidden rounded-[22px] border border-slate-300/20",
        "bg-[linear-gradient(145deg,rgba(248,250,252,0.97),rgba(226,232,240,0.86))]",
        "shadow-[inset_0_1px_0_rgba(255,255,255,0.7),0_18px_42px_rgba(2,6,23,0.28)]",
        containerClassName ?? ""
      ].join(" ")}
    >
      <div className="absolute inset-0 opacity-80" style={previewCheckerboardStyle} />
      <div className="absolute inset-0 bg-[radial-gradient(circle_at_top,rgba(255,255,255,0.78),transparent_58%)]" />
      <img
        src={src}
        alt={alt}
        className={[
          "relative z-10 h-full w-full object-contain drop-shadow-[0_12px_28px_rgba(15,23,42,0.28)]",
          imageClassName ?? ""
        ].join(" ")}
        style={{ imageRendering: pixelated ? "pixelated" : "auto" }}
        onError={onError}
      />
    </div>
  );
}

interface PastedTemplateDraft {
  bytes: number[];
  mimeType: string;
  name: string;
  previewUrl: string;
  sizeBytes: number;
  tags: string;
}

interface TemplatePreviewAsset {
  mimeType: string;
  url: string;
  width: number;
  height: number;
}

// ==========================================
// Main Component
// ==========================================
export default function TemplatesPage() {
  const templates = useConfigStore((state) => state.templates);
  const saving = useConfigStore((state) => state.saving);
  const importTemplate = useConfigStore((state) => state.importTemplate);
  const importPastedTemplate = useConfigStore((state) => state.importPastedTemplate);
  const removeTemplate = useConfigStore((state) => state.removeTemplate);
  const renameTemplate = useConfigStore((state) => state.renameTemplate);
  const refreshTemplates = useConfigStore((state) => state.refreshTemplates);

  const [filePath, setFilePath] = useState("");
  const [tags, setTags] = useState("");
  const [keyword, setKeyword] = useState("");
  const [activeTag, setActiveTag] = useState<string | null>(null);
  const [selectedIds, setSelectedIds] = useState<string[]>([]);
  const [selectedPreview, setSelectedPreview] = useState<TemplatePreviewAsset | null>(null);
  const [selectedPreviewBusy, setSelectedPreviewBusy] = useState(false);
  const [selectedPreviewError, setSelectedPreviewError] = useState<string | null>(null);
  const [pasteDraft, setPasteDraft] = useState<PastedTemplateDraft | null>(null);
  const [pasteBusy, setPasteBusy] = useState(false);
  const [pasteError, setPasteError] = useState<string | null>(null);
  const pastePreviewUrlRef = useRef<string | null>(null);
  const templatePreviewCacheRef = useRef<Map<string, TemplatePreviewAsset>>(new Map());

  const updatePasteDraft = (nextDraft: PastedTemplateDraft | null) => {
    revokePreviewUrl(pastePreviewUrlRef.current);
    pastePreviewUrlRef.current = nextDraft?.previewUrl ?? null;
    setPasteDraft(nextDraft);
  };

  const tagOptions = useMemo(() => 
    Array.from(new Set(templates.flatMap((template) => template.tags.map((tag) => tag.trim()).filter(Boolean))))
      .sort((l, r) => l.localeCompare(r, "zh-CN")),
    [templates]
  );

  const filteredTemplates = useMemo(() => {
    const lowerKeyword = keyword.trim().toLowerCase();
    return templates.filter((template) => {
      const matchesKeyword = !lowerKeyword || [
        template.name, template.hash, template.sourcePath ?? "", template.storedPath ?? "", ...template.tags
      ].join(" ").toLowerCase().includes(lowerKeyword);
      const matchesTag = !activeTag || template.tags.includes(activeTag);
      return matchesKeyword && matchesTag;
    });
  }, [activeTag, keyword, templates]);

  const selectedTemplate = filteredTemplates.find((t) => t.id === selectedIds[0]) 
    ?? templates.find((t) => t.id === selectedIds[0]) 
    ?? (filteredTemplates.length > 0 ? filteredTemplates[0] : null);

  const totalPixels = useMemo(() => templates.reduce((sum, t) => sum + t.width * t.height, 0), [templates]);
  const selectedSet = useMemo(() => new Set(selectedIds), [selectedIds]);

  useEffect(() => {
    return () => {
      revokePreviewUrl(pastePreviewUrlRef.current);
      for (const asset of templatePreviewCacheRef.current.values()) revokePreviewUrl(asset.url);
    };
  }, []);

  useEffect(() => {
    const validIds = new Set(templates.map((t) => t.id));
    for (const [id, asset] of templatePreviewCacheRef.current.entries()) {
      if (!validIds.has(id)) {
        revokePreviewUrl(asset.url);
        templatePreviewCacheRef.current.delete(id);
      }
    }
  }, [templates]);

  useEffect(() => {
    let disposed = false;
    const selectedTemplateId = selectedTemplate?.id ?? null;
    if (!selectedTemplateId) {
      setSelectedPreview(null);
      setSelectedPreviewBusy(false);
      setSelectedPreviewError(null);
      return () => { disposed = true; };
    }

    const cachedAsset = templatePreviewCacheRef.current.get(selectedTemplateId);
    if (cachedAsset) {
      setSelectedPreview(cachedAsset);
      setSelectedPreviewBusy(false);
      setSelectedPreviewError(null);
      return () => { disposed = true; };
    }

    setSelectedPreview(null);
    setSelectedPreviewBusy(true);
    setSelectedPreviewError(null);

    void tauriClient.getTemplatePreview(selectedTemplateId)
      .then((preview) => {
        const url = createAssetUrl(preview.bytes, preview.mimeType);
        if (!url) throw new Error("模板预览提取失败");
        const asset = { mimeType: preview.mimeType, url, width: preview.width, height: preview.height };
        if (disposed) { revokePreviewUrl(asset.url); return; }
        templatePreviewCacheRef.current.set(selectedTemplateId, asset);
        setSelectedPreview(asset);
      })
      .catch((err) => { if (!disposed) setSelectedPreviewError(err instanceof Error ? err.message : "图片读取失败"); })
      .finally(() => { if (!disposed) setSelectedPreviewBusy(false); });

    return () => { disposed = true; };
  }, [selectedTemplate?.id]);

  const buildDraftFromFile = async (file: File) => {
    const bytes = Array.from(new Uint8Array(await file.arrayBuffer()));
    return {
      bytes, mimeType: file.type || "image/png", name: buildPastedTemplateName(),
      previewUrl: URL.createObjectURL(file), sizeBytes: file.size, tags: ""
    } satisfies PastedTemplateDraft;
  };

  const handlePastedFile = async (file: File) => {
    setPasteBusy(true); setPasteError(null);
    try {
      if (file.size <= 0) {
        setPasteError("未能识别剪贴板中的图片内容");
        return;
      }
      if (file.size > MAX_PASTED_TEMPLATE_BYTES) {
        setPasteError("剪贴板图片过大，请压缩后再导入");
        return;
      }
      updatePasteDraft(await buildDraftFromFile(file));
      setSelectedIds([]);
    } catch (err) {
      setPasteError(err instanceof Error ? err.message : "读取粘贴源失败");
    } finally { setPasteBusy(false); }
  };

  useEffect(() => {
    const onPaste = (e: ClipboardEvent) => {
      if (isEditableElement(e.target)) return;
      const file = Array.from(e.clipboardData?.items ?? []).find(i => i.kind === "file" && i.type.startsWith("image/"))?.getAsFile();
      if (!file) return;
      e.preventDefault();
      void handlePastedFile(file);
    };
    window.addEventListener("paste", onPaste);
    return () => window.removeEventListener("paste", onPaste);
  }, []);

  const handlePasteAreaPaste = (e: ReactClipboardEvent<HTMLDivElement>) => {
    const file = Array.from(e.clipboardData.items).find(i => i.kind === "file" && i.type.startsWith("image/"))?.getAsFile();
    if (!file) {
      setPasteError("未能识别剪贴板中的图片内容");
      return;
    }
    e.preventDefault();
    void handlePastedFile(file);
  };

  const handleBatchRemove = async () => {
    for (const id of selectedIds) await removeTemplate(id);
    setSelectedIds([]);
  };

  const handleConfirmPaste = async () => {
    if (!pasteDraft) {
      setPasteError("暂无有效图片");
      return;
    }
    const nextName = pasteDraft.name.trim();
    if (!nextName) {
      setPasteError("输入有效模板名称");
      return;
    }
    setPasteBusy(true); setPasteError(null);
    const created = await importPastedTemplate(nextName, parseTags(pasteDraft.tags), pasteDraft.bytes);
    setPasteBusy(false);
    if (!created) {
      setPasteError("模板入库未成功完成");
      return;
    }
    setSelectedIds([created.id]);
    updatePasteDraft(null);
  };

  // ==========================================
  // Render Layout
  // ==========================================
  return (
    <section className="desk-page">
      <header className="desk-page-header">
        <div className="desk-page-header-main">
          <div className="min-w-0">
            <p className="desk-eyebrow">Vault</p>
            <div className="mt-2 flex flex-wrap items-center gap-3">
              <h1 className="desk-title">资产模板库</h1>
            </div>
          </div>
          <div className="desk-toolbar justify-end">
            <button
              title="刷新库列表"
              className="desk-button desk-button-neutral"
              onClick={() => void refreshTemplates()}
            >
              <Icons.Refresh />
              <span>刷新</span>
            </button>
            {selectedIds.length > 0 && (
              <button
                className="desk-button desk-button-danger"
                onClick={() => void handleBatchRemove()}
                disabled={saving}
              >
                <Icons.Trash />
                <span>删除选中 ({selectedIds.length})</span>
              </button>
            )}
          </div>
        </div>
        <div className="desk-statline shrink-0">
          <span>总数: {templates.length}</span>
          <span>尺寸预算: {(totalPixels / 1_000_000).toFixed(2)}m</span>
          <span>标签族: {tagOptions.length}</span>
        </div>
      </header>

      {/* 2. Main Content Split */}
      <main className="desk-page-body">
        <div className="flex flex-1 h-full overflow-hidden desk-panel border-white/10 p-0 rounded-xl relative">
          {/* ===================== Left Side: Library & Lists ===================== */}
          <div className="flex flex-col w-full lg:w-[60%] lg:flex-1 border-r border-white/10 relative z-10 bg-black/20">
          {/* Action Bar (Search & Path Import) */}
          <div className="p-4 border-b border-white/5 space-y-3 shrink-0">
            {/* Direct File Loading */}
            <div className="flex items-center bg-black/20 border border-white/10 rounded-lg p-1 focus-within:border-accent/40 focus-within:bg-black/30 transition-all">
               <div className="pl-3 pr-1 text-slate-500"><Icons.ImportIcon /></div>
               <input 
                 className="flex-1 bg-transparent border-none px-2 text-[13px] text-white outline-none placeholder:text-slate-500" 
                 placeholder="输入待导入图片路径 (例如 assets/btn_ok.png)" 
                 value={filePath} onChange={(e) => setFilePath(e.target.value)}
               />
               <div className="w-px h-4 bg-white/10 mx-2" />
               <input 
                 className="w-1/4 bg-transparent border-none px-2 text-[13px] text-white outline-none placeholder:text-slate-500" 
                 placeholder="预设标签..." 
                 value={tags} onChange={(e) => setTags(e.target.value)}
               />
               <button 
                 className="bg-accent/20 hover:bg-accent/30 text-accent font-medium px-4 py-1.5 rounded-md text-[13px] transition-colors ml-1 disabled:opacity-40 disabled:cursor-not-allowed"
                 disabled={saving || !filePath.trim()}
                 onClick={() => { void importTemplate(filePath.trim(), parseTags(tags)); setFilePath(""); setTags(""); }}
               >
                 导入特征
               </button>
            </div>

            {/* Smart Search */}
            <div className="flex gap-3">
              <div className="relative flex items-center flex-1">
                 <div className="absolute left-3 text-slate-500"><Icons.Search /></div>
                 <input 
                   className="w-full bg-white/[0.03] border border-transparent rounded-lg py-2 pl-9 pr-4 text-[13px] text-slate-100 placeholder:text-slate-500 outline-none focus:bg-black/30 focus:border-accent/50 transition-all ring-1 ring-white/5" 
                   placeholder="快速定位哈希、名称或路径..." 
                   value={keyword} onChange={(e) => setKeyword(e.target.value)}
                 />
              </div>
            </div>

            {/* Tags Ribbon */}
            <div className="flex items-center gap-2 overflow-x-auto scrollbar-none pb-1">
               <button 
                 className={`shrink-0 px-3 py-1 rounded-full text-xs font-medium border transition-colors ${activeTag === null ? "bg-accent/20 border-accent/40 text-accent shadow-[0_0_10px_rgba(70,128,186,0.15)]" : "bg-transparent border-white/10 text-muted hover:border-white/30 hover:text-white"}`}
                 onClick={() => setActiveTag(null)}
               >
                 全部合集
               </button>
               {tagOptions.map(tag => (
                 <button 
                   key={tag}
                   className={`shrink-0 px-3 py-1 rounded-full text-xs font-medium border transition-colors flex items-center gap-1.5 ${activeTag === tag ? "bg-accent/20 border-accent/40 text-accent shadow-[0_0_10px_rgba(70,128,186,0.15)]" : "bg-transparent border-white/10 text-muted hover:border-white/30 hover:text-white"}`}
                   onClick={() => setActiveTag((cur) => cur === tag ? null : tag)}
                 >
                   <Icons.Tag />
                   {tag}
                 </button>
               ))}
            </div>
          </div>

          {/* Table List Layout for modern feeling */}
          <div className="flex-1 overflow-y-auto w-full select-text pb-6">
             {filteredTemplates.length === 0 ? (
               <div className="flex flex-col items-center justify-center h-full text-slate-500 gap-3 pb-12 select-none">
                 <div className="w-16 h-16 rounded-full bg-white/5 flex items-center justify-center text-slate-600 mb-2">
                   <Icons.Photo />
                 </div>
                 <p className="text-[14px]">找不到匹配的模板特征</p>
                 <p className="text-xs text-slate-600 max-w-[260px] text-center">你可以尝试清除搜索过滤器或直接导入新的本地资产图片。</p>
               </div>
             ) : (
               <div className="flex flex-col">
                 <div className="grid grid-cols-[44px_1.5fr_1fr_60px] gap-4 px-5 py-2.5 text-[11px] font-medium text-slate-500 uppercase tracking-widest border-b border-white/5 sticky top-0 bg-panel/90 backdrop-blur-sm z-10">
                   <div className="text-center">#</div>
                   <div>特征标识</div>
                   <div>元数据 & 标签</div>
                   <div className="text-right">时间</div>
                 </div>
                 {filteredTemplates.map(template => {
                   const isSelected = selectedSet.has(template.id);
                   return (
                     <div 
                       key={template.id}
                       onClick={() => setSelectedIds([template.id])}
                       className={`grid grid-cols-[44px_1.5fr_1fr_80px] gap-4 items-center px-5 py-3 border-b border-white/5 transition-all group select-none cursor-pointer ${
                         isSelected 
                           ? "bg-accent/[0.08] relative before:absolute before:left-0 before:top-0 before:bottom-0 before:w-1 before:bg-accent" 
                           : "hover:bg-white/[0.02]"
                       }`}
                     >
                        <div className="flex items-center justify-center pl-1">
                          <input 
                            type="checkbox" 
                            className="w-4 h-4 rounded-sm border-white/20 bg-black/40 text-accent focus:ring-accent focus:ring-offset-0 cursor-pointer"
                            checked={isSelected}
                            onChange={(e) => {
                              setSelectedIds((cur) => cur.includes(template.id) ? cur.filter((x) => x !== template.id) : [...cur, template.id]);
                            }}
                            onClick={(e) => e.stopPropagation()}
                          />
                        </div>

                        <div className="flex flex-col min-w-0 pr-4">
                           <input 
                             className="bg-transparent border border-transparent rounded px-1 -ml-1 text-[14px] font-semibold text-slate-200 outline-none w-max max-w-full truncate focus:bg-black/30 focus:border-white/20 transition-all" 
                             defaultValue={template.name}
                             onClick={(e) => e.stopPropagation()}
                             onBlur={(e) => {
                               const next = e.target.value.trim();
                               if (next && next !== template.name) void renameTemplate(template.id, next);
                             }}
                           />
                           <div className="flex items-center gap-3 mt-1.5 px-1">
                              <span className="text-xs text-slate-500 font-mono tracking-wider"><Icons.Hash /> {template.hash.slice(0,10)}</span>
                              <span className="text-xs text-slate-500 font-mono tracking-wider"><Icons.ImageSize /> {template.width}×{template.height}</span>
                           </div>
                        </div>

                        <div className="flex flex-col items-start gap-1.5 min-w-0">
                           <div className="text-xs text-slate-400 truncate w-full" title={template.sourcePath ?? "无历史源路径"}>
                             {template.sourcePath ? template.sourcePath.split(/[/\\]/).pop() : "未知数据源"}
                           </div>
                           <div className="flex flex-wrap gap-1">
                             {template.tags.length > 0 ? (
                               template.tags.slice(0, 3).map(tag => (
                                 <span key={tag} className="px-1.5 py-0.5 rounded-sm bg-white/5 border border-white/10 text-[10px] text-slate-300 pointer-events-none truncate max-w-[80px]">{tag}</span>
                               ))
                             ) : (
                               <span className="text-[10px] text-slate-600 block pt-0.5">无预设域</span>
                             )}
                             {template.tags.length > 3 && (
                               <span className="px-1.5 py-0.5 rounded-sm bg-white/5 border border-white/10 text-[10px] text-slate-300">+{template.tags.length - 3}</span>
                             )}
                           </div>
                        </div>

                        <div className="text-[11px] text-slate-500 text-right leading-relaxed flex flex-col justify-center">
                           <span className="font-mono">{formatCreatedAt(template.createdAt).split(' ')[0]}</span>
                           <span className="font-mono">{formatCreatedAt(template.createdAt).split(' ')[1]}</span>
                        </div>

                     </div>
                   )
                 })}
               </div>
             )}
          </div>
        </div>

        {/* ===================== Right Side: Inspector & Intake ===================== */}
        <div className="flex flex-col w-full lg:w-[460px] xl:w-[500px] shrink-0 p-5 gap-5 overflow-y-auto bg-black/40">
          
          {/* Quick Paste Intake Panel - Pops up natively inside flow */}
          {pasteDraft ? (
            <div className="flex flex-col bg-[#1A2332] border border-accent/40 shadow-[0_0_20px_rgba(70,128,186,0.15)] rounded-xl overflow-hidden shrink-0 animate-in fade-in zoom-in-95 duration-200">
               <div className="bg-accent/[0.12] px-4 py-2 border-b border-accent/20 flex justify-between items-center">
                  <div className="flex items-center gap-2">
                    <span className="text-[11px] font-bold text-[#7cb7ed] tracking-widest uppercase flex items-center gap-1.5"><Icons.Clipboard /> 快速剪贴合并</span>
                  </div>
                  <button className="text-slate-400 hover:text-white p-1 rounded hover:bg-white/10 transition-colors" onClick={() => updatePasteDraft(null)}>✕</button>
               </div>
               
               <div className="p-4 flex gap-4">
                  <AssetPreviewCanvas
                    src={pasteDraft.previewUrl}
                    alt="Preview"
                    containerClassName="h-[110px] w-[110px] shrink-0"
                    imageClassName="p-3"
                    onError={() => setPasteError("剪贴板图片预览失败，请重新复制后再试")}
                  />
                  <div className="flex-1 flex flex-col gap-2.5 min-w-0 justify-center">
                     <div>
                       <label className="text-[10px] uppercase tracking-widest text-slate-500 block mb-1">模板名称符号</label>
                       <input 
                         className="w-full bg-black/30 border border-white/10 rounded px-3 py-1.5 text-sm text-slate-200 outline-none focus:border-accent/60 focus:bg-black/40 transition-colors" 
                         value={pasteDraft.name} 
                         onChange={(e) => setPasteDraft((c) => c ? { ...c, name: e.target.value } : c)} 
                       />
                     </div>
                     <div>
                       <label className="text-[10px] uppercase tracking-widest text-slate-500 block mb-1">附加业务标签</label>
                       <input 
                         className="w-full bg-black/30 border border-white/10 rounded px-3 py-1.5 text-sm text-slate-200 outline-none focus:border-accent/60 focus:bg-black/40 transition-colors placeholder:text-slate-600" 
                         value={pasteDraft.tags} 
                         placeholder="tagA, tagB..."
                         onChange={(e) => setPasteDraft((c) => c ? { ...c, tags: e.target.value } : c)} 
                       />
                     </div>
                  </div>
               </div>

               {pasteError && (
                 <div className="px-4 py-2 bg-danger/10 border-t border-b border-danger/20 text-xs text-danger flex items-center gap-2">
                   <div className="w-1.5 h-1.5 rounded-full bg-danger"></div> {pasteError}
                 </div>
               )}
               
               <div className="p-4 pt-3 flex items-center justify-between border-t border-white/5 bg-black/10">
                  <div className="text-[11px] text-slate-500 font-mono flex items-center gap-3">
                     <span>{formatBytes(pasteDraft.sizeBytes)}</span>
                     <span>|</span>
                     <span>{pasteDraft.mimeType}</span>
                  </div>
                  <button 
                     className="px-5 py-1.5 rounded bg-accent hover:bg-[rgb(80,140,200)] text-white text-sm font-medium shadow-md shadow-accent/20 transition-all disabled:opacity-50 flex items-center gap-1.5 focus:ring-2 focus:ring-accent focus:ring-offset-2 focus:ring-offset-[#1A2332]"
                     disabled={pasteBusy}
                     onClick={() => void handleConfirmPaste()}
                  >
                     {pasteBusy ? "处理中..." : "确认合并入库"}
                  </button>
               </div>
            </div>
          ) : (
            <div 
               className="flex flex-col items-center justify-center p-6 border-2 border-dashed border-white/10 hover:border-accent/40 bg-white/[0.01] hover:bg-accent/[0.03] transition-colors rounded-xl shrink-0 cursor-pointer group text-center"
               tabIndex={0}
               onPaste={handlePasteAreaPaste}
            >
               <div className="w-10 h-10 rounded-full bg-white/5 flex items-center justify-center text-slate-500 group-hover:scale-110 group-hover:bg-accent/15 group-hover:text-[rgb(110,168,226)] transition-all mb-3 shadow-[0_0_15px_transparent] group-hover:shadow-[0_0_15px_rgba(70,128,186,0.2)]">
                  <Icons.Clipboard />
               </div>
               <h3 className="text-[14px] font-medium text-slate-300 mb-1">剪贴板快捷入库焦点区域</h3>
               <p className="text-[12px] text-slate-500 max-w-[280px]">使用系统截图工具截取目标后，选中此页面并按 <br/><kbd className="font-mono bg-black/30 border border-white/10 px-1 py-0.5 rounded mx-1 text-slate-400">Ctrl+V</kbd> 直接进行导入预查。</p>
            </div>
          )}

          {/* Active Inspector Stage */}
          <div className="flex flex-col border border-white/5 bg-[#0F141D] rounded-xl overflow-hidden shadow-2xl relative shadow-[0_15px_40px_rgba(0,0,0,0.4)]">
             <div className="relative min-h-[420px] flex-1 bg-black/40 p-5">
                <div className="absolute inset-0 bg-[radial-gradient(ellipse_at_center,rgba(70,128,186,0.18)_0%,transparent_72%)] pointer-events-none" />

                {selectedTemplate && (
                  <>
                    <div className="absolute left-5 top-5 z-20 max-w-[calc(100%-168px)] rounded-full border border-white/10 bg-black/55 px-3 py-1.5 text-[12px] font-medium text-slate-100 shadow-lg backdrop-blur-md">
                      <span className="block truncate">{selectedTemplate.name}</span>
                    </div>
                    <div className="absolute right-5 top-5 z-20 flex items-center gap-2">
                      <span className="rounded-full border border-white/10 bg-black/60 px-2.5 py-1 text-[10px] text-white font-mono shadow-sm backdrop-blur-md">
                        {selectedPreview?.width ?? selectedTemplate.width} × {selectedPreview?.height ?? selectedTemplate.height} px
                      </span>
                      <button
                        className="flex h-8 w-8 items-center justify-center rounded-full border border-danger/30 bg-black/50 text-danger transition-colors hover:bg-danger/80 hover:text-white"
                        onClick={() => void removeTemplate(selectedTemplate.id)}
                        title="剔除当前"
                      >
                        <Icons.Trash />
                      </button>
                    </div>
                  </>
                )}

                {selectedPreview ? (
                  <AssetPreviewCanvas
                    src={selectedPreview.url}
                    alt="Template Stage"
                    width={selectedPreview.width}
                    height={selectedPreview.height}
                    containerClassName="h-full min-h-[380px] w-full"
                    imageClassName="p-6 sm:p-8"
                    onError={() => {
                      setSelectedPreview(null);
                      setSelectedPreviewError("模板图片解码失败，请重新导入后再试");
                    }}
                  />
                ) : selectedPreviewBusy ? (
                  <div className="relative z-10 flex min-h-[380px] items-center justify-center text-sm text-slate-500">
                    <div className="flex items-center gap-2 rounded-full border border-white/10 bg-black/45 px-4 py-2 backdrop-blur-md">
                      <div className="h-3 w-3 rounded-full border-2 border-slate-500 border-t-transparent animate-spin"></div>
                      解码流中...
                    </div>
                  </div>
                ) : selectedPreviewError ? (
                  <div className="relative z-10 flex min-h-[380px] items-center justify-center">
                    <div className="rounded-xl border border-danger/20 bg-danger/10 px-4 py-2 text-sm text-danger">{selectedPreviewError}</div>
                  </div>
                ) : (
                  <div className="relative z-10 flex min-h-[380px] flex-col items-center justify-center gap-2 text-sm text-slate-600">
                    <Icons.ImageSize />
                    <span>从左侧选中模板后，这里只显示图片预览。</span>
                  </div>
                )}
             </div>
          </div>
        </div>
        </div>
      </main>
    </section>
  );
}
