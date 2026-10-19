# GRKit

僅適用於 RICOH GR III 與 GR III HDF 韌體 2.10。

## 安裝

1. 關閉相機。
2. 把 `SD_ROOT` 中的內容複製到記憶卡根目錄。
3. 插入記憶卡。按住 MENU 按鈕開機。等待相機自動關機。
4. 正常開機。

第 3 步會開啟相機的 SD 腳本功能。相機先顯示 Mode Set Command，接著顯示 Complete，之後自動關機。

![Mode Set Command 畫面顯示 Script - Enable 和 Complete。](images/modeset.png)

如果 SD 腳本功能已經開啟，跳過第 3 步。
安裝完成後，你可以從記憶卡根目錄刪除 `00078350.588` 與 `DEVELOP.MOD`。這一步是選用的。

## 使用

在拍攝模式下按下 ADJ 桿進入 ADJ 模式，然後選擇第 6 個功能 GRK。GRKit 在開機幾秒後才載入，所以 GRK 可能稍後才出現。

![GRKit 頁面列出模組，並顯示目前的效果。](images/grkit-root.jpg)

照片：nagi usano 拍攝的 [Snow in Akihabara](https://www.flickr.com/photos/cafetricot/51803682755/)，[CC BY 2.0](https://creativecommons.org/licenses/by/2.0/)。

- **影像控制（Image Control）：** 反白一個效果，即可在實時顯示中預覽。按 OK 按鈕保留這個效果。按 Fn 按鈕調整這個效果的 [標準] 參數。每個效果單獨保存自己的調整，不會改變相機的影像控制設定。
- **畫幅（Framing）：** 選擇 XPan、16:9 或 4:3。顯示屏上照片以外的區域會變暗。只有當相機的 [長寬比] 為 3:2 時，畫幅才會生效。
- **輔助線（Guides）：** 選擇一種輔助線樣式。輔助線只顯示在顯示屏上，不會出現在照片中。
- **加印日期（Date Stamp）：** 選擇一種日期樣式。照片右下角會加印日期。
- **粒紋感（Grain）：** 選擇弱、中或強。粒紋感只加在照片上，不影響影片。

反白最後一行 GRK，即可看到 GRKit 的版本號。

按 MENU 按鈕返回上一層。關機後 GRKit 會保留你的設定。

## 移除或復原

要移除 GRKit，先關閉相機。然後從記憶卡刪除 `SCRIPT` 資料夾。

如果相機在啟動時停止回應，取出電池與記憶卡。然後不插記憶卡開機。如果仍然無法啟動，請參閱[復原指南](https://github.com/ahonn/grkit/blob/main/resources/RECOVERY.zh-Hant.md)。

GRKit 與 Ricoh 無關。原始碼與授權：https://github.com/ahonn/grkit
