# GRKit

[English](README.md) | 简体中文 | [繁體中文](README.zh-Hant.md)

[![Tests](https://github.com/ahonn/grkit/actions/workflows/tests.yml/badge.svg)](https://github.com/ahonn/grkit/actions/workflows/tests.yml)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)

GRKit 为 RICOH GR 相机增加胶片风格和拍摄辅助功能。它从存储卡加载，不改写相机固件。

![ADJ 模式中的 GRKit 页面](https://github.com/ahonn/grkit/raw/main/resources/images/grkit-root.png)

## 功能

GRKit 在 ADJ 模式中增加第 6 个功能 GRK，包含五个模块：

- **影像控制（Image Control）**：16 种胶片风格，例如 Portra 400、Velvia 100 和 Tri-X 400。可以在实时显示中预览效果。
- **画幅（Framing）**：拍摄 XPan（65:24）、16:9 或 4:3 照片。显示屏上照片以外的区域会变暗。
- **辅助线（Guides）**：三分法、中心、黄金分割或对角线。
- **加印日期（Date Stamp）**：在照片上加印橙色日期，效果类似胶片相机的数据后背。
- **颗粒感（Grain）**：为照片添加胶片颗粒感，有弱、中、强三档。

最后一行显示 GRKit 的版本号。

制作安装包时，你可以选择包含哪些模块。菜单只显示这些模块。

## 支持的机型

| 机型 | 固件 | 状态 |
| --- | --- | --- |
| RICOH GR III | 2.10 | 已经过真机测试 |
| RICOH GR IIIx | 1.60 | 尚未经过真机测试 |
| RICOH GR IV、GR IV HDF | 1.11 | 实验性支持，尚未经过真机测试。画幅只提供 XPan。 |

不支持其他机型和固件版本。你可以在相机菜单中查看固件版本。

> [!WARNING]
> GRKit 是实验性项目，使用风险由你自行承担。
> GRKit 只在相机开机期间修改内存，关机后修改即失效。安装过程会打开相机的 SD 脚本设置，这个设置会一直保持打开。
> 如果 GRKit 出错，相机可能在启动时停止响应。安装前请先阅读[恢复指南](resources/RECOVERY.zh-Hans.md)。

## 安装

每个安装包都包含安装指南，其中有对应机型的安装步骤和菜单用法：

- [GR III 和 GR IIIx](resources/INSTALLATION.zh-Hans.md)
- [GR IV 和 GR IV HDF](resources/INSTALLATION-GR4.zh-Hans.md)

要移除 GRKit，请先关闭相机，再删除存储卡中的 `SCRIPT` 文件夹。

## 从源码构建

你需要 Rust 1.92 或更高版本、支持 ARM 的 Clang 和 LLD，以及 Ricoh 官方固件文件。仓库中不包含固件。

```sh
cargo install --path crates/grkit-cli --locked
grkit effects build --source effects --out build/effects
grkit build --target gr3-2.10 --firmware firmware/fwdc239b.bin --out build/runtime
grkit package --build build/runtime --effects build/effects --out dist/GRKit.zip
grkit verify dist/GRKit.zip
```

- GR IIIx：在 `grkit package` 后加 `--camera gr3x`。它和 GR III 使用同一个运行时。
- GR IV：用 `fwdc248b.bin` 和 `--target gr4-1.11` 构建，再用 `--camera gr4` 打包。
- 减少插件：用 `--plugins` 列出要安装的插件，例如 `--plugins framing,guides`。安装器不会为其他插件修改固件。不加 `--effects` 时，安装包中没有 Image Control。
- 指南语言：加 `--language zh-Hans` 或 `--language zh-Hant`。安装包会在英文指南旁加入对应语言的指南。

## 仓库结构

| 路径 | 内容 |
| --- | --- |
| `crates/grkit-core` | 效果格式和安装包规则，不含平台 I/O |
| `crates/grkit-build` | 从固件文件构建运行时 |
| `crates/grkit-cli` | `grkit` 命令 |
| `crates/grkit-wasm` | 在浏览器中组装安装包 |
| `runtime/` | 在相机上运行的代码 |
| `effects/` | Image Control 效果的源文件 |
| `web/` | 在浏览器中制作安装包的网站 |
| `resources/` | 安装指南和恢复指南 |
| `docs/` | 架构和发布流程 |

## 参与贡献

环境配置、测试和提交规则见 [CONTRIBUTING.md](CONTRIBUTING.md)。
如果你在真机上测试了 GRKit，请提交一个 Device report issue，说明结果是通过还是失败。GR IIIx 和 GR IV 的报告最有帮助。

## 文档（英文）

- [架构](docs/architecture.md)
- [发布流程](docs/releasing.md)
- [效果](effects/README.md)
- [网站](web/README.md)

## 许可

代码采用 [MIT 许可证](LICENSE)。每个安装包都包含 `LICENSES.md`，列出包内文件的许可。

- 效果采用 CC BY-SA 4.0 或 MIT 许可。[effects/NOTICE.md](effects/NOTICE.md) 列出每个效果的许可。
- 图标文字使用 [Inter](https://github.com/rsms/inter) 字体，采用 [SIL Open Font License 1.1](crates/grkit-core/assets/Inter-LICENSE.txt)。
- 效果拟合方法移植自 [gr3-film-mods](https://github.com/idontknownihil/gr3-film-mods)，采用 [MIT 许可证](crates/grkit-core/GR3_FILM_MODS_LICENSE.txt)。

RICOH 和 GR 是 Ricoh Company, Ltd. 的商标。胶片名称是其所有者的商标。GRKit 是独立项目，与 Ricoh 及任何胶片厂商无关，也未获得它们的认可或支持。
