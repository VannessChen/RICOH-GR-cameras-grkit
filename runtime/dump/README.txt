GRKit firmware dump: GR III, GR IIIx, GR IV, and their HDF models

This tool copies the firmware image (RTOS) that the camera runs from memory to
the SD card. It only reads memory. It does not change the camera or its
settings. The image holds Ricoh firmware only, no photos or personal data.

SD scripts must be on. If you installed GRKit, they are on.
While this tool is on the card, GRKit does not load.

Steps
1. Copy the SCRIPT folder of the card to your computer as a backup.
2. Copy the contents of SD_ROOT to the root of the card. Replace SCRIPT/STARTUP.TTL.
3. Turn the camera on normally. Do not hold MENU.
4. Wait 2 minutes. Then turn the camera off.
5. On a computer, open SCRIPT/GRDUMP on the card. DUMP.OK shows a complete dump.
   DUMP.ERR shows a failure.
6. Zip the SCRIPT/GRDUMP folder and send it to the developer.
   Guide: https://github.com/ahonn/grkit/blob/main/resources/DUMP.md
7. Delete SCRIPT/GRDUMP. Restore the SCRIPT folder from your backup.


GRKit 固件导出工具：GR III、GR IIIx、GR IV 及其 HDF 型号

此工具把相机正在运行的固件（RTOS）从内存复制到 SD 卡。它只读取内存，
不修改相机和设置。导出的文件只有理光固件，没有照片和个人数据。

SD 脚本必须已开启。安装过 GRKit 的相机已经开启。
此工具在卡里时，GRKit 不会加载。

步骤
1. 把卡里的 SCRIPT 文件夹复制到电脑备份。
2. 把 SD_ROOT 里的内容复制到卡的根目录，替换 SCRIPT/STARTUP.TTL。
3. 正常开机，不要按住 MENU。
4. 等 2 分钟，然后关机。
5. 在电脑上打开卡里的 SCRIPT/GRDUMP。有 DUMP.OK 表示导出完成，
   有 DUMP.ERR 表示导出失败。
6. 把 SCRIPT/GRDUMP 文件夹压缩后发给开发者。
   指南：https://github.com/ahonn/grkit/blob/main/resources/DUMP.zh-Hans.md
7. 删除 SCRIPT/GRDUMP，用备份恢复 SCRIPT 文件夹。
