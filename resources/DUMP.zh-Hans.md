# 固件导出工具

固件导出工具把相机正在运行的固件映像从内存复制到存储卡。开发者用这个映像来支持 GRKit 还不认识的固件版本。

此工具只读取内存，不修改相机、固件和相机设置。导出的文件只包含理光固件，不包含照片和个人数据。

适用于 RICOH GR III、GR IIIx、GR IV 及其 HDF 型号。所有相机使用同一个 ZIP。

## 开始之前

- SD 脚本功能必须已经打开。安装 GRKit 时会打开它。如果你没有安装过 GRKit，先按相机的安装指南完成第 3 步。
- 此工具在存储卡上时，GRKit 不会加载。
- 使用充满电的电池。

## 获取工具

从[最新版本](https://github.com/ahonn/grkit/releases/latest)下载 `GRKit-<版本>-dump.zip`。https://grkit.app 页面底部的“固件导出工具”链接提供同一个文件。

## 导出固件

1. 关闭相机。取出存储卡。
2. 把存储卡上的 `SCRIPT` 文件夹复制到电脑，作为备份。
3. 把 ZIP 中 `SD_ROOT` 的内容复制到存储卡根目录。替换 `SCRIPT/STARTUP.TTL`。
4. 插入存储卡。正常开机，不要按住 MENU。
5. 等待 2 分钟，然后关机。
6. 在电脑上打开存储卡上的 `SCRIPT/GRDUMP`。

如果文件夹中有 `DUMP.OK`，表示导出完成。文件夹中还有约 20 MB 的 `RTOS.BIN` 和 `INFO.TXT`。

如果文件夹中有 `DUMP.ERR`，表示导出失败。`DUMP.ERR` 记录了错误代码。仍然发送这个文件夹。

如果两个文件都没有，表示相机没有运行此工具。确认 SD 脚本功能已经打开，然后重新执行第 4 到第 6 步。

## 发送文件

1. 把 `SCRIPT/GRDUMP` 文件夹压缩成 ZIP。
2. 把 ZIP 附加到一个 [GitHub issue](https://github.com/ahonn/grkit/issues) 中。写明相机型号和固件版本。

如果开发者在某个 issue 中请你导出，把 ZIP 附加到那个 issue。

## 恢复存储卡

1. 删除存储卡上的 `SCRIPT` 文件夹。
2. 把备份的 `SCRIPT` 文件夹复制到存储卡。

下次开机时，GRKit 会重新加载。
