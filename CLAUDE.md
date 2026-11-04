# 開発方針＆開発環境ルール(rs-FFmpeg)

全リポジトリ共通の開発ルール(自動継続・検証徹底等)は
[`open-raid-z`](https://github.com/aon-co-jp/open-raid-z)の`CLAUDE.md`を
正本として参照すること。この節では本リポジトリ固有の事項のみ記す。

## リポジトリの役割

FFmpegへの敬意を込めたRustでのリスペクト版。`RS-Blog`(WordPressのRust版)・
`RS-EC`(EC-CUBEのRust版)と同じ位置づけの長期プロジェクトで、本家との
機能差は現時点で非常に大きい(圧縮コーデック・動画は一切未対応)。
[`make-disk`](https://github.com/aon-co-jp/make-disk)は現状ffmpeg本体を
ラップしているが、本リポジトリが実用段階に達したら置き換え候補となる。

## アーキテクチャ

- `src/wav.rs` — WAV(PCM)の読み込み・書き出し・リサンプル・
  チャンネル数変換・単体テスト
- `src/main.rs` — ffmpeg風の簡易CLI(`probe`サブコマンド、
  `-i`/`-ar`/`-ac`オプション)

## 実装済み(2026-09-12時点)

- 非圧縮PCM WAV(8bit/16bit)の読み込み・解析(`probe`)
- 16bit PCM WAVとしての書き出し
- 線形補間によるサンプルレート変換(高品質フィルタは無し)
- モノラル<->ステレオのチャンネル数変換(2ch以外は非対応)
- 単体テスト4件(往復読み書きの精度・リサンプル後フレーム数・
  ステレオ->モノラル平均化・基本読み込み)全green
- 実機検証: PowerShellで生成した44.1kHzステレオWAVを22.05kHzモノラルへ
  実際に変換し、`probe`で変換後の値が正しいことを確認済み。

## 未実装・既知の制限

- 圧縮音声コーデック(MP3/AAC/FLAC/OGG等)は一切非対応
- 動画コーデック・コンテナは一切非対応
- ビットレート制御・トリミング等、本家ffmpegの大半のオプションは未実装
- リサンプルは線形補間のみ(エイリアシング対策等の高品質フィルタ無し)
- 2ch以外(5.1ch等)のチャンネル変換は非対応

## make-diskとの互換性検証結果(2026-09-12)

`make-disk`が実際に生成する引数(`-ss <t> -i <in> -t <dur> -c:a
libmp3lame -b:a 192k -y <out.mp3>`)で実機検証した結果、**現状は
互換性が無い**ことを確認した:

- 旧版は`-ss`/`-t`/`-c:a`/`-b:a`/`-y`を全て無視し、**exit 0で
  「変換完了」と表示しつつ、実際には無圧縮WAVを`.mp3`という名前で
  書き出す**という危険な不具合があった(`file`コマンドで
  `RIFF...WAVE audio, PCM`と実際に確認)。修正済み: 未対応フラグは
  即エラーとし、入出力とも拡張子`.wav`必須とした。
- 結論として、rs-ffmpegはmake-diskのconvert.rs呼び出し(コーデック
  指定・トリミング・ビットレート指定を伴う)には**現時点で対応できず、
  明示的にエラーで停止する**(黙って壊れたファイルを作ることはない)。
  make-diskがrs-ffmpegを実際に使うには、`-ss`/`-t`/`-c:*`/`-b:*`対応の
  拡張が必要。

## 次にやること候補

- 圧縮コーデックのうち、実装が比較的小さいもの(例: 単純なADPCM)から着手
- リサンプルの品質改善(帯域制限フィルタの追加)
- 3ch以上のチャンネルレイアウト対応

## API 方針: REST 全廃・gRPC / Connect へ (2026-11-04)

### 日本語
- **REST は全廃する。新しい API は REST で作らない。** ブラウザや `curl` から呼ぶ API は **Connect の単純呼び出し**(`POST /<パッケージ>.<Service>/<Method>`、`content-type: application/json`、本文は JSON)、サービス間は **gRPC / gRPC-web**(RPoem の管理API `open_runo.admin.v1.Admin` と同じ流儀)。ストリーミングは、必要になってから。
- **既存の REST は、触るときに順に移す。** 移したら旧 REST の経路は消す。移行が終わるまで残るが、**新規には追加しない**。
- **JSON は RS-JSON に統一**する(読み込みは `from_str_secure` / `parse_secure` でサイズ・深さ・重複キーの上限つき、書き出しは `rust_json::fast`)。ブラウザの WASM に `prost` は入れない(サイズが増えるため)。
- **例外(自分たちの API ではなく、相手の規格が決めているもの)**: ACME、OAuth/OIDC のコールバック、第三者(GitHub など)からの Webhook、ヘルスチェック、静的ファイル配信、OTLP など。これらは規格どおりの HTTP を受ける。
- 参考実装: `RPoem`(`open_runo.admin.v1.Admin`)、`open-easy-web`(`server/src/connect.rs`、`proto/open_easy_web/admin/v1/admin.proto`)。
- ※ このリポジトリのコードが既に方針どおりかどうかは、各リポジトリの現状による。この節は方針の記録であり、コードの変更を意味しない。

### English
- **REST is being abolished. New APIs are not built as REST.** APIs called from browsers or `curl` use **Connect unary calls** (`POST /<package>.<Service>/<Method>`, `content-type: application/json`, JSON body); service-to-service calls use **gRPC / gRPC-web** (the same style as RPoem's admin API `open_runo.admin.v1.Admin`). Streaming only when it is actually needed.
- **Existing REST endpoints are migrated one by one, when they are touched.** The old REST route is removed once migrated. Until migration is finished they remain, but **no new ones are added**.
- **JSON is unified on RS-JSON**: reading with `from_str_secure` / `parse_secure` (size, depth and duplicate-key limits), writing with `rust_json::fast`. `prost` is not put into browser WASM (it would grow the binary).
- **Exceptions (shapes dictated by someone else's standard, not our own API)**: ACME, OAuth/OIDC callbacks, webhooks from third parties (e.g. GitHub), health checks, static file serving, OTLP, and so on. These accept plain HTTP as the standard requires.
- Reference implementations: `RPoem` (`open_runo.admin.v1.Admin`) and `open-easy-web` (`server/src/connect.rs`, `proto/open_easy_web/admin/v1/admin.proto`).
- Note: whether the code in a given repository already follows this policy depends on that repository. This section records the policy; it does not mean the code has been changed.

### Deutsch (KI-Übersetzung, noch nicht von Muttersprachlern geprüft)
- **REST wird abgeschafft. Neue APIs werden nicht als REST gebaut.** APIs, die aus dem Browser oder mit `curl` aufgerufen werden, nutzen **Connect-Unary-Aufrufe** (`POST /<Paket>.<Service>/<Methode>`, `content-type: application/json`, JSON-Body); zwischen Diensten gilt **gRPC / gRPC-web** (wie bei der Admin-API von RPoem, `open_runo.admin.v1.Admin`). Streaming erst, wenn es wirklich nötig ist.
- **Bestehende REST-Endpunkte werden einzeln migriert, sobald man sie anfasst.** Nach der Migration wird die alte REST-Route entfernt. Bis dahin bleiben sie bestehen, es werden aber **keine neuen hinzugefügt**.
- **JSON wird auf RS-JSON vereinheitlicht**: Lesen mit `from_str_secure` / `parse_secure` (Grenzen für Größe, Tiefe und doppelte Schlüssel), Schreiben mit `rust_json::fast`. `prost` kommt nicht in Browser-WASM (die Datei würde größer).
- **Ausnahmen (Formate, die ein fremder Standard vorgibt, nicht unsere eigene API)**: ACME, OAuth/OIDC-Callbacks, Webhooks von Dritten (z. B. GitHub), Health-Checks, statische Dateien, OTLP usw. Diese nehmen schlichtes HTTP entgegen, wie es der Standard verlangt.
- Referenzimplementierungen: `RPoem` (`open_runo.admin.v1.Admin`) und `open-easy-web` (`server/src/connect.rs`, `proto/open_easy_web/admin/v1/admin.proto`).
- Hinweis: Ob der Code eines Repositorys diese Richtlinie schon befolgt, hängt vom jeweiligen Repository ab. Dieser Abschnitt hält die Richtlinie fest; er bedeutet nicht, dass der Code bereits geändert wurde.

### Русский (машинный перевод, носителями языка ещё не проверен)
- **REST полностью отменяется. Новые API не создаются как REST.** API, вызываемые из браузера или через `curl`, используют **унарные вызовы Connect** (`POST /<пакет>.<Service>/<Method>`, `content-type: application/json`, тело — JSON); между сервисами — **gRPC / gRPC-web** (так же, как административный API RPoem, `open_runo.admin.v1.Admin`). Потоковая передача — только когда она действительно нужна.
- **Существующие REST-эндпоинты переносятся по одному, когда до них доходят руки.** После переноса старый REST-маршрут удаляется. До завершения переноса они остаются, но **новые не добавляются**.
- **JSON унифицируется на RS-JSON**: чтение через `from_str_secure` / `parse_secure` (ограничения на размер, глубину и повторяющиеся ключи), запись через `rust_json::fast`. `prost` не добавляется в браузерный WASM (размер вырос бы).
- **Исключения (форматы, заданные чужим стандартом, а не наш собственный API)**: ACME, обратные вызовы OAuth/OIDC, вебхуки сторонних сервисов (например, GitHub), проверки работоспособности, раздача статических файлов, OTLP и т. п. Они принимают обычный HTTP, как требует стандарт.
- Эталонные реализации: `RPoem` (`open_runo.admin.v1.Admin`) и `open-easy-web` (`server/src/connect.rs`, `proto/open_easy_web/admin/v1/admin.proto`).
- Примечание: соответствует ли код конкретного репозитория этой политике, зависит от самого репозитория. Этот раздел фиксирует политику и не означает, что код уже изменён.
