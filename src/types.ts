export type Status = {
  defaultAssetStoragePath: string;
  defaultDatabaseBackupPath: string;
  defaultDatabasePath: string;
  initialized: boolean;
  assetCount: number;
  assetStoragePath: string | null;
  databaseBackupPath: string | null;
  databasePath: string | null;
};
export type Tag = { id: string; name: string; color: string | null };
export type Category = { id: string; name: string };
export type LibraryViewSize = "large" | "medium" | "small";
export type Asset = {
  id: string;
  title: string;
  creator: string | null;
  searchText: string;
  coverPath: string | null;
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
  productUrl: string | null;
  note: string | null;
  createdAt: number;
  updatedAt: number;
  coverPath: string | null;
  tags: Tag[];
  categories: Category[];
  files: FileItem[];
  relations: Relation[];
};
export type Capacity = {
  assetBytes: number;
  databaseBytes: number;
  databaseBackupBytes: number;
};
export type BackupEntry = { path: string; name: string; byteSize: number };
export type LegacyImportResult = {
  imported: { assetId: string; title: string }[];
  failures: { sourceFilename: string; reason: string }[];
};
export type LegacyImportProgress = { completed: number; total: number };
export type LegacyImportComplete = {
  result: LegacyImportResult | null;
  error: string | null;
};
export type BoothMetadataRefreshProgress = { completed: number; total: number };
export type BoothMetadataRefreshResult = {
  updated: number;
  failures: { title: string; reason: string }[];
};
export type BoothMetadataRefreshComplete = {
  result: BoothMetadataRefreshResult | null;
  error: string | null;
};
