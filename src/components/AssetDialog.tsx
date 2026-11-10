import { useCallback, useEffect, useRef, useState } from "react";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { open } from "../lib/dialog";
import { invoke } from "../lib/api";
import type { Category, Tag } from "../types";
import { AssetLabels } from "./AssetLabels";
import { Download, X } from "lucide-react";
import { nameOf, titleOf } from "../lib/utils";

type Props = {
  tags: Tag[];
  categories: Category[];
  onClose: () => void;
  onDone: (id: string, text: string) => Promise<void>;
  onError: (reason: string) => void;
};

export function AssetDialog({ tags, categories, onClose, onDone, onError }: Props) {
  const [availableTags, setAvailableTags] = useState(tags);
  const [availableCategories, setAvailableCategories] = useState(categories);
  const [files, setFiles] = useState<string[]>([]);
  const [title, setTitle] = useState("");
  const [creator, setCreator] = useState("");
  const [productUrl, setProductUrl] = useState("");
  const [note, setNote] = useState("");
  const [boothCoverUrl, setBoothCoverUrl] = useState<string | null>(null);
  const [tagIds, setTagIds] = useState<string[]>([]);
  const [categoryIds, setCategoryIds] = useState<string[]>([]);
  const [busy, setBusy] = useState(false);
  const [isDropTargetActive, setIsDropTargetActive] = useState(false);
  const dropTargetRef = useRef<HTMLDivElement>(null);
  const titleRef = useRef(title);
  const isBoothUrl = /^(https?:\/\/)?(?:[\w-]+\.)?booth\.pm\/(?:[a-z]{2}(?:-[a-z]{2})?\/)?items\/\d+(?:\.json)?(?:[/?#].*)?$/i.test(productUrl.trim());
  const fetchBoothMetadata = async () => {
    setBusy(true);
    try {
      const metadata = await invoke<{ title: string; creator: string | null; note: string | null; coverUrl: string | null; category: string | null; tags: string[] }>("fetch_booth_metadata", { request: { productUrl } });
      setTitle(metadata.title);
      setCreator(metadata.creator ?? "");
      setNote(metadata.note ?? "");
      setBoothCoverUrl(metadata.coverUrl);
      if (metadata.category) {
        let category = availableCategories.find((item) => item.name.localeCompare(metadata.category!, undefined, { sensitivity: "accent" }) === 0);
        if (!category) {
          category = await invoke<Category>("create_category", { request: { name: metadata.category, color: null } });
          setAvailableCategories((current) => [...current, category!]);
        }
        if (category) setCategoryIds([category.id]);
      }
      const matchedTags = await Promise.all(metadata.tags.map(async (name) => {
        const existing = availableTags.find((tag) => tag.name.localeCompare(name, undefined, { sensitivity: "accent" }) === 0);
        if (existing) return existing.id;
        const tag = await invoke<Tag>("create_tag", { request: { name, color: null } });
        setAvailableTags((current) => [...current, tag]);
        return tag.id;
      }));
      if (matchedTags.length) setTagIds((current) => [...new Set([...current, ...matchedTags])]);
    } catch (reason) { onError(String(reason)); } finally { setBusy(false); }
  };
  useEffect(() => {
    titleRef.current = title;
  }, [title]);
  const addFiles = useCallback((paths: string[]) => {
    const nextPaths = paths.filter(Boolean);
    if (!nextPaths.length) return;
    setFiles((current) => [...new Set([...current, ...nextPaths])]);
    if (!titleRef.current) setTitle(titleOf(nextPaths[0]));
  }, []);
  const isOverDropTarget = useCallback((position: { x: number; y: number }) => {
    const target = dropTargetRef.current;
    if (!target) return false;
    const bounds = target.getBoundingClientRect();
    const scale = window.devicePixelRatio || 1;
    const x = position.x / scale;
    const y = position.y / scale;
    return x >= bounds.left && x <= bounds.right && y >= bounds.top && y <= bounds.bottom;
  }, []);
  useEffect(() => {
    if (!("__TAURI_INTERNALS__" in window)) return;
    let disposed = false;
    let unlisten: (() => void) | undefined;
    void getCurrentWindow()
      .onDragDropEvent(({ payload }) => {
        if (payload.type === "leave") {
          setIsDropTargetActive(false);
          return;
        }
        const isOverTarget = isOverDropTarget(payload.position);
        if (payload.type === "enter" || payload.type === "over") {
          setIsDropTargetActive(isOverTarget);
          return;
        }
        setIsDropTargetActive(false);
        if (!busy && isOverTarget) addFiles(payload.paths);
      })
      .then((cleanup) => {
        if (disposed) cleanup();
        else unlisten = cleanup;
      })
      .catch(() => undefined);
    return () => {
      disposed = true;
      unlisten?.();
    };
  }, [addFiles, busy, isOverDropTarget]);
  const choose = async () => {
    try {
      const result = await open({
        multiple: true,
        title: "登録するファイルを選択",
      });
      const paths = Array.isArray(result) ? result : result ? [result] : [];
      addFiles(paths);
    } catch (reason) {
      onError(String(reason));
    }
  };
  const submit = async () => {
    if (!title.trim() || !files.length) {
      onError("アセット名とファイルを指定してください");
      return;
    }
    setBusy(true);
    try {
      const result = await invoke<{ assetId: string; title: string }>(
        "import_files",
        {
          request: {
            title,
            sourcePaths: files,
            creator: creator.trim() || null,
            productUrl: productUrl.trim() || null,
            note: note.trim() || null,
            tagIds,
            categoryIds,
          },
        },
      );
      let coverError: unknown = null;
      if (boothCoverUrl) {
        try {
          await invoke("set_asset_cover_from_url", { assetId: result.assetId, imageUrl: boothCoverUrl });
        } catch (reason) {
          coverError = reason;
        }
      }
      await onDone(result.assetId, result.title + " を登録しました");
      if (coverError) onError(`カバー画像を取得できませんでした: ${String(coverError)}`);
    } catch (reason) {
      onError(String(reason));
    } finally {
      setBusy(false);
    }
  };
  return (
    <div className="dialog-backdrop">
      <section
        className="dialog"
        role="dialog"
        aria-modal="true"
        aria-labelledby="asset-dialog-title"
      >
        <div className="dialog-head">
          <h2 id="asset-dialog-title">アセット追加</h2>
          <button
            className="icon-button"
            aria-label="アセット追加を閉じる"
            onClick={onClose}
          >
            <X aria-hidden="true" size={20} />
          </button>
        </div>
        <p className="hint">項目とファイルを指定し、「追加」で確定します。</p>
        <div className="dialog-grid">
          <label>
            アセット名
            <input
              autoFocus
              value={title}
              onChange={(e) => setTitle(e.target.value)}
            />
          </label>
          <label>
            作者
            <input
              value={creator}
              onChange={(e) => setCreator(e.target.value)}
            />
          </label>
          <label>
            <span className="field-label">商品URL{isBoothUrl && <button type="button" className="secondary url-fetch-button" disabled={busy} onClick={() => void fetchBoothMetadata()}><Download aria-hidden="true" size={14} /> URLから取得</button>}</span>
            <input value={productUrl} onChange={(e) => setProductUrl(e.target.value)} />
          </label>
        </div>
        <label>
          メモ
          <textarea value={note} onChange={(e) => setNote(e.target.value)} />
        </label>
        <section className="file-select">
          <b>登録ファイル</b>
          <div
            ref={dropTargetRef}
            className={`file-select__drop-target${isDropTargetActive ? " is-dragging" : ""}`}
            onDragEnter={(event) => {
              event.preventDefault();
              if (!busy) setIsDropTargetActive(true);
            }}
            onDragOver={(event) => event.preventDefault()}
            onDragLeave={(event) => {
              if (
                !(event.relatedTarget instanceof Node) ||
                !event.currentTarget.contains(event.relatedTarget)
              ) {
                setIsDropTargetActive(false);
              }
            }}
            onDrop={(event) => {
              event.preventDefault();
              setIsDropTargetActive(false);
              const paths = Array.from(event.dataTransfer.files)
                .map((file) => (file as File & { path?: string }).path)
                .filter((path): path is string => Boolean(path));
              if (!busy) addFiles(paths);
            }}
          >
            <button className="secondary compact" disabled={busy} onClick={() => void choose()}>
              ファイルを選択
            </button>
            <span>または、このエリア内にドラッグ＆ドロップ</span>
          </div>
          <p>
            {files.length
              ? files.length + " ファイルを選択中"
              : "ファイルが選択されていません"}
          </p>
          <ul>
            {files.map((path) => (
              <li key={path}>{nameOf(path)}</li>
            ))}
          </ul>
        </section>
        <AssetLabels
          tags={availableTags}
          categories={availableCategories}
          tagIds={tagIds}
          categoryIds={categoryIds}
          setTagIds={setTagIds}
          setCategoryIds={setCategoryIds}
        />
        <div className="dialog-actions">
          <button className="secondary" disabled={busy} onClick={onClose}>
            キャンセル
          </button>
          <button
            className="primary"
            disabled={busy}
            onClick={() => void submit()}
          >
            {busy ? "追加中…" : "追加"}
          </button>
        </div>
      </section>
    </div>
  );
}
