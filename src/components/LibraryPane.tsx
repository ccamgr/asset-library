import { X } from "lucide-react";
import { convertFileSrc } from "@tauri-apps/api/core";
import type { Asset, LegacyImportProgress, Status } from "../types";

type Props = {
  status: Status;
  assets: Asset[];
  filteredAssets: Asset[];
  selectedId: string | null;
  query: string;
  busy: boolean;
  message: string | null;
  error: string | null;
  hasMoreAssets: boolean;
  legacyProgress: LegacyImportProgress | null;
  legacyImportNotice: string | null;
  setQuery: (value: string) => void;
  onSelect: (id: string) => void;
  onDismissLegacyNotice: () => void;
  onLoadMore: () => void;
};

export function LibraryPane({
  status, assets, filteredAssets, selectedId, query, busy,
  message, error, hasMoreAssets, legacyProgress, legacyImportNotice, setQuery,
  onSelect, onDismissLegacyNotice, onLoadMore,
}: Props) {
  return <section className="library-pane">
    <header className="library-header"><div><h1>ライブラリ</h1><p className="muted">{status.assetCount} アセット</p></div><input className="search" placeholder="名前・作者を検索" value={query} onChange={(event) => setQuery(event.target.value)} /></header>
    {message && <p className="success">{message}</p>}
    {error && <p className="error">{error}</p>}
    {(legacyProgress || legacyImportNotice) && <div className="legacy-progress-toast" role="status">{legacyProgress ? <><b>旧形式ZIPを取り込み中</b><span>{legacyProgress.completed} / {legacyProgress.total} 件</span><progress value={legacyProgress.completed} max={legacyProgress.total} /></> : <><div className="legacy-progress-toast__head"><b>旧形式ZIPの取り込み完了</b><button className="icon-button" aria-label="通知を閉じる" onClick={onDismissLegacyNotice}><X aria-hidden="true" size={18} /></button></div><span>{legacyImportNotice}</span></>}</div>}
    <div className="asset-grid">{filteredAssets.map((asset) => <button key={asset.id} className={selectedId === asset.id ? "asset-card selected" : "asset-card"} onClick={() => onSelect(asset.id)}><div className="cover">{asset.coverPath ? <img src={convertFileSrc(asset.coverPath)} alt="" /> : "◇"}</div><b>{asset.title}</b><small>{asset.creator ?? "作者未設定"}</small><div className="chip-row">{asset.tags.map((tag) => <span className="chip" key={tag.id}>{tag.name}</span>)}</div></button>)}</div>
    {hasMoreAssets && <div className="load-more"><button className="secondary" disabled={busy} onClick={onLoadMore}>さらに読み込む</button></div>}
    {!filteredAssets.length && <div className="empty-state"><span className="asset-icon">◇</span><h2>{assets.length ? "一致するアセットはありません" : "まだアセットはありません"}</h2><p>ファイルを追加して整理しましょう。</p></div>}
  </section>;
}
