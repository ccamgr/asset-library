import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { open } from "./lib/dialog";
import { invoke, listen } from "./lib/api";
import { useContextMenu } from "./lib/context-menu";
import { useCategorySidebar } from "./lib/category-sidebar";
import { useSettingsPanel } from "./lib/settings-panel";
import { openTextDialog } from "./lib/text-dialog";
import { confirmDialog } from "./lib/confirm-dialog";
import { AssetLabels } from "./components/AssetLabels";
import { ImportLabels } from "./components/ImportLabels";
import { AssetDialog } from "./components/AssetDialog";
import { LegacyImportDialog } from "./components/LegacyImportDialog";
import { DetailEditor } from "./components/DetailEditor";
import { Plus, Trash2 } from "lucide-react";
import type {
  Asset,
  BackupEntry,
  Capacity,
  Category,
  Detail,
  LegacyImportProgress,
  LegacyImportResult,
  Status,
  Tag,
} from "./types";

const nameOf = (path: string) => path.split(/[\\/]/).pop() ?? path;
const titleOf = (path: string) => nameOf(path).replace(/\.[^.]+$/, "");
const bytes = (n: number) =>
  n >= 1024 ** 3
    ? `${(n / 1024 ** 3).toFixed(1)} GB`
    : n >= 1024 ** 2
      ? `${(n / 1024 ** 2).toFixed(1)} MB`
      : `${n} B`;
const nullable = (value: string) => value.trim() || null;

