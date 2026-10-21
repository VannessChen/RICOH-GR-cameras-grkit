# Firmware dump tool

The firmware dump tool copies the firmware image that the camera runs from memory to the SD card. The developer uses this image to support a firmware build that GRKit does not know yet.

The tool only reads memory. It does not change the camera, the firmware, or the camera settings. The files contain Ricoh firmware only. They do not contain photos or personal data.

For RICOH GR III, GR IIIx, GR IV, and their HDF models. One ZIP serves each camera.

## Before you start

- SD scripts must be on. The GRKit installation turns them on. If you did not install GRKit, do step 3 of the installation guide for your camera first.
- While the tool is on the card, GRKit does not load.
- Use a charged battery.

## Get the tool

Download `GRKit-<version>-dump.zip` from the [latest release](https://github.com/ahonn/grkit/releases/latest). The Firmware dump tool link at the bottom of https://grkit.app gives the same file.

## Dump the firmware

1. Turn the camera off. Remove the card.
2. Copy the `SCRIPT` folder of the card to your computer as a backup.
3. Copy the contents of `SD_ROOT` from the ZIP to the root of the card. Replace `SCRIPT/STARTUP.TTL`.
4. Insert the card. Turn the camera on normally. Do not hold MENU.
5. Wait 2 minutes. Then turn the camera off.
6. On a computer, open `SCRIPT/GRDUMP` on the card.

If the folder has `DUMP.OK`, the dump is complete. The folder also has `RTOS.BIN`, about 20 MB, and `INFO.TXT`.

If the folder has `DUMP.ERR`, the dump failed. `DUMP.ERR` holds the error code. Send the folder anyway.

If the folder has neither file, the camera did not run the tool. Make sure that SD scripts are on. Then do steps 4 to 6 again.

## Send the files

1. Zip the `SCRIPT/GRDUMP` folder.
2. Attach the ZIP to a [GitHub issue](https://github.com/ahonn/grkit/issues). Give the camera model and the firmware version.

If the developer asked for the dump in an issue, attach the ZIP to that issue.

## Restore the card

1. Delete the `SCRIPT` folder from the card.
2. Copy the `SCRIPT` folder from your backup to the card.

GRKit loads again at the next startup.
