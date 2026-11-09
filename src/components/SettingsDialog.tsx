import { useEffect, useState } from "react";
import { X } from "lucide-react";
import { invoke } from "../lib/api";
import { confirmDialog } from "../lib/confirm-dialog";
import { bytes } from "../lib/utils";
import type { BackupEntry, Capacity } from "../types";

type Props = {
  onClose(): void;
  onOpenLegacyImport(): void;
  onError(reason: string): void;
  onMessage(message: string): void;
};

type ThumbnailCacheRebuildResult = { generated: number; skipped: number };

export function SettingsDialog({ onClose, onOpenLegacyImport, onError, onMessage }: Props) {
  const [capacity, setCapacity] = useState<Capacity | null>(null);
  const [backups, setBackups] = useState<BackupEntry[]>([]);
  const [selected, setSelected] = useState("");
  const [busy, setBusy] = useState(false);
  const [showOptional, setShowOptional] = useState(
    () => localStorage.getItem("asset-library.show-optional-fields") === "true",
  );

  const refresh = async () => {
    const [nextCapacity, nextBackups] = await Promise.all([
      invoke<Capacity>("get_capacity_report"),
      invoke<BackupEntry[]>("list_database_backups"),
    ]);
    setCapacity(nextCapacity);
    setBackups(nextBackups);
  };

  useEffect(() => {
    void refresh().catch((error) => onError(String(error)));
  }, []);

  const createBackup = async () => {
    setBusy(true);
    try {
      const result = await invoke<{ path: string }>("create_database_backup");
      onMessage(`作成しました: ${result.path}`);
      await refresh();
    } catch (error) {
      onError(String(error));
    } finally {
      setBusy(false);
    }
  };

  const restore = async () => {
    const confirmed = selected && await confirmDialog(
      "DBを復元",
      "現在のライブDBを選択したバックアップで復元します。続けますか？",
      "復元",
    );
    if (!confirmed) return;

    setBusy(true);
    try {
      await invoke("restore_database_backup", { backupPath: selected });
      onMessage("復元しました");
      await refresh();
    } catch (error) {
      onError(String(error));
    } finally {
      setBusy(false);
    }
  };

  const rebuildThumbnailCache = async () => {
    setBusy(true);
    try {
      const result = await invoke<ThumbnailCacheRebuildResult>("rebuild_thumbnail_cache");
      onMessage(
        result.skipped
          ? `${result.generated}件のサムネイルを再生成しました（${result.skipped}件をスキップ）`
          : `${result.generated}件のサムネイルを再生成しました`,
      );
      await refresh();
    } catch (error) {
      onError(String(error));
    } finally {
      setBusy(false);
    }
  };

  const updateOptionalFields = (checked: boolean) => {
    setShowOptional(checked);
    localStorage.setItem("asset-library.show-optional-fields", String(checked));
  };

  return (
    <div className="settings-screen" role="dialog" aria-modal="true" aria-label="設定">
      <section className="settings-panel">
        <header>
          <h1>設定</h1>
          <button className="icon-button" aria-label="設定を閉じる" onClick={onClose}>
            <X aria-hidden size={20} />
          </button>
        </header>
        <section className="settings-section">
          <h2>ストレージ</h2>
          <p>アセット: {capacity ? bytes(capacity.assetBytes) : "読み込み中…"}</p>
          <p>DBバックアップ: {capacity ? bytes(capacity.databaseBackupBytes) : "読み込み中…"}</p>
          <button className="secondary" disabled={busy} onClick={() => void rebuildThumbnailCache()}>
            サムネイルキャッシュを再生成
          </button>
        </section>
        <section className="settings-section">
          <h2>データベースのバックアップ</h2>
          <button className="secondary" disabled={busy} onClick={() => void createBackup()}>
            DBをバックアップ
          </button>
          <select disabled={busy} value={selected} onChange={(event) => setSelected(event.target.value)}>
            <option value="">復元するバックアップを選択</option>
            {backups.map((backup) => (
              <option key={backup.path} value={backup.path}>
                {backup.name} ({bytes(backup.byteSize)})
              </option>
            ))}
          </select>
          <button className="secondary" disabled={busy || !selected} onClick={() => void restore()}>
            選択したDBを復元
          </button>
        </section>
        <section className="settings-section">
          <h2>データの取り込み</h2>
          <button className="secondary" disabled={busy} onClick={() => { onClose(); onOpenLegacyImport(); }}>
            旧形式ZIPを一括取込
          </button>
        </section>
        <section className="settings-section">
          <h2>表示項目</h2>
          <label className="check">
            <input type="checkbox" checked={showOptional} onChange={(event) => updateOptionalFields(event.target.checked)} />
            配布元・バージョンを表示
          </label>
        </section>
      </section>
    </div>
  );
}
