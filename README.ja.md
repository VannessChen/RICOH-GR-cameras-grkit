# GRKit

[English](README.md) | [简体中文](README.zh-Hans.md) | [繁體中文](README.zh-Hant.md) | 日本語

[![Tests](https://github.com/ahonn/grkit/actions/workflows/tests.yml/badge.svg)](https://github.com/ahonn/grkit/actions/workflows/tests.yml)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)

GRKit は、RICOH GR カメラにフィルムルックと撮影ツールを追加します。メモリーカードから読み込まれ、カメラのファームウェアには書き込みません。

![ADJ モードの GRKit のページ](https://github.com/ahonn/grkit/raw/main/resources/images/grkit-root.jpg)

写真：nagi usano による [Snow in Akihabara](https://www.flickr.com/photos/cafetricot/51803682755/)、[CC BY 2.0](https://creativecommons.org/licenses/by/2.0/)。

## 機能

GRKit は ADJ モードに 6 番目の項目 GRK を追加します。GRK には 5 つのモジュールがあります。

- **イメージコントロール（Image Control）**：Portra 400、Velvia 100、Tri-X 400 など、16 種類のフィルムルックです。撮影画面でルックをプレビューできます。
- **フレーミング（Framing）**：XPan（65:24）、16:9、4:3 の写真を撮影できます。撮影画面では、写真の範囲外が暗く表示されます。
- **グリッド（Guides）**：9 分割、画面中央、黄金比、対角線の線を表示します。
- **日付入れ（Date Stamp）**：フィルムカメラのデータバックのように、写真にオレンジ色の日付を入れます。
- **粒状感（Grain）**：写真にフィルムの粒状感を加えます。強さは 3 段階です。

最後の行には GRKit のバージョンが表示されます。

モジュールはパッケージの作成時に選びます。メニューには、選んだモジュールだけが表示されます。

## 対応カメラ

| カメラ | ファームウェア | 状態 |
| --- | --- | --- |
| RICOH GR III | 2.10 | 実機でテスト済み |
| RICOH GR III HDF | 2.10 | 対応済み。GR III と同じファームウェアとランタイムです。 |
| RICOH GR IIIx | 1.60 | 対応済み。実機でテスト済みの GR III と同じランタイムです。 |
| RICOH GR IIIx HDF | 1.60 | 対応済み。GR IIIx と同じファームウェアとランタイムです。 |
| RICOH GR IV | 1.11 | 実機でテスト済み。フレーミングは XPan のみです。 |
| RICOH GR IV HDF | 1.11 | 対応済み。GR IV と同じファームウェアとランタイムです。フレーミングは XPan のみです。 |

HDF モデルはベースのカメラと同じファームウェアで動作し、同じパッケージを使います。ほかの機種やファームウェアのバージョンには対応していません。ファームウェアのバージョンは、カメラのメニューで確認できます。

> [!WARNING]
> GRKit は試験的なプロジェクトです。自己責任で使用してください。
> GRKit がカメラのメモリーを変更するのは、電源を切るまでの間だけです。インストールするとカメラの SD スクリプトの設定がオンになり、オフにするまでオンのままです。
> GRKit に問題が起きると、起動の途中でカメラが止まることがあります。インストールの前に[復旧ガイド](resources/RECOVERY.ja.md)を読んでください。

## インストール

パッケージを作成するには、[grkit.app](https://grkit.app) を開きます。カメラ、モジュール、エフェクトを選び、`GRKit.zip` をダウンロードします。

各パッケージには、そのカメラのインストールガイドが、選んだ言語の PDF で入っています。復旧ガイドはその末尾にあります。ソースファイルは次のとおりです。

- [GR III、GR IIIx とそれぞれの HDF モデル](resources/INSTALLATION.ja.md)
- [GR IV と GR IV HDF](resources/INSTALLATION-GR4.ja.md)

GRKit を削除するには、カメラの電源を切ってから、カードの `SCRIPT` フォルダーを削除します。
SD スクリプトの設定をオフにするには、[grkit.app](https://grkit.app) のインストール手順の下にあるファイルを使います。手順はインストールガイドにあります。

## ソースからのビルド

Rust 1.92 以降、ARM に対応した Clang と LLD、Ricoh の公式ファームウェアファイルが必要です。リポジトリにはファームウェアは含まれていません。

```sh
cargo install --path crates/grkit-cli --locked
grkit effects build --source effects --out build/effects
grkit build --target gr3-2.10 --firmware firmware/fwdc239b.bin --out build/runtime
sh .github/scripts/install-guide-tools.sh build/guide-tools
sh .github/scripts/build-guides.sh grkit build/guide-tools build/guides
grkit package --build build/runtime --effects build/effects --guide build/guides/gr3/en.pdf --out dist/GRKit.zip
grkit verify dist/GRKit.zip
```

- ガイド：`install-guide-tools.sh` は Typst と Noto Sans SC・TC・JP フォントをダウンロードし、SHA-256 を検証します。Linux x86_64 と macOS arm64 に対応しています。`build-guides.sh` は `build/guides/<カメラ>/<言語>.pdf` を書き出します。
- GR IIIx：`--camera gr3x` と `--guide build/guides/gr3x/en.pdf` を指定してパッケージを作成します。ランタイムは GR III と同じです。
- HDF モデル：ベースのカメラ用のパッケージを作成します。たとえば GR III HDF では `--camera gr3` を指定します。
- GR IV：`fwdc248b.bin` から `--target gr4-1.11` でビルドし、`--camera gr4` と `--guide build/guides/gr4/en.pdf` を指定してパッケージを作成します。
- プラグインを減らす：インストールするプラグインを `--plugins` で指定します。例：`--plugins framing,guides`。インストーラーは、ほかのプラグインのためにファームウェアを変更しません。`--effects` を指定しない場合、パッケージにイメージコントロールは含まれません。
- ガイドの言語：`--language zh-Hans`、`--language zh-Hant`、`--language ja` のいずれかを追加し、その言語のガイドを指定します。例：`build/guides/gr3/zh-Hans.pdf`。

## リポジトリの構成

| パス | 内容 |
| --- | --- |
| `crates/grkit-core` | エフェクトの形式とパッケージの規則。プラットフォームの I/O は含みません |
| `crates/grkit-build` | ファームウェアファイルからのランタイムのビルド |
| `crates/grkit-cli` | `grkit` コマンド |
| `crates/grkit-wasm` | ブラウザーでのパッケージの組み立て |
| `runtime/` | カメラ上で動作するコード |
| `effects/` | イメージコントロールのエフェクトのソース |
| `web/` | ブラウザーでパッケージを作成するサイト |
| `resources/` | インストールガイドと復旧ガイド |
| `docs/` | アーキテクチャとリリース手順 |

## コントリビューション

セットアップ、テスト、プルリクエストのルールは [CONTRIBUTING.md](CONTRIBUTING.md) を参照してください。
GRKit を実機でテストした場合は、Device report の Issue を作成し、成功か失敗かの結果を報告してください。GR IIIx と HDF モデルの報告が特に役立ちます。

## ドキュメント（英語）

- [アーキテクチャ](docs/architecture.md)
- [リリース手順](docs/releasing.md)
- [エフェクト](effects/README.md)
- [ウェブサイト](web/README.md)

## ライセンス

コードは [MIT ライセンス](LICENSE)で提供されています。各パッケージには、含まれるファイルのライセンスを記載した `LICENSES.md` が入っています。

- エフェクトのライセンスは CC BY-SA 4.0 または MIT です。各エフェクトのライセンスは [effects/NOTICE.md](effects/NOTICE.md) に記載されています。
- アイコンの文字には、[Inter](https://github.com/rsms/inter) 書体を [SIL Open Font License 1.1](crates/grkit-core/assets/Inter-LICENSE.txt) のもとで使用しています。
- エフェクトのフィッティング手法は、[gr3-film-mods](https://github.com/idontknownihil/gr3-film-mods) から [MIT ライセンス](crates/grkit-core/GR3_FILM_MODS_LICENSE.txt)のもとで移植したものです。

RICOH および GR は、Ricoh Company, Ltd. の商標です。フィルム名は、それぞれの所有者の商標です。GRKit は独立したプロジェクトであり、Ricoh やフィルムメーカーとは提携しておらず、承認や支援も受けていません。
