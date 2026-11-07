import { useEffect } from "react";
import { invoke } from "./api";

type Capacity = { assetBytes: number; databaseBackupBytes: number };
type Backup = { path: string; name: string; byteSize: number };
type SettingsPanelOptions = { enabled: boolean; onOpenLegacyImport: () => void };

const bytes = (value: number) => value >= 1024 ** 3 ? `${(value / 1024 ** 3).toFixed(1)} GB` : value >= 1024 ** 2 ? `${(value / 1024 ** 2).toFixed(1)} MB` : `${value} B`;

/** Mounts the persistent settings launcher and its application-level settings view. */
export function useSettingsPanel({ enabled, onOpenLegacyImport }: SettingsPanelOptions) {
  useEffect(() => {
    if (!enabled) return;
    const sidebar = document.querySelector<HTMLElement>(".sidebar");
    if (!sidebar) return;
    const addAssetButton = sidebar.querySelector<HTMLButtonElement>("button.primary.full");
    const originalAddAssetLabel = addAssetButton?.textContent;
    if (addAssetButton) {
      addAssetButton.textContent = "＋ 追加";
      addAssetButton.setAttribute("aria-label", "アセットを追加");
    }
    const launcher = document.createElement("button");
    launcher.type = "button";
    launcher.className = "icon-button settings-launcher";
    launcher.textContent = "⚙";
    launcher.setAttribute("aria-label", "設定を開く");
    launcher.title = "設定";
    sidebar.append(launcher);

    const open = () => {
      const screen = document.createElement("div");
      screen.className = "settings-screen";
      screen.setAttribute("role", "dialog");
      screen.setAttribute("aria-modal", "true");
      screen.setAttribute("aria-label", "設定");
      const panel = document.createElement("section");
      panel.className = "settings-panel";
      const header = document.createElement("header");
      const title = document.createElement("h1");
      title.textContent = "設定";
      const closeButton = document.createElement("button");
      closeButton.type = "button";
      closeButton.className = "icon-button";
      closeButton.textContent = "×";
      closeButton.setAttribute("aria-label", "設定を閉じる");
      closeButton.title = "閉じる";
      header.append(title, closeButton);

      const storage = document.createElement("section");
      storage.className = "settings-section";
      storage.innerHTML = "<h2>ストレージ</h2>";
      const assetCapacity = document.createElement("p");
      const backupCapacity = document.createElement("p");
      storage.append(assetCapacity, backupCapacity);

      const backup = document.createElement("section");
      backup.className = "settings-section";
      backup.innerHTML = "<h2>データベースのバックアップ</h2><p>ライブDBのスナップショットを、設定したバックアップ先へ作成します。</p>";
      const createBackup = document.createElement("button");
      createBackup.type = "button";
      createBackup.className = "secondary";
      createBackup.textContent = "DBをバックアップ";
      const backupSelect = document.createElement("select");
      const restore = document.createElement("button");
      restore.type = "button";
      restore.className = "secondary";
      restore.textContent = "選択したDBを復元";
      const backupMessage = document.createElement("p");
      backupMessage.className = "hint";
      backup.append(createBackup, backupSelect, restore, backupMessage);

      const migration = document.createElement("section");
      migration.className = "settings-section";
      migration.innerHTML = "<h2>データの取り込み</h2><p>旧形式のZIPを複数フォルダからまとめて取り込めます。</p>";
      const legacyImport = document.createElement("button");
      legacyImport.type = "button";
      legacyImport.className = "secondary";
      legacyImport.textContent = "旧形式ZIPを一括取込";
      migration.append(legacyImport);

      panel.append(header, storage, backup, migration);
      screen.append(panel);
      document.body.append(screen);

      const close = () => screen.remove();
      closeButton.addEventListener("click", close);
      screen.addEventListener("pointerdown", (event) => { if (event.target === screen) close(); });
      screen.addEventListener("keydown", (event) => { if (event.key === "Escape") close(); });
      legacyImport.addEventListener("click", () => { close(); onOpenLegacyImport(); });

      const refresh = async () => {
        const [capacity, backups] = await Promise.all([invoke<Capacity>("get_capacity_report"), invoke<Backup[]>("list_database_backups")]);
        assetCapacity.textContent = `アセット: ${bytes(capacity.assetBytes)}`;
        backupCapacity.textContent = `DBバックアップ: ${bytes(capacity.databaseBackupBytes)}`;
        backupSelect.replaceChildren();
        const placeholder = document.createElement("option");
        placeholder.value = "";
        placeholder.textContent = "復元するバックアップを選択";
        backupSelect.append(placeholder);
        for (const item of backups) {
          const option = document.createElement("option");
          option.value = item.path;
          option.textContent = `${item.name} (${bytes(item.byteSize)})`;
          backupSelect.append(option);
        }
        restore.disabled = !backupSelect.value;
      };
      backupSelect.addEventListener("change", () => { restore.disabled = !backupSelect.value; });
      createBackup.addEventListener("click", async () => {
        createBackup.disabled = true;
        try { const result = await invoke<{ path: string }>("create_database_backup"); backupMessage.textContent = `作成しました: ${result.path}`; await refresh(); }
        catch (reason) { backupMessage.textContent = String(reason); }
        finally { createBackup.disabled = false; }
      });
      restore.addEventListener("click", async () => {
        if (!backupSelect.value || !window.confirm("現在のライブDBを選択したバックアップで復元します。続けますか？")) return;
        restore.disabled = true;
        try { await invoke("restore_database_backup", { backupPath: backupSelect.value }); backupMessage.textContent = "復元しました"; await refresh(); }
        catch (reason) { backupMessage.textContent = String(reason); }
      });
      void refresh().catch((reason) => { backupMessage.textContent = String(reason); });
      closeButton.focus();
    };
    launcher.addEventListener("click", open);
    return () => {
      launcher.removeEventListener("click", open);
      launcher.remove();
      if (addAssetButton && originalAddAssetLabel) addAssetButton.textContent = originalAddAssetLabel;
    };
  }, [enabled, onOpenLegacyImport]);
}
