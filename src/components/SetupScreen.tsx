import { useState } from "react";

type Props = {
  storagePath: string;
  backupPath: string;
  databasePath: string;
  busy: boolean;
  setStoragePath: (value: string) => void;
  setBackupPath: (value: string) => void;
  setDatabasePath: (value: string) => void;
  onChooseStorage: () => void;
  onChooseBackup: () => void;
  onChooseDatabase: () => void;
  onInitialize: () => void;
};

export function SetupScreen({
  storagePath,
  backupPath,
  databasePath,
  busy,
  setStoragePath,
  setBackupPath,
  setDatabasePath,
  onChooseStorage,
  onChooseBackup,
  onChooseDatabase,
  onInitialize,
}: Props) {
  const [advancedOpen, setAdvancedOpen] = useState(false);
  return (
    <main className="shell setup">
      <p className="eyebrow">ASSETLIBRARY</p>
      <h1>保存先を決めましょう</h1>
      <p className="intro">キャッシュはアプリデータフォルダに保存されます。</p>
      <section className="card">
        <label>アセット保存先</label>
        <div className="path-picker">
          <input
            value={storagePath}
            onChange={(event) => setStoragePath(event.target.value)}
          />
          <button className="secondary" onClick={onChooseStorage}>選択…</button>
        </div>
        <p className="hint">実アセットとカバー画像を保存します。</p>
        <button className="secondary compact" type="button" onClick={() => setAdvancedOpen((value) => !value)}>
          {advancedOpen ? "高度な設定を閉じる" : "高度な設定"}
        </button>
        {advancedOpen && <section className="setup-advanced">
          <label>DB本体の保存先</label>
          <div className="path-picker">
            <input value={databasePath} onChange={(event) => setDatabasePath(event.target.value)} />
            <button className="secondary" onClick={onChooseDatabase}>選択…</button>
          </div>
          <p className="hint">指定したフォルダに <code>library.db</code> を作成します。変更は初回作成時のみ可能です。</p>
        </section>}
        <label>DBバックアップ先</label>
        <div className="path-picker">
          <input
            value={backupPath}
            onChange={(event) => setBackupPath(event.target.value)}
          />
          <button className="secondary" onClick={onChooseBackup}>選択…</button>
        </div>
        <button
          className="primary"
          disabled={busy || !storagePath || !backupPath}
          onClick={onInitialize}
        >
          {busy ? "作成中…" : "ライブラリを作成"}
        </button>
      </section>
    </main>
  );
}
