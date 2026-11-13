import { appendPath } from "../lib/utils";

type Props = {
  storagePath: string;
  backupPath: string;
  databasePath: string;
  busy: boolean;
  onChooseStorage: () => void;
  onChooseBackup: () => void;
  onChooseDatabase: () => void;
  onInitialize: () => void;
  onCancel?: () => void;
};

export function SetupScreen({
  storagePath,
  backupPath,
  databasePath,
  busy,
  onChooseStorage,
  onChooseBackup,
  onChooseDatabase,
  onInitialize,
  onCancel,
}: Props) {
  const storageExample = appendPath(
    appendPath(
      appendPath(
        appendPath(storagePath, "xx"),
        "xxxxxxxx-xxxx-xxxx-xxxx-xxxxxxxxxxxx",
      ),
      "files",
    ),
    "…",
  );
  const databaseExample = appendPath(databasePath, "library.db");
  const backupExample = appendPath(backupPath, "library-<timestamp>-<uuid>.db");
  return (
    <main className="shell setup">
      <p className="eyebrow">ASSETLIBRARY</p>
      <h1>保存先を決めましょう</h1>
      <p className="intro">キャッシュはアプリデータフォルダに保存されます。</p>
      <section className="card">
        <section className="setup-storage-setting">
          <div className="setup-storage-setting__head">
            <h2>アセット保存先</h2>
            <button className="secondary" onClick={onChooseStorage}>変更…</button>
          </div>
          <p className="hint">実アセットとカバー画像を保存します。</p>
          <pre className="storage-example">{storageExample}</pre>
        </section>
        <section className="setup-storage-setting">
          <div className="setup-storage-setting__head">
            <h2>DB本体の保存先</h2>
            <button className="secondary" onClick={onChooseDatabase}>変更…</button>
          </div>
          <p className="hint">指定したフォルダの <code>library.db</code> を使用します。既存のライブラリを選ぶ場合は、対応するアセット保存先も指定してください。</p>
          <pre className="storage-example">{databaseExample}</pre>
        </section>
        <section className="setup-storage-setting">
          <div className="setup-storage-setting__head">
            <h2>DBバックアップ先</h2>
            <button className="secondary" onClick={onChooseBackup}>変更…</button>
          </div>
          <p className="hint">バックアップDBは指定したフォルダに作成します。</p>
          <pre className="storage-example">{backupExample}</pre>
        </section>
        <button
          className="primary"
          disabled={busy || !storagePath || !backupPath}
          onClick={onInitialize}
        >
          {busy ? "適用中…" : onCancel ? "保存先を適用" : "ライブラリを作成"}
        </button>
        {onCancel && <button className="secondary" disabled={busy} onClick={onCancel}>戻る</button>}
      </section>
    </main>
  );
}
