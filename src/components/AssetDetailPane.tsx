import { ExternalLink, FolderOpen, Pencil, Trash2, X } from "lucide-react";
import type { Detail } from "../types";
import { bytes } from "../lib/utils";

type Props = {
  detail: Detail | null;
  busy: boolean;
  onClose(): void;
  onEdit(): void;
  onDeleteAsset(): void;
  onReveal(fileId: string): void;
  onUrl(url: string): void;
  onSelect(id: string): void;
};

export function AssetDetailPane({
  detail,
  busy,
  onClose,
  onEdit,
  onDeleteAsset,
  onReveal,
  onUrl,
  onSelect,
}: Props) {
  if (!detail) {
    return <aside className="detail-pane"><div className="detail-empty">アセットを選択すると詳細を表示します。</div></aside>;
  }

  return (
    <aside className="detail-pane">
      <div className="detail-head">
        <h2>詳細</h2>
        <div className="detail-head-actions">
          <button className="icon-button" aria-label="アセットを編集" disabled={busy} onClick={onEdit}>
            <Pencil aria-hidden="true" size={20} />
          </button>
          <button className="icon-button" aria-label="アセットを削除" disabled={busy} onClick={onDeleteAsset}>
            <Trash2 aria-hidden="true" size={20} />
          </button>
          <button className="icon-button" aria-label="詳細を閉じる" onClick={onClose}>
            <X aria-hidden="true" size={20} />
          </button>
        </div>
      </div>
      <div className="cover-editor">
        {detail.coverDataUrl ? <img src={detail.coverDataUrl} alt="カバー" /> : "カバー画像なし"}
      </div>
      <section className="detail-section">
        <h3>{detail.title}</h3>
        <p>{detail.creator ?? "作者未設定"}</p>
        {detail.productUrl && (
          <button className="secondary compact" onClick={() => onUrl(detail.productUrl!)}>
            <ExternalLink aria-hidden="true" size={18} /> 商品ページを開く
          </button>
        )}
        {detail.source && <p><b>配布元:</b> {detail.source}</p>}
        {detail.version && <p><b>バージョン:</b> {detail.version}</p>}
        {detail.note && <p className="detail-note">{detail.note}</p>}
      </section>
      <section className="detail-section">
        <b>カテゴリー</b>
        <div className="chip-row">{detail.categories.map((category) => <span className="chip" key={category.id}>{category.name}</span>)}</div>
      </section>
      <section className="detail-section">
        <b>タグ</b>
        <div className="chip-row">{detail.tags.map((tag) => <span className="chip" key={tag.id}>{tag.name}</span>)}</div>
      </section>
      <section className="detail-section">
        <b>ファイル</b>
        {detail.files.map((file) => (
          <div className="file-row" key={file.id}>
            <span>{file.originalFilename}<small>{bytes(file.byteSize)}</small></span>
            <button className="secondary compact" aria-label="ファイルの場所を開く" onClick={() => onReveal(file.id)}>
              <FolderOpen aria-hidden="true" size={18} />
            </button>
          </div>
        ))}
      </section>
      <section className="detail-section">
        <b>関連アセット</b>
        {detail.relations.map((relation) => (
          <button className="relation" key={`${relation.assetId}-${relation.relationType}-${relation.direction}`} onClick={() => onSelect(relation.assetId)}>
            {relation.title}
          </button>
        ))}
      </section>
    </aside>
  );
}
