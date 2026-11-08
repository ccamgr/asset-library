type Props = {
  storagePath: string;
  backupPath: string;
  busy: boolean;
  error: string | null;
  setStoragePath: (value: string) => void;
  setBackupPath: (value: string) => void;
  onChooseStorage: () => void;
  onChooseBackup: () => void;
  onInitialize: () => void;
};

export function SetupScreen({
  storagePath,
  backupPath,
  busy,
  error,
  setStoragePath,
  setBackupPath,
  onChooseStorage,
  onChooseBackup,
  onInitialize,
}: Props) {
  return (
    <main className="shell setup">
      <p className="eyebrow">ASSETLIBRARY</p>
      <h1>保存先を決めましょう</h1>
      <p className="intro">DBとキャッシュはアプリデータフォルダに保存されます。</p>
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
        <label>DBバックアップ先</label>
        <div className="path-picker">
          <input
            value={backupPath}
            onChange={(event) => setBackupPath(event.target.value)}
          />
          <button className="secondary" onClick={onChooseBackup}>選択…</button>
        </div>
        {error && <p className="error">{error}</p>}
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
