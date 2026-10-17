# GRKit

[English](README.md) | [简体中文](README.zh-Hans.md) | 繁體中文

[![Tests](https://github.com/ahonn/grkit/actions/workflows/tests.yml/badge.svg)](https://github.com/ahonn/grkit/actions/workflows/tests.yml)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)

GRKit 為 RICOH GR 相機加入底片風格與拍攝輔助功能。它從記憶卡載入，不會改寫相機韌體。

![ADJ 模式中的 GRKit 頁面](https://github.com/ahonn/grkit/raw/main/resources/images/grkit-root.png)

## 功能

GRKit 在 ADJ 模式中加入第 6 個功能 GRK，包含五個模組：

- **影像控制（Image Control）**：16 種底片風格，例如 Portra 400、Velvia 100 與 Tri-X 400。可以在實時顯示中預覽效果。
- **畫幅（Framing）**：拍攝 XPan（65:24）、16:9 或 4:3 照片。顯示屏上照片以外的區域會變暗。
- **輔助線（Guides）**：三分法、中心、黃金分割或對角線。
- **加印日期（Date Stamp）**：在照片上加印橘色日期，效果類似底片相機的資料背蓋。
- **粒紋感（Grain）**：為照片加入底片粒紋感，有弱、中、強三檔。

製作安裝包時，你可以選擇包含哪些模組。功能表只會顯示這些模組。

## 支援的機型

| 機型 | 韌體 | 狀態 |
| --- | --- | --- |
| RICOH GR III | 2.10 | 已經過實機測試 |
| RICOH GR IIIx | 1.60 | 尚未經過實機測試 |
| RICOH GR IV、GR IV HDF | 1.11 | 實驗性支援，尚未經過實機測試。畫幅只提供 XPan。 |

不支援其他機型與韌體版本。你可以在相機功能表中查看韌體版本。

> [!WARNING]
> GRKit 是實驗性專案，使用風險由你自行承擔。
> GRKit 只在相機開機期間修改記憶體，關機後修改即失效。安裝過程會開啟相機的 SD 腳本設定，這個設定會一直保持開啟。
> 如果 GRKit 出錯，相機可能在啟動時停止回應。安裝前請先閱讀[復原指南](resources/RECOVERY.zh-Hant.md)。

## 安裝

每個安裝包都包含安裝指南，其中有對應機型的安裝步驟與功能表用法：

- [GR III 與 GR IIIx](resources/INSTALLATION.zh-Hant.md)
- [GR IV 與 GR IV HDF](resources/INSTALLATION-GR4.zh-Hant.md)

要移除 GRKit，請先關閉相機，再刪除記憶卡中的 `SCRIPT` 資料夾。

## 從原始碼建置

你需要 Rust 1.92 或更新版本、支援 ARM 的 Clang 與 LLD，以及 Ricoh 官方韌體檔案。儲存庫中不包含韌體。

```sh
cargo install --path crates/grkit-cli --locked
grkit effects build --source effects --out build/effects
grkit build --target gr3-2.10 --firmware firmware/fwdc239b.bin --out build/runtime
grkit package --build build/runtime --effects build/effects --out dist/GRKit.zip
grkit verify dist/GRKit.zip
```

- GR IIIx：在 `grkit package` 後加上 `--camera gr3x`。它與 GR III 使用同一個執行環境。
- GR IV：用 `fwdc248b.bin` 與 `--target gr4-1.11` 建置，再用 `--camera gr4` 打包。
- 減少模組：加上 `--hide framing,guides,date-stamp,grain`。不加 `--effects` 時，安裝包中沒有 Image Control。
- 指南語言：加上 `--language zh-Hans` 或 `--language zh-Hant`。安裝包會在英文指南旁加入對應語言的指南。

## 儲存庫結構

| 路徑 | 內容 |
| --- | --- |
| `crates/grkit-core` | 效果格式與安裝包規則，不含平台 I/O |
| `crates/grkit-build` | 從韌體檔案建置執行環境 |
| `crates/grkit-cli` | `grkit` 命令 |
| `crates/grkit-wasm` | 在瀏覽器中組裝安裝包 |
| `runtime/` | 在相機上執行的程式碼 |
| `effects/` | Image Control 效果的原始檔 |
| `web/` | 在瀏覽器中製作安裝包的網站 |
| `resources/` | 安裝指南與復原指南 |
| `docs/` | 架構與發布流程 |

## 參與貢獻

環境設定、測試與提交規則請見 [CONTRIBUTING.md](CONTRIBUTING.md)。
如果你在實機上測試了 GRKit，請提交一個 Device report issue，說明結果是通過還是失敗。GR IIIx 與 GR IV 的回報最有幫助。

## 文件（英文）

- [架構](docs/architecture.md)
- [發布流程](docs/releasing.md)
- [效果](effects/README.md)
- [網站](web/README.md)

## 授權

程式碼採用 [MIT 授權條款](LICENSE)。每個安裝包都包含 `LICENSES.md`，列出包內檔案的授權。

- 效果採用 CC BY-SA 4.0 或 MIT 授權。[effects/NOTICE.md](effects/NOTICE.md) 列出每個效果的授權。
- 圖示文字使用 [Inter](https://github.com/rsms/inter) 字型，採用 [SIL Open Font License 1.1](crates/grkit-core/assets/Inter-LICENSE.txt)。
- 效果擬合方法移植自 [gr3-film-mods](https://github.com/idontknownihil/gr3-film-mods)，採用 [MIT 授權條款](crates/grkit-core/GR3_FILM_MODS_LICENSE.txt)。

RICOH 與 GR 是 Ricoh Company, Ltd. 的商標。底片名稱是其所有者的商標。GRKit 是獨立專案，與 Ricoh 及任何底片廠商無關，也未獲得它們的認可或支援。
