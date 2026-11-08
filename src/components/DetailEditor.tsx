import type { Asset, Category, Detail, Tag } from "../types";
import { ExternalLink, FolderOpen, Plus, Trash2, X } from "lucide-react";

type Props = {
  detail: Detail;
  form: Record<string, string>;
  setForm: (value: Record<string, string>) => void;
  tags: Tag[];
  categories: Category[];
  assets: Asset[];
  relationTarget: string;
  relationType: string;
  setRelationTarget: (value: string) => void;
  setRelationType: (value: string) => void;
  busy: boolean;
  onClose: () => void;
  onCover: () => void;
  onAssign: (kind: "tags" | "categories", ids: string[]) => void;
  onEntity: (kind: "tag" | "category") => void;
  onReveal: (id: string) => void;
  onDeleteFile: (id: string) => void;
  onAddFiles: () => void;
  onUrl: (url: string) => void;
  onRelation: () => void;
  onSelect: (id: string) => void;
};
const bytes = (n: number) =>
  n >= 1024 ** 3
    ? `${(n / 1024 ** 3).toFixed(1)} GB`
    : n >= 1024 ** 2
      ? `${(n / 1024 ** 2).toFixed(1)} MB`
      : `${n} B`;

export function DetailEditor({
  detail,
  form,
  setForm,
  tags,
  categories,
  assets,
  relationTarget,
  relationType,
  setRelationTarget,
  setRelationType,
  busy,
  onClose,
  onCover,
  onAssign,
  onEntity,
  onReveal,
  onDeleteFile,
  onAddFiles,
  onUrl,
  onRelation,
  onSelect,
}: Props) {
  const edit = (key: string, value: string) =>
    setForm({ ...form, [key]: value });
  const showOptional =
    localStorage.getItem("asset-library.show-optional-fields") === "true";
  return (
    <>
      <div className="detail-head">
        <h2>詳細</h2>
        <button className="icon-button" onClick={onClose}>
          <X aria-hidden="true" size={20} />
        </button>
      </div>
      <button className="cover-editor" disabled={busy} onClick={onCover}>
        {detail.coverDataUrl ? (
          <img src={detail.coverDataUrl} alt="カバー" />
        ) : (
          "カバー画像を設定"
        )}
      </button>
      <label>
        アセット名
        <input
          value={form.title ?? ""}
          onChange={(event) => edit("title", event.target.value)}
        />
      </label>
      <label>
        作者
        <input
          value={form.creator ?? ""}
          onChange={(event) => edit("creator", event.target.value)}
        />
      </label>
      <label>
        商品URL
        <div className="url-field">
          <input
            value={form.productUrl ?? ""}
            onChange={(event) => edit("productUrl", event.target.value)}
          />
          <button
            className="icon-button"
            aria-label="商品ページを開く"
            disabled={busy || !form.productUrl}
            onClick={() => onUrl(form.productUrl)}
          >
            <ExternalLink aria-hidden="true" size={18} />
          </button>
        </div>
      </label>
      {showOptional && (
        <>
          <label>
            配布元
            <input
              value={form.source ?? ""}
              onChange={(event) => edit("source", event.target.value)}
            />
          </label>
          <label>
            バージョン
            <input
              value={form.version ?? ""}
              onChange={(event) => edit("version", event.target.value)}
            />
          </label>
        </>
      )}
      <label>
        メモ
        <textarea
          value={form.note ?? ""}
          onChange={(event) => edit("note", event.target.value)}
        />
      </label>
      <section className="detail-section">
        <div className="side-title">
          <b>タグ</b>
          <button className="icon-button" onClick={() => onEntity("tag")}>
            <Plus aria-hidden="true" size={18} />
          </button>
        </div>
        {tags.map((tag) => (
          <label className="check" key={tag.id}>
            <input
              type="checkbox"
              disabled={busy}
              checked={detail.tags.some((value) => value.id === tag.id)}
              onChange={(event) =>
                onAssign(
                  "tags",
                  event.target.checked
                    ? [...detail.tags.map((value) => value.id), tag.id]
                    : detail.tags
                        .filter((value) => value.id !== tag.id)
                        .map((value) => value.id),
                )
              }
            />
            {tag.name}
          </label>
        ))}
      </section>
      <section className="detail-section">
        <div className="side-title">
          <b>カテゴリー</b>
          <button
            className="icon-button"
            aria-label="カテゴリーを追加"
            onClick={() => onEntity("category")}
          >
            <Plus aria-hidden="true" size={18} />
          </button>
        </div>
        {categories.map((category) => (
          <label className="check" key={category.id}>
            <input
              type="radio"
              disabled={busy}
              name="category"
              checked={detail.categories.some(
                (value) => value.id === category.id,
              )}
              onClick={() =>
                onAssign(
                  "categories",
                  detail.categories.some((value) => value.id === category.id)
                    ? []
                    : [category.id],
                )
              }
              readOnly
            />
            {category.name}
          </label>
        ))}
      </section>
      <section className="detail-section">
        <div className="side-title">
          <b>ファイル</b>
          <button
            className="secondary compact"
            disabled={busy}
            onClick={onAddFiles}
          >
            ファイルを追加
          </button>
        </div>
        {detail.files.map((file) => (
          <div className="file-row" key={file.id}>
            <span>
              {file.originalFilename}
              <small>{bytes(file.byteSize)}</small>
            </span>
            <div>
              <button
                className="secondary compact"
                disabled={busy}
                aria-label="ファイルの場所を開く"
                onClick={() => onReveal(file.id)}
              >
                <FolderOpen aria-hidden="true" size={18} />
              </button>
              <button
                className="secondary compact"
                disabled={busy}
                onClick={() => onDeleteFile(file.id)}
              >
                <Trash2 aria-label="ファイルを削除" size={18} />
              </button>
            </div>
          </div>
        ))}
      </section>
      <section className="detail-section">
        <b>関連アセット</b>
        {detail.relations.map((relation) => (
          <button
            className="relation"
            key={`${relation.assetId}-${relation.relationType}-${relation.direction}`}
            onClick={() => onSelect(relation.assetId)}
          >
            {relation.title}
          </button>
        ))}
        <select
          value={relationTarget}
          onChange={(event) => setRelationTarget(event.target.value)}
        >
          <option value="">関連先を選択</option>
          {assets
            .filter((asset) => asset.id !== detail.id)
            .map((asset) => (
              <option value={asset.id} key={asset.id}>
                {asset.title}
              </option>
            ))}
        </select>
        <select
          value={relationType}
          onChange={(event) => setRelationType(event.target.value)}
        >
          <option value="related_to">関連</option>
          <option value="addon_for">追加・拡張</option>
          <option value="requires">必須</option>
          <option value="compatible_with">対応</option>
          <option value="variant_of">派生</option>
        </select>
        <button
          className="secondary compact"
          disabled={busy || !relationTarget}
          onClick={onRelation}
        >
          関連を追加
        </button>
      </section>
    </>
  );
}
