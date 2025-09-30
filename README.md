# {{ project-name }}

AtCoder向けRustテンプレート．

## コマンド
- すべてテスト: `cargo xtask test` または `just test-all`
- 個別テストのみ: `cargo xtask test a` または `just test a`
- サンプル実行: `cargo xtask run a 1` または `just run a`/`just run a 1`（ケース未指定時は`001`）
- サンプル取得: `cargo xtask fetch abc999 a` や `cargo xtask fetch a`、`just fetch a` / `just fetch abc999 a`（コンテストIDを省略するとカレントディレクトリ名を利用、`just fetch-overwrite a` で再取得）

`just run ...` などを使う場合は [`just`](https://github.com/casey/just) をインストールしてください。

## テストケース
- 例: `tests/a/001.in`に対する期待出力 → `tests/a/001.out`
- 初期状態ではサンプルは含まれないので、必要に応じて `cargo xtask fetch ...` で取得してください。
