# 韌體匯出工具

韌體匯出工具把相機正在執行的韌體映像從記憶體複製到記憶卡。開發者用這個映像來支援 GRKit 還不認識的韌體版本。

此工具只讀取記憶體，不修改相機、韌體和相機設定。匯出的檔案只包含理光韌體，不包含照片和個人資料。

適用於 RICOH GR III、GR IIIx、GR IV 及其 HDF 型號。所有相機使用同一個 ZIP。

## 開始之前

- SD 腳本功能必須已經開啟。安裝 GRKit 時會開啟它。如果你沒有安裝過 GRKit，先依照相機的安裝指南完成第 3 步。
- 此工具在記憶卡上時，GRKit 不會載入。
- 使用充飽電的電池。

## 取得工具

從[最新版本](https://github.com/ahonn/grkit/releases/latest)下載 `GRKit-<版本>-dump.zip`。https://grkit.app 頁面底部的「韌體匯出工具」連結提供同一個檔案。

## 匯出韌體

1. 關閉相機。取出記憶卡。
2. 把記憶卡上的 `SCRIPT` 資料夾複製到電腦，作為備份。
3. 把 ZIP 中 `SD_ROOT` 的內容複製到記憶卡根目錄。取代 `SCRIPT/STARTUP.TTL`。
4. 插入記憶卡。正常開機，不要按住 MENU。
5. 等待 2 分鐘，然後關機。
6. 在電腦上開啟記憶卡上的 `SCRIPT/GRDUMP`。

如果資料夾中有 `DUMP.OK`，表示匯出完成。資料夾中還有約 20 MB 的 `RTOS.BIN` 和 `INFO.TXT`。

如果資料夾中有 `DUMP.ERR`，表示匯出失敗。`DUMP.ERR` 記錄了錯誤代碼。仍然傳送這個資料夾。

如果兩個檔案都沒有，表示相機沒有執行此工具。確認 SD 腳本功能已經開啟，然後重新執行第 4 到第 6 步。

## 傳送檔案

1. 把 `SCRIPT/GRDUMP` 資料夾壓縮成 ZIP。
2. 把 ZIP 附加到一個 [GitHub issue](https://github.com/ahonn/grkit/issues) 中。寫明相機型號和韌體版本。

如果開發者在某個 issue 中請你匯出，把 ZIP 附加到那個 issue。

## 還原記憶卡

1. 刪除記憶卡上的 `SCRIPT` 資料夾。
2. 把備份的 `SCRIPT` 資料夾複製到記憶卡。

下次開機時，GRKit 會重新載入。
