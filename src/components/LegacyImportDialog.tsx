import { useState } from "react";
import { open } from "../lib/dialog";
import { invoke } from "../lib/api";
import type { Category, Tag } from "../types";
import { confirmDialog } from "../lib/confirm-dialog";
import { AssetLabels } from "./AssetLabels";

type Props = {
  tags: Tag[];
  categories: Category[];
  onClose: () => void;
  onStarted: (total: number) => void;
  onError: (reason: string) => void;
};

export function LegacyImportDialog({
  tags,
  categories,
  onClose,
  onStarted,
  onError,
}: Props) {
  const [folders, setFolders] = useState<string[]>([]);
  const [tagIds, setTagIds] = useState<string[]>([]);
  const [categoryIds, setCategoryIds] = useState<string[]>([]);
  const [busy, setBusy] = useState(false);
  const choose = async () => {
    try {
      const result = await open({
        directory: true,
        multiple: true,
        title: "旧形式ZIPが入ったフォルダを選択",
      });
      const paths = Array.isArray(result) ? result : result ? [result] : [];
      setFolders((current) => [...new Set([...current, ...paths])]);
    } catch (reason) {
      onError(String(reason));
    }
  };
  const submit = async () => {
    if (!folders.length) {
      onError("フォルダを指定してください");
      return;
    }
    setBusy(true);
    try {
      const counts = await Promise.all(
        folders.map((directoryPath) =>
          invoke<number>("count_legacy_archives", { directoryPath }),
        ),
      );
      const total = counts.reduce((a, b) => a + b, 0);
      if (!total) {
        onError("指定フォルダ直下にZIPがありません");
        return;
      }
      if (
        !(await confirmDialog(
          "旧形式ZIPを取り込む",
          `${total}個のZIPを取り込みますか？`,
          "取り込み開始",
        ))
      )
        return;
      await invoke("start_legacy_import", {
        request: { directoryPaths: folders, tagIds, categoryIds },
      });
      onStarted(total);
      onClose();
    } catch (reason) {
      onError(String(reason));
    } finally {
      setBusy(false);
    }
  };
  return (
    <div className="dialog-backdrop">
      <section
        className="dialog"
        role="dialog"
        aria-modal="true"
        aria-labelledby="legacy-title"
      >
        <h2 id="legacy-title">旧形式ZIPを一括取り込み</h2>
        <p className="hint">
          複数フォルダを指定できます。各フォルダ直下のZIPを対象にします。
        </p>
        <section className="file-select">
          <b>対象フォルダ</b>
          <button className="secondary compact" disabled={busy} onClick={() => void choose()}>
            フォルダを追加
          </button>
          <ul>
            {folders.map((path) => (
              <li key={path}>
                {path}
                <button
                  className="icon-button"
                  onClick={() =>
                    setFolders(folders.filter((item) => item !== path))
                  }
                >
                  解除
                </button>
              </li>
            ))}
          </ul>
          {!folders.length && <p>フォルダが選択されていません</p>}
        </section>
        <AssetLabels
          tags={tags}
          categories={categories}
          tagIds={tagIds}
          categoryIds={categoryIds}
          setTagIds={setTagIds}
          setCategoryIds={setCategoryIds}
        />
        <div className="dialog-actions">
          <button className="secondary" disabled={busy} onClick={onClose}>
            キャンセル
          </button>
          <button
            className="primary"
            disabled={busy}
            onClick={() => void submit()}
          >
            {busy ? "取り込み中…" : "取り込み開始"}
          </button>
        </div>
      </section>
    </div>
  );
}
