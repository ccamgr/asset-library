import { invoke as tauriInvoke } from "@tauri-apps/api/core";
import { listen as tauriListen } from "@tauri-apps/api/event";

type AnyRecord = Record<string, any>;
const browserPreview =
  typeof window !== "undefined" && !("__TAURI_INTERNALS__" in window);
const eventHandlers = new Map<string, Set<(event: { payload: any }) => void>>();
const emitMock = (event: string, payload: any) =>
  eventHandlers.get(event)?.forEach((handler) => handler({ payload }));
let nextId = 3;
const tags = [
  { id: "tag-clothes", name: "衣装", color: null },
  { id: "tag-kikyo", name: "桔梗対応", color: null },
];
const categories = [
  { id: "cat-avatar", name: "アバター" },
  { id: "cat-outfit", name: "衣装" },
];
const mockAssets = [
  ["サンプル衣装", "Sample Shop", 1],
  ["アーバンパーカー", "mikanbox", 1],
  ["フリルワンピース", "atelier Lune", 1],
  ["ストリートジャケット", "Kuroko Lab", 1],
  ["夏色セーラーセット", "aozora", 1],
  ["オーバーサイズニット", "mofu-mofu", 1],
  ["サイバーゴーグル", "Pixel Forge", 1],
  ["ネオンスニーカー", "Kuroko Lab", 1],
  ["キツネ耳アクセサリー", "komorebi", 1],
  ["ハンドサインギミック", "Gizmo Factory", 0],
  ["表情アニメーションセット", "Smile Works", 0],
  ["ワールド用ポスター集", "Palette Room", 0],
  ["桔梗用ヘアスタイル", "mikanbox", 0],
  ["アバター向けシェーダー設定", "Shader Note", 0],
  ["雨の日レインコート", "atelier Lune", 1],
  ["和風ストリートコーデ", "komorebi", 1],
  ["メカニカルウイング", "Gizmo Factory", 0],
  ["ライブ用ペンライト", "Pixel Forge", 0],
  ["ローポリ展示台", "Palette Room", 0],
  ["冬のマフラーセット", "mofu-mofu", 1],
] as const;

const assets: AnyRecord[] = mockAssets.map(([title, creator, category], index) => {
  const id = `demo-${index + 1}`;
  return {
    id,
    title,
    creator,
    source: "BOOTH",
    productUrl: `https://booth.pm/ja/items/${1000 + index}`,
    version: index % 3 === 0 ? "1.1.0" : "1.0.0",
    note: index === 0 ? "モックデータ" : "ブラウザプレビュー用のサンプルアセット",
    coverDataUrl: null,
    tags: category ? [tags[0], ...(index % 2 === 0 ? [tags[1]] : [])] : [],
    categories: [categories[category]],
    files: [
      {
        id: `file-${index + 1}`,
        originalFilename: `${title.replaceAll(" ", "-")}.zip`,
        byteSize: (index + 1) * 4_194_304,
      },
    ],
    relations: [],
  };
});

const summary = (asset: AnyRecord) => ({
  id: asset.id,
  title: asset.title,
  creator: asset.creator,
  coverPath: asset.coverDataUrl,
  tags: asset.tags,
  categories: asset.categories,
  searchText: [
    asset.title,
    asset.creator,
    asset.source,
    asset.productUrl,
    asset.note,
  ]
    .filter(Boolean)
    .join(" "),
});
const request = (args?: AnyRecord) => args?.request ?? {};

