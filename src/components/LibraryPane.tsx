import { convertFileSrc } from "@tauri-apps/api/core";
import type { Asset, Status } from "../types";

type Props = {
  status: Status;
  assets: Asset[];
  selectedId: string | null;
  query: string;
  busy: boolean;
  hasMoreAssets: boolean;
  setQuery(value: string): void;
  onSelect(id: string): void;
  onLoadMore(): void;
};

export function LibraryPane({
  status,
  assets,
  selectedId,
  query,
  busy,
  hasMoreAssets,
  setQuery,
  onSelect,
  onLoadMore,
}: Props) {
  return (
    <section className="library-pane">
      <header className="library-header">
        <p className="muted">{status.assetCount} アセット</p>
        <input
          className="search"
          placeholder="名前・作者を検索"
          value={query}
          onChange={(event) => setQuery(event.target.value)}
        />
      </header>
      <div className="asset-grid">
        {assets.map((asset) => (
          <button
            key={asset.id}
            className={selectedId === asset.id ? "asset-card selected" : "asset-card"}
            onClick={() => onSelect(asset.id)}
          >
            <div className="cover">
              {asset.coverPath ? <img src={convertFileSrc(asset.coverPath)} alt="" /> : "◇"}
            </div>
            <b>{asset.title}</b>
            <small>{asset.creator ?? "作者未設定"}</small>
            <div className="chip-row">
              {asset.tags.map((tag) => <span className="chip" key={tag.id}>{tag.name}</span>)}
            </div>
          </button>
        ))}
      </div>
      {hasMoreAssets && (
        <div className="load-more">
          <button className="secondary" disabled={busy} onClick={onLoadMore}>さらに読み込む</button>
        </div>
      )}
      {!assets.length && (
        <div className="empty-state">
          <span className="asset-icon">◇</span>
          <h2>{status.assetCount ? "一致するアセットはありません" : "まだアセットはありません"}</h2>
          <p>ファイルを追加して整理しましょう。</p>
        </div>
      )}
    </section>
  );
}
