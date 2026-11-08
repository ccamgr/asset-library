import { useCallback, useEffect, useRef, useState } from "react";
import { open } from "./lib/dialog";
import { invoke, listen } from "./lib/api";
import { useContextMenu } from "./lib/context-menu";
import { openTextDialog } from "./lib/text-dialog";
import { confirmDialog } from "./lib/confirm-dialog";
import { nullable } from "./lib/utils";
import { AssetDialog } from "./components/AssetDialog";
import { LegacyImportDialog } from "./components/LegacyImportDialog";
import { SetupScreen } from "./components/SetupScreen";
import { LibraryPane } from "./components/LibraryPane";
import { AssetSidebar } from "./components/AssetSidebar";
import { AssetDetailPane } from "./components/AssetDetailPane";
import { SettingsDialog } from "./components/SettingsDialog";
import { Trash2 } from "lucide-react";
import type {
  Asset,
  Category,
  Detail,
  LegacyImportProgress,
  LegacyImportComplete,
  Status,
  Tag,
} from "./types";

const ASSET_PAGE_SIZE = 100;

function App() {
  const [status, setStatus] = useState<Status | null>(null);
  const [storagePath, setStoragePath] = useState("");
  const [backupPath, setBackupPath] = useState("");
  const [assets, setAssets] = useState<Asset[]>([]);
  const [tags, setTags] = useState<Tag[]>([]);
  const [categories, setCategories] = useState<Category[]>([]);
  const [selected, setSelected] = useState<Detail | null>(null);
  const [form, setForm] = useState<Record<string, string>>({});
  const [query, setQuery] = useState("");
  const [tagFilterIds, setTagFilterIds] = useState<string[]>([]);
  const [tagSearch, setTagSearch] = useState("");
  const [categoryFilter, setCategoryFilter] = useState<string | null>(null);
  const [legacyProgress, setLegacyProgress] =
    useState<LegacyImportProgress | null>(null);
  const [legacyImportNotice, setLegacyImportNotice] = useState<string | null>(
    null,
  );
  const [assetDialogOpen, setAssetDialogOpen] = useState(false);
  const [legacyImportOpen, setLegacyImportOpen] = useState(false);
  const [settingsOpen, setSettingsOpen] = useState(false);
  const [relationTarget, setRelationTarget] = useState("");
  const [relationType, setRelationType] = useState("related_to");
  const [busy, setBusy] = useState(false);
  const [message, setMessage] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [hasMoreAssets, setHasMoreAssets] = useState(false);
  const assetRequest = useRef(0);

  const reportFailure = (reason: unknown) => setError(String(reason));

  const loadAssets = useCallback(async (
    search: string,
    tagIds: string[],
    categoryId: string | null,
    offset = 0,
    append = false,
  ) => {
    const request = ++assetRequest.current;
    const nextAssets = await invoke<Asset[]>("list_assets", {
      query: search,
      tagIds,
      categoryId,
      limit: ASSET_PAGE_SIZE,
      offset,
    });
    if (request === assetRequest.current) {
      setAssets((current) => append ? [...current, ...nextAssets] : nextAssets);
      setHasMoreAssets(nextAssets.length === ASSET_PAGE_SIZE);
    }
  }, []);

  const refresh = async () => {
    const next = await invoke<Status>("get_library_status");
    setStatus(next);
    setStoragePath(next.assetStoragePath ?? next.defaultAssetStoragePath);
    setBackupPath(next.databaseBackupPath ?? next.defaultDatabaseBackupPath);
    if (!next.initialized) return;
    const [, nextTags, nextCategories] =
      await Promise.all([
        loadAssets(query, tagFilterIds, categoryFilter),
        invoke<Tag[]>("list_tags"),
        invoke<Category[]>("list_categories"),
      ]);
    setTags(nextTags);
    setCategories(nextCategories);
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
    const timer = window.setTimeout(() => {
      void loadAssets(query, tagFilterIds, categoryFilter).catch(reportFailure);
    }, 180);
    return () => window.clearTimeout(timer);
  }, [categoryFilter, loadAssets, query, tagFilterIds]);
  const loadMoreAssets = async () => {
    setBusy(true);
    try {
      await loadAssets(query, tagFilterIds, categoryFilter, assets.length, true);
    } catch (reason) {
      reportFailure(reason);
    } finally {
      setBusy(false);
    }
  };
  useEffect(() => {
    let unlisten: (() => void) | undefined;
    void listen<LegacyImportComplete>("legacy-import-complete", (event) => {
      setLegacyProgress(null);
      const { result, error: importError } = event.payload;
      if (importError) {
        setError(importError);
        return;
      }
      if (!result) return;
      const failures = result.failures.length
        ? `（失敗: ${result.failures.map((item) => `${item.sourceFilename}: ${item.reason}`).join(" / ")}）`
        : "";
      setLegacyImportNotice(
        `${result.imported.length}件を旧形式ZIPから登録しました${failures}`,
      );
      void refresh()
        .then(async () => {
          if (result.imported[0]) await select(result.imported[0].assetId);
        })
        .catch(reportFailure);
    }).then((stop) => {
      unlisten = stop;
    });
    return () => unlisten?.();
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
    try {
      const result = await open({ directory: true, multiple: false, title });
      if (typeof result === "string") set(result);
    } catch (reason) {
      reportFailure(reason);
    }
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
    setBusy(true);
    try {
      await invoke(kind === "tags" ? "set_asset_tags" : "set_asset_categories", {
        request: { assetId: selected.id, ids },
      });
      await refresh();
      await select(selected.id);
    } catch (reason) {
      reportFailure(reason);
    } finally {
      setBusy(false);
    }
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
    setBusy(true);
    try {
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
    } catch (reason) {
      reportFailure(reason);
    } finally {
      setBusy(false);
    }
  };
  const addRelation = async () => {
    if (!selected || !relationTarget) return;
    setBusy(true);
    try {
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
    } catch (reason) {
      reportFailure(reason);
    } finally {
      setBusy(false);
    }
  };
  const contextMenuItems = useCallback(
    (event: MouseEvent) => {
      const target = event.target instanceof HTMLElement ? event.target : null;
      const tagRow = target?.closest<HTMLElement>(
        ".tag-filter-row[data-tag-id]",
      );
      if (tagRow?.dataset.tagId) {
        const tag = tags.find((item) => item.id === tagRow.dataset.tagId);
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
                setBusy(true);
                try {
                  await invoke("delete_tag", { tagId: tag.id });
                  setTagFilterIds((current) =>
                    current.filter((id) => id !== tag.id),
                  );
                  await refresh();
                } catch (reason) {
                  setError(String(reason));
                } finally {
                  setBusy(false);
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
                setBusy(true);
                try {
                  await invoke("delete_category", { categoryId: category.id });
                  if (categoryFilter === category.id) setCategoryFilter(null);
                  await refresh();
                } catch (reason) {
                  setError(String(reason));
                } finally {
                  setBusy(false);
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
        const asset = assets[index];
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
                setBusy(true);
                try {
                  await invoke("delete_asset", { assetId: asset.id });
                  if (selected?.id === asset.id) setSelected(null);
                  await refresh();
                } catch (reason) {
                  setError(String(reason));
                } finally {
                  setBusy(false);
                }
              },
            },
          ];
      }
      return null;
    },
    [assets, categories, categoryFilter, selected?.id, tags],
  );
  useContextMenu(contextMenuItems);
  const openLegacyImportFromSettings = useCallback(
    () => setLegacyImportOpen(true),
    [],
  );

  if (!status) return <main className="loading">読み込み中…</main>;
  if (!status.initialized)
    return (
      <SetupScreen
        storagePath={storagePath}
        backupPath={backupPath}
        busy={busy}
        error={error}
        setStoragePath={setStoragePath}
        setBackupPath={setBackupPath}
        onChooseStorage={() =>
          void chooseDirectory("アセット保存先", setStoragePath)
        }
        onChooseBackup={() =>
          void chooseDirectory("DBバックアップ先", setBackupPath)
        }
        onInitialize={() => void initialize()}
      />
    );

  return (
    <div className="app-root">
      <main className="app-shell">
        <AssetSidebar
          tags={tags}
          categories={categories}
          categoryFilter={categoryFilter}
          tagFilterIds={tagFilterIds}
          tagSearch={tagSearch}
          busy={busy}
          onOpenAssetDialog={() => setAssetDialogOpen(true)}
          onOpenSettings={() => setSettingsOpen(true)}
          onAddEntity={(kind) => void addEntity(kind)}
          onCategoryFilter={setCategoryFilter}
          onTagFilterIds={setTagFilterIds}
          onTagSearch={setTagSearch}
        />
        <LibraryPane
          status={status}
          assets={assets}
          selectedId={selected?.id ?? null}
          query={query}
          busy={busy}
          message={message}
          error={error}
          hasMoreAssets={hasMoreAssets}
          legacyProgress={legacyProgress}
          legacyImportNotice={legacyImportNotice}
          setQuery={setQuery}
          onSelect={(id) => void select(id).catch(reportFailure)}
          onDismissLegacyNotice={() => setLegacyImportNotice(null)}
          onLoadMore={() => void loadMoreAssets()}
        />
        <AssetDetailPane
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
          onCover={setCover}
          onAssign={assign}
          onEntity={addEntity}
          onReveal={async (fileId) => {
            setBusy(true);
            try {
              await invoke("reveal_asset_file", { fileId });
            } catch (reason) {
              reportFailure(reason);
            } finally {
              setBusy(false);
            }
          }}
          onDeleteFile={async (fileId) => {
            if (!(await confirmDialog("ファイルを削除", "このファイルを削除しますか？"))) return;
            setBusy(true);
            try {
              await invoke("delete_asset_file", { fileId });
              await refresh();
              if (selected) await select(selected.id);
            } catch (reason) {
              reportFailure(reason);
            } finally {
              setBusy(false);
            }
          }}
          onAddFiles={async () => {
            if (!selected) return;
            setBusy(true);
            try {
              const result = await open({ multiple: true, title: "追加するファイルを選択" });
              const paths = Array.isArray(result) ? result : result ? [result] : [];
              if (!paths.length) return;
              await invoke("add_asset_files", { request: { assetId: selected.id, sourcePaths: paths } });
              await refresh();
              await select(selected.id);
            } catch (reason) {
              reportFailure(reason);
            } finally {
              setBusy(false);
            }
          }}
          onUrl={async (url) => {
            setBusy(true);
            try {
              await invoke("open_external_url", { url });
            } catch (reason) {
              reportFailure(reason);
            } finally {
              setBusy(false);
            }
          }}
          onRelation={addRelation}
          onSelect={(id) => void select(id).catch(reportFailure)}
          onSave={save}
          onDeleteAsset={async () => {
            if (!(await confirmDialog("アセットを削除", "アセットと保存ファイルを削除しますか？"))) return;
            if (!selected) return;
            setBusy(true);
            try {
              await invoke("delete_asset", { assetId: selected.id });
              setSelected(null);
              await refresh();
            } catch (reason) {
              reportFailure(reason);
            } finally {
              setBusy(false);
            }
          }}
        />
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
          onStarted={(total) => setLegacyProgress({ completed: 0, total })}
        />
      )}
      {settingsOpen && <SettingsDialog onClose={() => setSettingsOpen(false)} onOpenLegacyImport={openLegacyImportFromSettings} />}
    </div>
  );
}

export default App;
