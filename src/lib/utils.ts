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
