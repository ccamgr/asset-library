import { useState } from "react";
import { open } from "../lib/dialog";
import { invoke } from "../lib/api";
import type { Category, Tag } from "../types";
import { AssetLabels } from "./AssetLabels";
import { X } from "lucide-react";
import { nameOf, titleOf } from "../lib/utils";

type Props = {
  tags: Tag[];
  categories: Category[];
  onClose: () => void;
  onDone: (id: string, text: string) => Promise<void>;
};

export function AssetDialog({ tags, categories, onClose, onDone }: Props) {
  const [files, setFiles] = useState<string[]>([]);
  const [title, setTitle] = useState("");
  const [creator, setCreator] = useState("");
  const [source, setSource] = useState("");
  const [version, setVersion] = useState("");
  const [productUrl, setProductUrl] = useState("");
  const [note, setNote] = useState("");
  const [tagIds, setTagIds] = useState<string[]>([]);
  const [categoryIds, setCategoryIds] = useState<string[]>([]);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const showOptional =
    localStorage.getItem("asset-library.show-optional-fields") === "true";
  const choose = async () => {
    try {
      const result = await open({
        multiple: true,
        title: "登録するファイルを選択",
      });
      const paths = Array.isArray(result) ? result : result ? [result] : [];
      if (paths.length) {
        setFiles(paths);
        if (!title) setTitle(titleOf(paths[0]));
      }
    } catch (reason) {
      setError(String(reason));
    }
  };
  const submit = async () => {
    if (!title.trim() || !files.length) {
      setError("アセット名とファイルを指定してください");
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
            source: source.trim() || null,
            version: version.trim() || null,
            productUrl: productUrl.trim() || null,
            note: note.trim() || null,
            tagIds,
            categoryIds,
          },
        },
      );
      await onDone(result.assetId, result.title + " を登録しました");
    } catch (reason) {
      setError(String(reason));
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
            商品URL
            <input
              value={productUrl}
              onChange={(e) => setProductUrl(e.target.value)}
            />
          </label>
          {showOptional && (
            <>
              <label>
                配布元
                <input
                  value={source}
                  onChange={(event) => setSource(event.target.value)}
                />
              </label>
              <label>
                バージョン
                <input
                  value={version}
                  onChange={(event) => setVersion(event.target.value)}
                />
              </label>
            </>
          )}
        </div>
        <label>
          メモ
          <textarea value={note} onChange={(e) => setNote(e.target.value)} />
        </label>
        <section className="file-select">
          <b>登録ファイル</b>
          <button className="secondary compact" disabled={busy} onClick={() => void choose()}>
            ファイルを選択
          </button>
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
          tags={tags}
          categories={categories}
          tagIds={tagIds}
          categoryIds={categoryIds}
          setTagIds={setTagIds}
          setCategoryIds={setCategoryIds}
        />
        {error && <p className="error">{error}</p>}
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
