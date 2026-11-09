import { useState } from "react";
import { Plus, Trash2, X } from "lucide-react";
import { open } from "../lib/dialog";
import { invoke } from "../lib/api";
import type { Category, Detail, Tag } from "../types";
import { AssetLabels } from "./AssetLabels";
import { nameOf } from "../lib/utils";

type Props = {
  asset: Detail;
  tags: Tag[];
  categories: Category[];
  onClose(): void;
  onDone(text: string): Promise<void>;
};

const optional = (value: string) => value.trim() || null;

export function AssetEditDialog({ asset, tags, categories, onClose, onDone }: Props) {
  const [title, setTitle] = useState(asset.title);
  const [creator, setCreator] = useState(asset.creator ?? "");
  const [source, setSource] = useState(asset.source ?? "");
  const [version, setVersion] = useState(asset.version ?? "");
  const [productUrl, setProductUrl] = useState(asset.productUrl ?? "");
  const [note, setNote] = useState(asset.note ?? "");
  const [tagIds, setTagIds] = useState(asset.tags.map((tag) => tag.id));
  const [categoryIds, setCategoryIds] = useState(asset.categories.map((category) => category.id));
  const [coverPath, setCoverPath] = useState<string | null>(null);
  const [filesToAdd, setFilesToAdd] = useState<string[]>([]);
  const [fileIdsToDelete, setFileIdsToDelete] = useState<string[]>([]);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const showOptional = localStorage.getItem("asset-library.show-optional-fields") === "true";

  const submit = async () => {
    if (!title.trim()) {
      setError("アセット名を入力してください");
      return;
    }
    setBusy(true);
    try {
      await invoke("update_asset", {
        request: {
          id: asset.id,
          title,
          creator: optional(creator),
          source: optional(source),
          productUrl: optional(productUrl),
          version: optional(version),
          note: optional(note),
        },
      });
      await invoke("set_asset_tags", { request: { assetId: asset.id, ids: tagIds } });
      await invoke("set_asset_categories", { request: { assetId: asset.id, ids: categoryIds } });
      if (coverPath) {
        await invoke("set_asset_cover", { assetId: asset.id, sourcePath: coverPath });
      }
      for (const fileId of fileIdsToDelete) {
        await invoke("delete_asset_file", { fileId });
      }
      if (filesToAdd.length) {
        await invoke("add_asset_files", { request: { assetId: asset.id, sourcePaths: filesToAdd } });
      }
      await onDone(`${title.trim()} を保存しました`);
    } catch (reason) {
      setError(String(reason));
    } finally {
      setBusy(false);
    }
  };

  const chooseCover = async () => {
    try {
      const result = await open({
        multiple: false,
        filters: [{ name: "画像", extensions: ["png", "jpg", "jpeg", "webp", "gif"] }],
      });
      if (typeof result === "string") setCoverPath(result);
    } catch (reason) {
      setError(String(reason));
    }
  };

  const chooseFiles = async () => {
    try {
      const result = await open({ multiple: true, title: "追加するファイルを選択" });
      const paths = Array.isArray(result) ? result : result ? [result] : [];
      setFilesToAdd((current) => [...new Set([...current, ...paths])]);
    } catch (reason) {
      setError(String(reason));
    }
  };

  return (
    <div className="dialog-backdrop">
      <section className="dialog" role="dialog" aria-modal="true" aria-labelledby="asset-edit-dialog-title">
        <div className="dialog-head">
          <h2 id="asset-edit-dialog-title">アセットを編集</h2>
          <button className="icon-button" aria-label="編集を閉じる" disabled={busy} onClick={onClose}>
            <X aria-hidden="true" size={20} />
          </button>
        </div>
        <div className="dialog-grid">
          <label>アセット名<input autoFocus value={title} onChange={(event) => setTitle(event.target.value)} /></label>
          <label>作者<input value={creator} onChange={(event) => setCreator(event.target.value)} /></label>
          <label>商品URL<input value={productUrl} onChange={(event) => setProductUrl(event.target.value)} /></label>
          {showOptional && <><label>配布元<input value={source} onChange={(event) => setSource(event.target.value)} /></label><label>バージョン<input value={version} onChange={(event) => setVersion(event.target.value)} /></label></>}
        </div>
        <label>メモ<textarea value={note} onChange={(event) => setNote(event.target.value)} /></label>
        <AssetLabels tags={tags} categories={categories} tagIds={tagIds} categoryIds={categoryIds} setTagIds={setTagIds} setCategoryIds={setCategoryIds} />
        <section className="file-select">
          <b>カバー画像</b>
          <button className="secondary compact" disabled={busy} onClick={() => void chooseCover()}>画像を選択</button>
          <p>{coverPath ? nameOf(coverPath) : "変更しない"}</p>
        </section>
        <section className="file-select">
          <div className="side-title"><b>ファイル</b><button className="secondary compact" disabled={busy} onClick={() => void chooseFiles()}><Plus aria-hidden="true" size={18} /> ファイルを追加</button></div>
          <ul>
            {asset.files.map((file) => (
              <li key={file.id}>
                {file.originalFilename}
                <button
                  className={fileIdsToDelete.includes(file.id) ? "secondary compact" : "icon-button"}
                  aria-label={`${file.originalFilename}を削除対象にする`}
                  disabled={busy}
                  onClick={() => setFileIdsToDelete((current) => current.includes(file.id) ? current.filter((id) => id !== file.id) : [...current, file.id])}
                ><Trash2 aria-hidden="true" size={18} /></button>
              </li>
            ))}
            {filesToAdd.map((path) => <li key={path}>{nameOf(path)}</li>)}
          </ul>
        </section>
        {error && <p className="error">{error}</p>}
        <div className="dialog-actions">
          <button className="secondary" disabled={busy} onClick={onClose}>キャンセル</button>
          <button className="primary" disabled={busy} onClick={() => void submit()}>{busy ? "保存中…" : "保存"}</button>
        </div>
      </section>
    </div>
  );
}
