# GRKit

仅适用于 RICOH GR III 和 GR III HDF 固件 2.10。

## 安装

1. 关闭相机。
2. 把 `SD_ROOT` 中的内容复制到存储卡根目录。
3. 插入存储卡。按住 MENU 按钮开机。等待相机自动关机。
4. 正常开机。

第 3 步会打开相机的 SD 脚本功能。相机先显示 Mode Set Command，然后显示 Complete，之后自动关机。

![Mode Set Command 画面显示 Script - Enable 和 Complete。](images/modeset.png)

如果 SD 脚本功能已经打开，跳过第 3 步。
安装完成后，你可以从存储卡根目录删除 `00078350.588` 和 `DEVELOP.MOD`。这一步是可选的。

## 使用

在拍摄模式下按下 ADJ 杆进入 ADJ 模式，然后选择第 6 个功能 GRK。GRKit 在开机几秒后才加载，所以 GRK 可能稍后才出现。

![GRKit 页面列出模块，并显示当前效果。](images/grkit-root.jpg)

照片：nagi usano 拍摄的 [Snow in Akihabara](https://www.flickr.com/photos/cafetricot/51803682755/)，[CC BY 2.0](https://creativecommons.org/licenses/by/2.0/)。

- **影像控制（Image Control）：** 高亮一个效果，即可在实时显示中预览。按 OK 按钮保留这个效果。按 Fn 按钮调整这个效果的 [标准] 参数。在 GRK 菜单的影像控制一行按 Fn 按钮，可以调整当前选择的效果。每个效果单独保存自己的调整，不会改变相机的影像控制设置。
- **画幅（Framing）：** 选择 XPan、16:9 或 4:3。显示屏上照片以外的区域会变暗。只有当相机的 [长宽比] 为 3:2 时，画幅才会生效。
- **辅助线（Guides）：** 选择一种辅助线样式。辅助线只显示在显示屏上，不会出现在照片中。
- **加印日期摄影（Date Imprint）：** 按 OK 按钮打开设置。在 **日期样式（Date Style）** 中选择一种日期样式。在 **日期字体（Date Look）** 中选择数字的字体：经典、GR1s 或 GR II。照片右下角会加印日期。
- **颗粒感（Grain）：** 选择弱、中或强。颗粒感只加在照片上，不影响影片。

高亮最后一行 GRK，即可看到 GRKit 的版本号。

按 MENU 按钮返回上一级。关机后 GRKit 会保留你的设定。

## 移除或恢复

要移除 GRKit，先关闭相机。然后从存储卡删除 `SCRIPT` 文件夹。

SD 脚本设置会保持打开。要关闭它：

1. 在 https://grkit.app 下载你的相机对应的关闭 SD 脚本的文件。按钮在安装步骤下方。
2. 把 `SD_ROOT` 中的内容复制到存储卡根目录。
3. 插入存储卡。按住 MENU 按钮开机，等待相机自动关机。
4. 从存储卡根目录删除 `00078350.588` 和 `DEVELOP.MOD`。

如果相机在启动时停止响应，取出电池和存储卡。然后不插存储卡开机。如果仍然无法启动，请参阅[恢复指南](https://github.com/ahonn/grkit/blob/main/resources/RECOVERY.zh-Hans.md)。

GRKit 与 Ricoh 无关。源代码和许可：https://github.com/ahonn/grkit
