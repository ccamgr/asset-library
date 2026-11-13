export const nameOf = (path: string) => path.split(/[\\/]/).pop() ?? path;

export const titleOf = (path: string) => nameOf(path).replace(/\.[^.]+$/, "");

export const bytes = (value: number) =>
  value >= 1024 ** 3
    ? `${(value / 1024 ** 3).toFixed(1)} GB`
    : value >= 1024 ** 2
      ? `${(value / 1024 ** 2).toFixed(1)} MB`
      : value >= 1024
        ? `${(value / 1024).toFixed(1)} KB`
        : `${value} B`;

export const nullable = (value: string) => value.trim() || null;

export const directoryOf = (path: string) => {
  const normalized = path.trim().replace(/[\\/]+$/, "");
  const separator = Math.max(normalized.lastIndexOf("/"), normalized.lastIndexOf("\\"));
  if (separator < 0) return normalized;
  if (separator === 0) return normalized.slice(0, 1);
  if (separator === 2 && normalized[1] === ":") return normalized.slice(0, 3);
  return normalized.slice(0, separator);
};

export const appendPath = (directory: string, filename: string) => {
  const normalized = directory.trim().replace(/[\\/]+$/, "");
  const separator = normalized.includes("\\") ? "\\" : "/";
  return `${normalized || "<保存先ディレクトリ>"}${separator}${filename}`;
};