async function mockInvoke<T>(command: string, args?: AnyRecord): Promise<T> {
  const data = request(args);
  switch (command) {
    case "get_library_status":
      return {
        defaultAssetStoragePath: "ブラウザプレビュー",
        defaultDatabaseBackupPath: "ブラウザプレビュー",
        initialized: true,
        assetCount: assets.length,
        assetStoragePath: "ブラウザプレビュー",
        databaseBackupPath: "ブラウザプレビュー",
      } as T;
    case "list_assets": {
      const query = String(args?.query ?? "").trim().toLocaleLowerCase();
      const offset = Number(args?.offset ?? 0);
      const limit = Number(args?.limit ?? 100);
      return assets
        .map(summary)
        .filter((asset) => !query || asset.searchText.toLocaleLowerCase().includes(query))
        .slice(offset, offset + limit) as T;
    }
    case "get_asset_detail":
      return assets.find((asset) => asset.id === args?.assetId) as T;
    case "list_tags":
      return tags as T;
    case "list_categories":
      return categories as T;
    case "get_capacity_report":
      return {
        assetBytes: assets.reduce(
          (sum, asset) =>
            sum +
            asset.files.reduce(
              (fileSum: number, file: AnyRecord) => fileSum + file.byteSize,
              0,
            ),
          0,
        ),
        databaseBackupBytes: 0,
      } as T;
    case "list_database_backups":
      return [] as T;
    case "import_files": {
      const id = `preview-${nextId++}`;
      const asset = {
        id,
        title: data.title,
        creator: data.creator ?? null,
        source: data.source ?? null,
        productUrl: data.productUrl ?? null,
        version: data.version ?? null,
        note: data.note ?? null,
        coverDataUrl: null,
        tags: tags.filter((tag) => (data.tagIds ?? []).includes(tag.id)),
        categories: categories.filter((category) =>
          (data.categoryIds ?? []).includes(category.id),
        ),
        files: (data.sourcePaths ?? []).map((path: string, index: number) => ({
          id: `${id}-${index}`,
          originalFilename: path.split(/[\\/]/).pop(),
          byteSize: 0,
        })),
        relations: [],
      };
      assets.unshift(asset);
      return {
        assetId: id,
        title: asset.title,
        fileCount: asset.files.length,
        totalBytes: 0,
      } as T;
    }
    case "update_asset": {
      const asset = assets.find((item) => item.id === data.id);
      if (asset) Object.assign(asset, data);
      return undefined as T;
    }
    case "set_asset_tags": {
      const asset = assets.find((item) => item.id === data.assetId);
      if (asset) asset.tags = tags.filter((tag) => data.ids.includes(tag.id));
      return undefined as T;
    }
    case "set_asset_categories": {
      const asset = assets.find((item) => item.id === data.assetId);
      if (asset)
        asset.categories = categories.filter((category) =>
          data.ids.includes(category.id),
        );
      return undefined as T;
    }
    case "create_tag": {
      const tag = {
        id: `tag-${nextId++}`,
        name: data.name,
        color: data.color ?? null,
      };
      tags.push(tag);
      return tag as T;
    }
    case "delete_tag": {
      const index = tags.findIndex((tag) => tag.id === args?.tagId);
      if (index >= 0) tags.splice(index, 1);
      assets.forEach((asset) => {
        asset.tags = asset.tags.filter(
          (tag: AnyRecord) => tag.id !== args?.tagId,
        );
      });
      return undefined as T;
    }
    case "rename_tag": {
      const tag = tags.find((item) => item.id === data.id);
      if (tag) tag.name = data.name;
      return undefined as T;
    }
    case "create_category": {
      const category = { id: `cat-${nextId++}`, name: data.name };
      categories.push(category);
      return category as T;
    }
    case "delete_category": {
      const index = categories.findIndex(
        (category) => category.id === args?.categoryId,
      );
      if (index >= 0) categories.splice(index, 1);
      assets.forEach((asset) => {
        asset.categories = asset.categories.filter(
          (category: AnyRecord) => category.id !== args?.categoryId,
        );
      });
      return undefined as T;
    }
    case "add_asset_relation":
      return undefined as T;
    case "delete_asset": {
      const index = assets.findIndex((asset) => asset.id === args?.assetId);
      if (index >= 0) assets.splice(index, 1);
      return undefined as T;
    }
    case "add_asset_files": {
      const asset = assets.find((item) => item.id === data.assetId);
      if (asset)
        asset.files.push(
          ...(data.sourcePaths ?? []).map((path: string) => ({
            id: `file-${nextId++}`,
            originalFilename: path.split(/[\\/]/).pop(),
            byteSize: 0,
          })),
        );
      return undefined as T;
    }
    case "delete_asset_file": {
      assets.forEach((asset) => {
        asset.files = asset.files.filter(
          (file: AnyRecord) => file.id !== args?.fileId,
        );
      });
      return undefined as T;
    }
    case "count_legacy_archives":
      return 3 as T;
    case "start_legacy_import": {
      const directories = data.directoryPaths ?? [];
      const total = Math.max(directories.length * 3, 1);
      for (let completed = 1; completed <= total; completed += 1) {
        window.setTimeout(
          () => emitMock("legacy-import-progress", { completed, total }),
          completed * 250,
        );
      }
      window.setTimeout(
        () =>
          emitMock("legacy-import-complete", {
            result: { imported: [], failures: [] },
            error: null,
          }),
        (total + 1) * 250,
      );
      return undefined as T;
    }
    case "create_database_backup":
      return { path: "ブラウザプレビュー/mock-backup.db" } as T;
    default:
      return undefined as T;
  }
}

export const invoke = <T>(command: string, args?: AnyRecord) =>
  browserPreview ? mockInvoke<T>(command, args) : tauriInvoke<T>(command, args);
export const listen = <T>(
  event: string,
  handler: (event: { payload: T }) => void,
) => {
  if (!browserPreview) return tauriListen<T>(event, handler);
  const handlers = eventHandlers.get(event) ?? new Set();
  handlers.add(handler);
  eventHandlers.set(event, handlers);
  return Promise.resolve(() => handlers.delete(handler));
};
