export type Status = {
  defaultAssetStoragePath: string;
  defaultDatabaseBackupPath: string;
  initialized: boolean;
  assetCount: number;
  assetStoragePath: string | null;
  databaseBackupPath: string | null;
};
export type Tag = { id: string; name: string; color: string | null };
export type Category = { id: string; name: string };
export type Asset = {
  id: string;
  title: string;
  creator: string | null;
  searchText: string;
  coverDataUrl: string | null;
  tags: Tag[];
  categories: Category[];
};
export type FileItem = {
  id: string;
  originalFilename: string;
  byteSize: number;
};
export type Relation = {
  assetId: string;
  title: string;
  relationType: string;
  direction: string;
};
export type Detail = {
  id: string;
  title: string;
  creator: string | null;
  source: string | null;
  productUrl: string | null;
  version: string | null;
  note: string | null;
  coverDataUrl: string | null;
  tags: Tag[];
  categories: Category[];
  files: FileItem[];
  relations: Relation[];
};
export type Capacity = { assetBytes: number; databaseBackupBytes: number };
export type BackupEntry = { path: string; name: string; byteSize: number };
export type LegacyImportResult = {
  imported: { assetId: string; title: string }[];
  failures: { sourceFilename: string; reason: string }[];
};
export type LegacyImportProgress = { completed: number; total: number };
