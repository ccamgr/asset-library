import { useState } from "react";
import { Download, Plus, Trash2, X } from "lucide-react";
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
  onError(reason: string): void;
  onAddTag(): void;
};

const optional = (value: string) => value.trim() || null;

export function AssetEditDialog({ asset, tags, categories, onClose, onDone, onError, onAddTag }: Props) {
  const [availableTags, setAvailableTags] = useState(tags);
  const [availableCategories, setAvailableCategories] = useState(categories);
  const [title, setTitle] = useState(asset.title);
  const [creator, setCreator] = useState(asset.creator ?? "");
  const [productUrl, setProductUrl] = useState(asset.productUrl ?? "");
  const [note, setNote] = useState(asset.note ?? "");
  const [tagIds, setTagIds] = useState(asset.tags.map((tag) => tag.id));
  const [categoryIds, setCategoryIds] = useState(asset.categories.map((category) => category.id));
  const [coverPath, setCoverPath] = useState<string | null>(null);
  const [filesToAdd, setFilesToAdd] = useState<string[]>([]);
  const [fileIdsToDelete, setFileIdsToDelete] = useState<string[]>([]);
  const [busy, setBusy] = useState(false);
  const isBoothUrl = /^(https?:\/\/)?(?:[\w-]+\.)?booth\.pm\/(?:[a-z]{2}(?:-[a-z]{2})?\/)?items\/\d+(?:\.json)?(?:[/?#].*)?$/i.test(productUrl.trim());

  const fetchBoothMetadata = async () => {
    setBusy(true);
    try {
      const metadata = await invoke<{ title: string; creator: string | null; note: string | null; category: string | null; tags: string[] }>("fetch_booth_metadata", { request: { productUrl } });
      setTitle(metadata.title);
      setCreator(metadata.creator ?? "");
      setNote(metadata.note ?? "");
      if (metadata.category) {
        let category = availableCategories.find((item) => item.name.localeCompare(metadata.category!, undefined, { sensitivity: "accent" }) === 0);
        if (!category) {
          category = await invoke<Category>("create_category", { request: { name: metadata.category, color: null } });
          setAvailableCategories((current) => [...current, category!]);
        }
        if (category) setCategoryIds([category.id]);
      }
      const ids = await Promise.all(metadata.tags.map(async (name) => {
        const existing = availableTags.find((tag) => tag.name.localeCompare(name, undefined, { sensitivity: "accent" }) === 0);
        if (existing) return existing.id;
        const tag = await invoke<Tag>("create_tag", { request: { name, color: null } });
        setAvailableTags((current) => [...current, tag]);
        return tag.id;
      }));
      if (ids.length) setTagIds((current) => [...new Set([...current, ...ids])]);
    } catch (reason) { onError(String(reason)); } finally { setBusy(false); }
  };

  const submit = async () => {
    if (!title.trim()) {
      onError("アセット名を入力してください");
      return;
    }
    setBusy(true);
    try {
      await invoke("update_asset", {
        request: {
          id: asset.id,
          title,
          creator: optional(creator),
          productUrl: optional(productUrl),
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
      onError(String(reason));
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
      onError(String(reason));
    }
  };

  const chooseFiles = async () => {
    try {
      const result = await open({ multiple: true, title: "追加するファイルを選択" });
      const paths = Array.isArray(result) ? result : result ? [result] : [];
      setFilesToAdd((current) => [...new Set([...current, ...paths])]);
    } catch (reason) {
      onError(String(reason));
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
          <label><span className="field-label">商品URL{isBoothUrl && <button type="button" className="secondary url-fetch-button" disabled={busy} onClick={() => void fetchBoothMetadata()}><Download aria-hidden="true" size={14} /> URLから取得</button>}</span><input value={productUrl} onChange={(event) => setProductUrl(event.target.value)} /></label>
        </div>
        <label>メモ<textarea value={note} onChange={(event) => setNote(event.target.value)} /></label>
        <AssetLabels tags={availableTags} categories={availableCategories} tagIds={tagIds} categoryIds={categoryIds} setTagIds={setTagIds} setCategoryIds={setCategoryIds} onAddTag={onAddTag} />
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
        <div className="dialog-actions">
          <button className="secondary" disabled={busy} onClick={onClose}>キャンセル</button>
          <button className="primary" disabled={busy} onClick={() => void submit()}>{busy ? "保存中…" : "保存"}</button>
        </div>
      </section>
    </div>
  );
}