function App() {
  const [status, setStatus] = useState<Status | null>(null);
  const [storagePath, setStoragePath] = useState("");
  const [backupPath, setBackupPath] = useState("");
  const [assets, setAssets] = useState<Asset[]>([]);
  const [tags, setTags] = useState<Tag[]>([]);
  const [categories, setCategories] = useState<Category[]>([]);
  const [capacity, setCapacity] = useState<Capacity | null>(null);
  const [backups, setBackups] = useState<BackupEntry[]>([]);
  const [selectedBackup, setSelectedBackup] = useState("");
  const [selected, setSelected] = useState<Detail | null>(null);
  const [form, setForm] = useState<Record<string, string>>({});
  const [query, setQuery] = useState("");
  const [tagFilter, setTagFilter] = useState<string | null>(null);
  const [categoryFilter, setCategoryFilter] = useState<string | null>(null);
  const [pendingFiles, setPendingFiles] = useState<string[]>([]);
  const [importTitle, setImportTitle] = useState("");
  const [importTagIds, setImportTagIds] = useState<string[]>([]);
  const [importCategoryIds, setImportCategoryIds] = useState<string[]>([]);
  const [legacyDirectory, setLegacyDirectory] = useState<string | null>(null);
  const [legacyArchiveCount, setLegacyArchiveCount] = useState(0);
  const [legacyProgress, setLegacyProgress] =
    useState<LegacyImportProgress | null>(null);
  const [assetDialogOpen, setAssetDialogOpen] = useState(false);
  const [legacyImportOpen, setLegacyImportOpen] = useState(false);
  const [relationTarget, setRelationTarget] = useState("");
  const [relationType, setRelationType] = useState("related_to");
  const [busy, setBusy] = useState(false);
  const [message, setMessage] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);

  const refresh = async () => {
    const next = await invoke<Status>("get_library_status");
    setStatus(next);
    setStoragePath(next.assetStoragePath ?? next.defaultAssetStoragePath);
    setBackupPath(next.databaseBackupPath ?? next.defaultDatabaseBackupPath);
    if (!next.initialized) return;
    const [nextAssets, nextTags, nextCategories, nextCapacity, nextBackups] =
      await Promise.all([
        invoke<Asset[]>("list_assets", { query: "" }),
        invoke<Tag[]>("list_tags"),
        invoke<Category[]>("list_categories"),
        invoke<Capacity>("get_capacity_report"),
        invoke<BackupEntry[]>("list_database_backups"),
      ]);
    setAssets(nextAssets);
    setTags(nextTags);
    setCategories(nextCategories);
    setCapacity(nextCapacity);
    setBackups(nextBackups);
  };
  const select = async (id: string | null) => {
    if (!id) {
      setSelected(null);
      return;
    }
    const next = await invoke<Detail>("get_asset_detail", { assetId: id });
    setSelected(next);
    setForm(
      Object.fromEntries(
        ["title", "creator", "source", "productUrl", "version", "note"].map(
          (key) => [key, String(next[key as keyof Detail] ?? "")],
        ),
      ),
    );
  };
  useEffect(() => {
    void refresh().catch((reason: unknown) => setError(String(reason)));
  }, []);
  useEffect(() => {
    let unlisten: (() => void) | undefined;
    void listen<LegacyImportProgress>("legacy-import-progress", (event) =>
      setLegacyProgress(event.payload),
    ).then((stop) => {
      unlisten = stop;
    });
    return () => unlisten?.();
  }, []);
  useEffect(() => {
    const showUnhandledError = (event: PromiseRejectionEvent) => {
      event.preventDefault();
      setError(String(event.reason));
    };
    window.addEventListener("unhandledrejection", showUnhandledError);
    return () =>
      window.removeEventListener("unhandledrejection", showUnhandledError);
  }, []);

  const chooseDirectory = async (
    title: string,
    set: (path: string) => void,
  ) => {
    const result = await open({ directory: true, multiple: false, title });
    if (typeof result === "string") set(result);
  };
  const initialize = async () => {
    setBusy(true);
    try {
      await invoke("initialize_library", {
        request: {
          assetStoragePath: storagePath,
          databaseBackupPath: backupPath,
        },
      });
      await refresh();
    } catch (reason) {
      setError(String(reason));
    } finally {
      setBusy(false);
    }
  };
  const chooseFiles = async () => {
    const result = await open({
      multiple: true,
      title: "ファイルを選択",
    });
    const paths = Array.isArray(result) ? result : result ? [result] : [];
    if (paths.length) {
      setPendingFiles(paths);
      setImportTitle(titleOf(paths[0]));
    }
  };
  const importFiles = async () => {
    setBusy(true);
    try {
      const result = await invoke<{ assetId: string; title: string }>(
        "import_files",
        {
          request: {
            title: importTitle,
            sourcePaths: pendingFiles,
            tagIds: importTagIds,
            categoryIds: importCategoryIds,
          },
        },
      );
      setPendingFiles([]);
      setImportTagIds([]);
      setImportCategoryIds([]);
      setMessage(`${result.title} を登録しました`);
      await refresh();
      await select(result.assetId);
    } catch (reason) {
      setError(String(reason));
    } finally {
      setBusy(false);
    }
  };
  const chooseLegacyDirectory = async () => {
    const result = await open({
      directory: true,
      multiple: false,
      title: "旧形式ZIPが入ったフォルダを選択",
    });
    if (typeof result !== "string") return;
    try {
      const count = await invoke<number>("count_legacy_archives", {
        directoryPath: result,
      });
      if (!count) {
        setError("指定フォルダ直下にZIPがありません");
        return;
      }
      if (
        !(await confirmDialog(
          "旧形式ZIPを取り込む",
          `${count}個のZIPを取り込みますか？`,
          "取り込み開始",
        ))
      )
        return;
      setLegacyArchiveCount(count);
      setLegacyDirectory(result);
    } catch (reason) {
      setError(String(reason));
    }
  };
  const importLegacyDirectory = async () => {
    if (!legacyDirectory) return;
    setBusy(true);
    setLegacyProgress({ completed: 0, total: legacyArchiveCount });
    try {
      const result = await invoke<LegacyImportResult>(
        "import_legacy_directory",
        {
          request: {
            directoryPath: legacyDirectory,
            tagIds: importTagIds,
            categoryIds: importCategoryIds,
          },
        },
      );
      setLegacyDirectory(null);
      setImportTagIds([]);
      setImportCategoryIds([]);
      const failures = result.failures.length
        ? `（失敗: ${result.failures.map((item) => `${item.sourceFilename}: ${item.reason}`).join(" / ")}）`
        : "";
      setMessage(
        `${result.imported.length}件を旧形式ZIPから登録しました${failures}`,
      );
      await refresh();
      if (result.imported[0]) await select(result.imported[0].assetId);
    } catch (reason) {
      setError(String(reason));
    } finally {
      setBusy(false);
      setLegacyProgress(null);
    }
  };
  const save = async () => {
    if (!selected) return;
    setBusy(true);
    try {
      await invoke("update_asset", {
        request: {
          id: selected.id,
          title: form.title,
          creator: nullable(form.creator),
          source: nullable(form.source),
          productUrl: nullable(form.productUrl),
          version: nullable(form.version),
          note: nullable(form.note),
        },
      });
      setMessage("保存しました");
      await refresh();
      await select(selected.id);
    } catch (reason) {
      setError(String(reason));
    } finally {
      setBusy(false);
    }
  };
  const assign = async (kind: "tags" | "categories", ids: string[]) => {
    if (!selected) return;
    await invoke(kind === "tags" ? "set_asset_tags" : "set_asset_categories", {
      request: { assetId: selected.id, ids },
    });
    await refresh();
    await select(selected.id);
  };
  const addEntity = (kind: "tag" | "category") =>
    openTextDialog({
      title: `${kind === "tag" ? "タグ" : "カテゴリー"}を追加`,
      label: "名前",
      initialValue: "",
      confirmLabel: "追加",
      onSubmit: async (name) => {
        await invoke(kind === "tag" ? "create_tag" : "create_category", {
          request: { name, color: null },
        });
        await refresh();
      },
    });
  const setCover = async () => {
    if (!selected) return;
    const result = await open({
      multiple: false,
      filters: [
        { name: "画像", extensions: ["png", "jpg", "jpeg", "webp", "gif"] },
      ],
    });
    if (typeof result !== "string") return;
    await invoke("set_asset_cover", {
      assetId: selected.id,
      sourcePath: result,
    });
    await refresh();
    await select(selected.id);
  };
  const addRelation = async () => {
    if (!selected || !relationTarget) return;
    await invoke("add_asset_relation", {
      request: {
        sourceAssetId: selected.id,
        targetAssetId: relationTarget,
        relationType,
        note: null,
      },
    });
    setRelationTarget("");
    await select(selected.id);
  };
  const backup = async () => {
    setBusy(true);
    try {
      const result = await invoke<{ path: string }>("create_database_backup");
      setMessage(`DBバックアップを作成しました: ${result.path}`);
      await refresh();
    } catch (reason) {
      setError(String(reason));
    } finally {
      setBusy(false);
    }
  };
  const restoreBackup = async () => {
    if (
      !selectedBackup ||
      !(await confirmDialog(
        "DBを復元",
        "現在のDBを選択したバックアップで復元します。続けますか？",
        "復元",
      ))
    )
      return;
    setBusy(true);
    try {
      await invoke("restore_database_backup", { backupPath: selectedBackup });
      setMessage("DBバックアップを復元しました");
      await refresh();
      setSelected(null);
    } catch (reason) {
      setError(String(reason));
    } finally {
      setBusy(false);
    }
  };
  const filtered = useMemo(
    () =>
      assets.filter(
        (asset) =>
          `${asset.searchText} ${asset.tags.map((tag) => tag.name).join(" ")}`
            .toLowerCase()
            .includes(query.toLowerCase()) &&
          (!tagFilter || asset.tags.some((tag) => tag.id === tagFilter)) &&
          (!categoryFilter ||
            asset.categories?.some(
              (category) => category.id === categoryFilter,
            )),
      ),
    [assets, categoryFilter, query, tagFilter],
  );
  const contextMenuItems = useCallback(
    (event: MouseEvent) => {
      const target = event.target instanceof HTMLElement ? event.target : null;
      const tagRow = target?.closest(".tag-filter-row");
      if (tagRow) {
        const name = tagRow.querySelector(".filter")?.textContent;
        const tag = tags.find((item) => item.name === name);
        if (tag)
          return [
            {
              label: "名前を変更",
              run: () =>
                openTextDialog({
                  title: "タグ名を変更",
                  label: "タグ名",
                  initialValue: tag.name,
                  confirmLabel: "変更",
                  onSubmit: async (name) => {
                    await invoke("rename_tag", {
                      request: { id: tag.id, name },
                    });
                    await refresh();
                  },
                }),
            },
            {
              label: `タグ「${tag.name}」を削除`,
              icon: <Trash2 aria-hidden="true" size={18} />,
              destructive: true,
              run: async () => {
                if (
                  !(await confirmDialog(
                    "タグを削除",
                    `タグ「${tag.name}」を削除しますか？`,
                  ))
                )
                  return;
                try {
                  await invoke("delete_tag", { tagId: tag.id });
                  if (tagFilter === tag.id) setTagFilter(null);
                  await refresh();
                } catch (reason) {
                  setError(String(reason));
                }
              },
            },
          ];
      }
      const categoryRow = target?.closest<HTMLElement>(
        ".category-filter-row[data-category-id]",
      );
      if (categoryRow?.dataset.categoryId) {
        const category = categories.find(
          (item) => item.id === categoryRow.dataset.categoryId,
        );
        if (category)
          return [
            {
              label: `カテゴリー「${category.name}」を削除`,
              icon: <Trash2 aria-hidden="true" size={18} />,
              destructive: true,
              run: async () => {
                if (
                  !(await confirmDialog(
                    "カテゴリーを削除",
                    `カテゴリー「${category.name}」を削除しますか？`,
                  ))
                )
                  return;
                try {
                  await invoke("delete_category", { categoryId: category.id });
                  if (categoryFilter === category.id) setCategoryFilter(null);
                  await refresh();
                } catch (reason) {
                  setError(String(reason));
                }
              },
            },
          ];
      }
      const card = target?.closest(".asset-card");
      if (card) {
        const index = Array.from(
          document.querySelectorAll(".asset-card"),
        ).indexOf(card);
        const asset = filtered[index];
        if (asset)
          return [
            {
              label: `「${asset.title}」を削除`,
              icon: <Trash2 aria-hidden="true" size={18} />,
              destructive: true,
              run: async () => {
                if (
                  !(await confirmDialog(
                    "アセットを削除",
                    `アセット「${asset.title}」と保存ファイルを削除しますか？`,
                  ))
                )
                  return;
                try {
                  await invoke("delete_asset", { assetId: asset.id });
                  if (selected?.id === asset.id) setSelected(null);
                  await refresh();
                } catch (reason) {
                  setError(String(reason));
                }
              },
            },
          ];
      }
      return null;
    },
    [categories, categoryFilter, filtered, selected?.id, tagFilter, tags],
  );
  useContextMenu(contextMenuItems);
  const openLegacyImportFromSettings = useCallback(
    () => setLegacyImportOpen(true),
    [],
  );
  useSettingsPanel({
    enabled: status?.initialized === true,
    onOpenLegacyImport: openLegacyImportFromSettings,
  });
  useCategorySidebar({
    enabled: status?.initialized === true,
    categories,
    selectedId: categoryFilter,
    onSelect: setCategoryFilter,
  });

  if (!status) return <main className="loading">読み込み中…</main>;
  if (!status.initialized)
    return (
      <main className="shell setup">
        <p className="eyebrow">ASSETLIBRARY</p>
        <h1>保存先を決めましょう</h1>
        <p className="intro">
          DBとキャッシュはアプリデータフォルダに保存されます。
        </p>
        <section className="card">
          <label>アセット保存先</label>
          <div className="path-picker">
            <input
              value={storagePath}
              onChange={(event) => setStoragePath(event.target.value)}
            />
            <button
              className="secondary"
              onClick={() =>
                void chooseDirectory("アセット保存先", setStoragePath)
              }
            >
              選択…
            </button>
          </div>
          <p className="hint">実アセットとカバー画像を保存します。</p>
          <label>DBバックアップ先</label>
          <div className="path-picker">
            <input
              value={backupPath}
              onChange={(event) => setBackupPath(event.target.value)}
            />
            <button
              className="secondary"
              onClick={() =>
                void chooseDirectory("DBバックアップ先", setBackupPath)
              }
            >
              選択…
            </button>
          </div>
          {error && <p className="error">{error}</p>}
          <button
            className="primary"
            disabled={busy || !storagePath || !backupPath}
            onClick={() => void initialize()}
          >
            {busy ? "作成中…" : "ライブラリを作成"}
          </button>
        </section>
      </main>
    );

  return (
    <div className="app-root">
      <main className="app-shell">
        <aside className="sidebar">
          <p className="eyebrow">ASSETLIBRARY</p>
          <button
            className="primary full"
            onClick={() => setAssetDialogOpen(true)}
          >
            <Plus aria-hidden="true" size={18} /> 追加
          </button>
          <button
            className="secondary full"
            disabled={busy}
            onClick={() => setLegacyImportOpen(true)}
          >
            旧形式ZIPを一括取込
          </button>
          <div className="side-section">
            <div className="side-title">
              <b>タグ</b>
              <button
                className="icon-button"
                onClick={() => void addEntity("tag")}
              >
                ＋
              </button>
            </div>
            <button
              className={!tagFilter ? "filter active" : "filter"}
              onClick={() => setTagFilter(null)}
            >
              すべて
            </button>
            {tags.map((tag) => (
              <div className="tag-filter-row" key={tag.id}>
                <button
                  className={tagFilter === tag.id ? "filter active" : "filter"}
                  onClick={() => setTagFilter(tag.id)}
                >
                  {tag.name}
                </button>
                <button
                  className="icon-button"
                  aria-label={tag.name + "を削除"}
                  onClick={async () => {
                    if (
                      !(await confirmDialog(
                        "タグを削除",
                        "タグ「" + tag.name + "」を削除しますか？",
                      ))
                    )
                      return;
                    await invoke("delete_tag", { tagId: tag.id });
                    if (tagFilter === tag.id) setTagFilter(null);
                    await refresh();
                  }}
                >
                  ×
                </button>
              </div>
            ))}
          </div>
          {capacity && (
            <div className="capacity">
              <b>ストレージ</b>
              <span>アセット {bytes(capacity.assetBytes)}</span>
              <span>DBバックアップ {bytes(capacity.databaseBackupBytes)}</span>
              <button
                className="secondary compact"
                disabled={busy}
                onClick={() => void backup()}
              >
                DBをバックアップ
              </button>
              {backups.length > 0 && (
                <>
                  <select
                    value={selectedBackup}
                    onChange={(event) => setSelectedBackup(event.target.value)}
                  >
                    <option value="">復元するバックアップを選択</option>
                    {backups.map((entry) => (
                      <option key={entry.path} value={entry.path}>
                        {entry.name} ({bytes(entry.byteSize)})
                      </option>
                    ))}
                  </select>
                  <button
                    className="secondary compact"
                    disabled={busy || !selectedBackup}
                    onClick={() => void restoreBackup()}
                  >
                    選択したDBを復元
                  </button>
                </>
              )}
            </div>
          )}
        </aside>
        <section className="library-pane">
          <header className="library-header">
            <div>
              <h1>ライブラリ</h1>
              <p className="muted">{status.assetCount} アセット</p>
            </div>
            <input
              className="search"
              placeholder="名前・作者を検索"
              value={query}
              onChange={(event) => setQuery(event.target.value)}
            />
          </header>
          {(pendingFiles.length > 0 || legacyDirectory) && (
            <section className="import-bar">
              {legacyDirectory ? (
                <>
                  <b>旧形式ZIP: {nameOf(legacyDirectory)}</b>
                </>
              ) : (
                <>
                  <input
                    value={importTitle}
                    onChange={(event) => setImportTitle(event.target.value)}
                  />
                  <span>{pendingFiles.length} ファイル</span>
                </>
              )}
              {legacyProgress && (
                <div className="import-progress">
                  <span>
                    {legacyProgress.completed} / {legacyProgress.total} 件
                  </span>
                  <progress
                    value={legacyProgress.completed}
                    max={legacyProgress.total}
                  />
                </div>
              )}
              <ImportLabels
                tags={tags}
                categories={categories}
                tagIds={importTagIds}
                categoryIds={importCategoryIds}
                setTagIds={setImportTagIds}
                setCategoryIds={setImportCategoryIds}
              />
              <button
                className="secondary"
                onClick={() => {
                  setPendingFiles([]);
                  setLegacyDirectory(null);
                  setImportTagIds([]);
                  setImportCategoryIds([]);
                }}
              >
                キャンセル
              </button>
              <button
                className="primary"
                disabled={busy || (!legacyDirectory && !importTitle)}
                onClick={() =>
                  void (legacyDirectory
                    ? importLegacyDirectory()
                    : importFiles())
                }
              >
                登録
              </button>
            </section>
          )}
          {message && <p className="success">{message}</p>}
          {error && <p className="error">{error}</p>}
          <div className="asset-grid">
            {filtered.map((asset) => (
              <button
                key={asset.id}
                className={
                  selected?.id === asset.id
                    ? "asset-card selected"
                    : "asset-card"
                }
                onClick={() => void select(asset.id)}
              >
                <div className="cover">
                  {asset.coverDataUrl ? (
                    <img src={asset.coverDataUrl} alt="" />
                  ) : (
                    "◇"
                  )}
                </div>
                <b>{asset.title}</b>
                <small>{asset.creator ?? "作者未設定"}</small>
                <div className="chip-row">
                  {asset.tags.map((tag) => (
                    <span className="chip" key={tag.id}>
                      {tag.name}
                    </span>
                  ))}
                </div>
              </button>
            ))}
          </div>
          {!filtered.length && (
            <div className="empty-state">
              <span className="asset-icon">◇</span>
              <h2>
                {assets.length
                  ? "一致するアセットはありません"
                  : "まだアセットはありません"}
              </h2>
              <p>ファイルを追加して整理しましょう。</p>
            </div>
          )}
        </section>
        <aside className="detail-pane">
          {!selected ? (
            <div className="detail-empty">
              アセットを選択すると詳細を編集できます。
            </div>
          ) : (
            <>
              <DetailEditor
                detail={selected}
                form={form}
                setForm={setForm}
                tags={tags}
                categories={categories}
                assets={assets}
                relationTarget={relationTarget}
                relationType={relationType}
                setRelationTarget={setRelationTarget}
                setRelationType={setRelationType}
                busy={busy}
                onClose={() => setSelected(null)}
                onSave={save}
                onCover={setCover}
                onAssign={assign}
                onEntity={addEntity}
                onReveal={(fileId) =>
                  void invoke("reveal_asset_file", { fileId })
                }
                onDeleteFile={async (fileId) => {
                  if (
                    !(await confirmDialog(
                      "ファイルを削除",
                      "このファイルを削除しますか？",
                    ))
                  )
                    return;
                  await invoke("delete_asset_file", { fileId });
                  await refresh();
                  await select(selected.id);
                }}
                onAddFiles={async () => {
                  const result = await open({
                    multiple: true,
                    title: "追加するファイルを選択",
                  });
                  const paths = Array.isArray(result)
                    ? result
                    : result
                      ? [result]
                      : [];
                  if (!paths.length) return;
                  await invoke("add_asset_files", {
                    request: { assetId: selected.id, sourcePaths: paths },
                  });
                  await refresh();
                  await select(selected.id);
                }}
                onUrl={(url) => void invoke("open_external_url", { url })}
                onRelation={addRelation}
                onSelect={(id) => void select(id)}
              />{" "}
              <button
                className="secondary compact"
                onClick={async () => {
                  if (
                    !(await confirmDialog(
                      "アセットを削除",
                      "アセットと保存ファイルを削除しますか？",
                    ))
                  )
                    return;
                  await invoke("delete_asset", { assetId: selected.id });
                  setSelected(null);
                  await refresh();
                }}
              >
                <>
                  <Trash2 aria-hidden="true" size={18} /> アセットを削除
                </>
              </button>
            </>
          )}
        </aside>
      </main>
      {assetDialogOpen && (
        <AssetDialog
          tags={tags}
          categories={categories}
          onClose={() => setAssetDialogOpen(false)}
          onDone={async (id, text) => {
            setAssetDialogOpen(false);
            setMessage(text);
            await refresh();
            await select(id);
          }}
        />
      )}
      {legacyImportOpen && (
        <LegacyImportDialog
          tags={tags}
          categories={categories}
          onClose={() => setLegacyImportOpen(false)}
          onDone={async (text) => {
            setLegacyImportOpen(false);
            setMessage(text);
            await refresh();
          }}
        />
      )}
    </div>
  );
}

export default App;
