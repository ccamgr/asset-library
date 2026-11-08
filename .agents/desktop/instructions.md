# Desktop Rules

対象は `src/`、`src-tauri/`、`makefile` です。

- Rust バックエンドは `src-tauri/`、React / TypeScript UI は `src/` に置き、UI とネイティブ処理の責務を分離します。
- Tauri コマンドまたはイベントを変更した場合は、フロントエンドの呼び出し、型、モック実装を合わせて確認します。
- SQLite スキーマの変更は、既存ライブラリを壊さない移行処理として実装し、既存データとの互換性を検証します。
- アセット本体、サムネイル、DBバックアップはローカル保存が正本です。ストレージ操作は部分失敗時の整合性を保ちます。
- ビルド、リリース、バージョン更新は GitHub Actions の責務です。明示的な依頼なしに実行しません。

## Development Commands

リポジトリルートで、次の `make` コマンドを使います。

- `make run`: Tauri デスクトップアプリケーションを起動します。
- `make run-front`: フロントエンドだけを起動します。
- `make test`: フロントエンドの型検証と Rust テストを実行します。
- `make check`: Rust 書式確認を含む検証を実行します。
- `make fmt`: Rust ソースを整形します。
- `make gen-icons`: デスクトップ用アイコンを生成し、不要なモバイル・ストア用生成物を除外します。
