# AI Agent Rules Index

このディレクトリが、Codex と GitHub Copilot で共有するプロジェクト規約の正本です。

| 作業対象 | 必ず読む規約 | 追加で読む規約 |
| --- | --- | --- |
| 全作業 | [common.md](./common.md) | 対象に応じた規約 |
| `src/`、`src-tauri/`、`makefile` | [desktop/instructions.md](./desktop/instructions.md) | 必要に応じて `docs/design/` の資料 |
| `docs/`、`.agents/`、デプロイ・リリース | [documentation.md](./documentation.md) | 必要に応じてデスクトップ規約 |

エージェント固有の入口ファイルは、この規約を参照するだけに保ちます。規約を変更する場合は、まずこのディレクトリを更新します。対象別規約と共通規約が矛盾するときは、対象別規約を優先します。
