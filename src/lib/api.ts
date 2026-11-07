import { invoke as tauriInvoke } from "@tauri-apps/api/core";
import { listen as tauriListen } from "@tauri-apps/api/event";

type AnyRecord = Record<string, any>;
const browserPreview = typeof window !== "undefined" && !("__TAURI_INTERNALS__" in window);
let nextId = 3;
const tags = [{ id: "tag-clothes", name: "衣装", color: null }, { id: "tag-kikyo", name: "桔梗対応", color: null }];
const categories = [{ id: "cat-avatar", name: "アバター" }, { id: "cat-outfit", name: "衣装" }];
const assets: AnyRecord[] = [{ id: "demo-1", title: "サンプル衣装", creator: "Sample Shop", source: "BOOTH", productUrl: "https://booth.pm/", shopUrl: "https://booth.pm/", description: "ブラウザ開発用のサンプルです。", version: "1.0.0", note: "モックデータ", coverDataUrl: null, tags: [tags[0], tags[1]], categories: [categories[1]], files: [{ id: "file-1", originalFilename: "sample-outfit.zip", byteSize: 12_582_912 }], relations: [] }];

const summary = (asset: AnyRecord) => ({ id: asset.id, title: asset.title, creator: asset.creator, coverDataUrl: asset.coverDataUrl, tags: asset.tags, searchText: [asset.title, asset.creator, asset.source, asset.productUrl, asset.note].filter(Boolean).join(" ") });
const request = (args?: AnyRecord) => args?.request ?? {};

async function mockInvoke<T>(command: string, args?: AnyRecord): Promise<T> {
  const data = request(args);
  switch (command) {
    case "get_library_status": return { defaultAssetStoragePath: "ブラウザプレビュー", defaultDatabaseBackupPath: "ブラウザプレビュー", initialized: true, assetCount: assets.length, assetStoragePath: "ブラウザプレビュー", databaseBackupPath: "ブラウザプレビュー" } as T;
    case "list_assets": return assets.map(summary) as T;
    case "get_asset_detail": return assets.find((asset) => asset.id === args?.assetId) as T;
    case "list_tags": return tags as T;
    case "list_categories": return categories as T;
    case "get_capacity_report": return { assetBytes: assets.reduce((sum, asset) => sum + asset.files.reduce((fileSum: number, file: AnyRecord) => fileSum + file.byteSize, 0), 0), databaseBackupBytes: 0 } as T;
    case "list_database_backups": return [] as T;
    case "import_files": { const id = `preview-${nextId++}`; const asset = { id, title: data.title, creator: data.creator ?? null, source: data.source ?? null, productUrl: data.productUrl ?? null, shopUrl: data.shopUrl ?? null, description: data.description ?? null, version: data.version ?? null, note: data.note ?? null, coverDataUrl: null, tags: tags.filter((tag) => (data.tagIds ?? []).includes(tag.id)), categories: categories.filter((category) => (data.categoryIds ?? []).includes(category.id)), files: (data.sourcePaths ?? []).map((path: string, index: number) => ({ id: `${id}-${index}`, originalFilename: path.split(/[\\/]/).pop(), byteSize: 0 })), relations: [] }; assets.unshift(asset); return { assetId: id, title: asset.title, fileCount: asset.files.length, totalBytes: 0 } as T; }
    case "update_asset": { const asset = assets.find((item) => item.id === data.id); if (asset) Object.assign(asset, data); return undefined as T; }
    case "set_asset_tags": { const asset = assets.find((item) => item.id === data.assetId); if (asset) asset.tags = tags.filter((tag) => data.ids.includes(tag.id)); return undefined as T; }
    case "set_asset_categories": { const asset = assets.find((item) => item.id === data.assetId); if (asset) asset.categories = categories.filter((category) => data.ids.includes(category.id)); return undefined as T; }
    case "create_tag": { const tag = { id: `tag-${nextId++}`, name: data.name, color: data.color ?? null }; tags.push(tag); return tag as T; }
    case "delete_tag": { const index = tags.findIndex((tag) => tag.id === args?.tagId); if (index >= 0) tags.splice(index, 1); assets.forEach((asset) => { asset.tags = asset.tags.filter((tag: AnyRecord) => tag.id !== args?.tagId); }); return undefined as T; }
    case "create_category": { const category = { id: `cat-${nextId++}`, name: data.name }; categories.push(category); return category as T; }
    case "add_asset_relation": return undefined as T;
    case "delete_asset": { const index = assets.findIndex((asset) => asset.id === args?.assetId); if (index >= 0) assets.splice(index, 1); return undefined as T; }
    case "add_asset_files": { const asset = assets.find((item) => item.id === data.assetId); if (asset) asset.files.push(...(data.sourcePaths ?? []).map((path: string) => ({ id: `file-${nextId++}`, originalFilename: path.split(/[\\/]/).pop(), byteSize: 0 }))); return undefined as T; }
    case "delete_asset_file": { assets.forEach((asset) => { asset.files = asset.files.filter((file: AnyRecord) => file.id !== args?.fileId); }); return undefined as T; }
    case "count_legacy_archives": return 3 as T;
    case "import_legacy_directory": return { imported: [], failures: [] } as T;
    case "create_database_backup": return { path: "ブラウザプレビュー/mock-backup.db" } as T;
    default: return undefined as T;
  }
}

export const invoke = <T>(command: string, args?: AnyRecord) => browserPreview ? mockInvoke<T>(command, args) : tauriInvoke<T>(command, args);
export const listen = <T>(event: string, handler: (event: { payload: T }) => void) => browserPreview ? Promise.resolve(() => {}) : tauriListen<T>(event, handler);
