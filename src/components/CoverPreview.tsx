import { useEffect, useState } from "react";
import { convertFileSrc } from "@tauri-apps/api/core";

type Props = {
  localPath?: string | null;
  remoteUrl?: string | null;
  fallbackPath?: string | null;
  status: string;
};

export function CoverPreview({ localPath, remoteUrl, fallbackPath, status }: Props) {
  const source = localPath
    ? convertFileSrc(localPath)
    : remoteUrl ?? (fallbackPath ? convertFileSrc(fallbackPath) : null);
  const [failed, setFailed] = useState(false);

  useEffect(() => setFailed(false), [source]);

  return (
    <div className="cover-preview">
      <div className="cover-preview__image">
        {source && !failed ? (
          <img src={source} alt="選択中のカバー画像" onError={() => setFailed(true)} />
        ) : (
          "プレビューなし"
        )}
      </div>
      <p>{failed ? "画像を表示できません" : status}</p>
    </div>
  );
}
