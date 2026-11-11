import { X } from "lucide-react";
import type { BoothMetadataRefreshProgress, LegacyImportProgress } from "../types";

type Props = {
  message: string | null;
  error: string | null;
  legacyProgress: LegacyImportProgress | null;
  legacyImportNotice: string | null;
  boothMetadataRefreshProgress: BoothMetadataRefreshProgress | null;
  boothMetadataRefreshNotice: string | null;
  onDismissMessage(): void;
  onDismissError(): void;
  onDismissLegacyNotice(): void;
  onDismissBoothMetadataRefreshNotice(): void;
};

export function ToastStack({
  message,
  error,
  legacyProgress,
  legacyImportNotice,
  boothMetadataRefreshProgress,
  boothMetadataRefreshNotice,
  onDismissMessage,
  onDismissError,
  onDismissLegacyNotice,
  onDismissBoothMetadataRefreshNotice,
}: Props) {
  if (!message && !error && !legacyProgress && !legacyImportNotice && !boothMetadataRefreshProgress && !boothMetadataRefreshNotice) return null;
  return <div className="toast-stack" aria-live="polite">
    {message && <Toast title="完了" value={message} onDismiss={onDismissMessage} />}
    {error && <Toast title="エラー" value={error} error onDismiss={onDismissError} />}
    {(legacyProgress || legacyImportNotice) && <div className="legacy-progress-toast" role="status">
      {legacyProgress ? <><b>旧形式ZIPを取り込み中</b><span>{legacyProgress.completed} / {legacyProgress.total} 件</span><progress value={legacyProgress.completed} max={legacyProgress.total} /></> : <ToastBody title="旧形式ZIPの取り込み完了" value={legacyImportNotice ?? ""} onDismiss={onDismissLegacyNotice} />}
    </div>}
    {(boothMetadataRefreshProgress || boothMetadataRefreshNotice) && <div className="legacy-progress-toast" role="status">
      {boothMetadataRefreshProgress ? <><b>BOOTHメタデータを再取得中</b><span>{boothMetadataRefreshProgress.completed} / {boothMetadataRefreshProgress.total} 件</span><progress value={boothMetadataRefreshProgress.completed} max={boothMetadataRefreshProgress.total} /></> : <ToastBody title="BOOTHメタデータの再取得完了" value={boothMetadataRefreshNotice ?? ""} onDismiss={onDismissBoothMetadataRefreshNotice} />}
    </div>}
  </div>;
}

function Toast({ title, value, error = false, onDismiss }: { title: string; value: string; error?: boolean; onDismiss?: () => void }) {
  return <div className={error ? "legacy-progress-toast toast-error" : "legacy-progress-toast"} role={error ? "alert" : "status"}><ToastBody title={title} value={value} onDismiss={onDismiss} /></div>;
}

function ToastBody({ title, value, onDismiss }: { title: string; value: string; onDismiss?: () => void }) {
  return <><div className="legacy-progress-toast__head"><b>{title}</b>{onDismiss && <button className="icon-button" aria-label="通知を閉じる" onClick={onDismiss}><X aria-hidden="true" size={18} /></button>}</div><span>{value}</span></>;
}
