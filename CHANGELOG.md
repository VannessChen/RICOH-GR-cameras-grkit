## 1.2.0 (2026-10-22)

### Features

- OK on Date Imprint in the GRK menu opens its settings, where Date Look selects the digits of the date imprint: the classic seven-segment digits, the dot matrix of the GR1s, or the digits of the GR II.
- Color film looks and imported LUTs follow their source more closely with the camera's hue-sector color stage, in photos and in the viewfinder.
- Fn on the Image Control row of the GRK menu opens the adjustments of the selected effect.
- GR III and GR III HDF firmware 2.20 and GR IIIx and GR IIIx HDF firmware 1.70 are supported as experimental, with the same package as 2.10 and 1.60.
- GR IV and GR IV HDF firmware 1.20 are supported as experimental, with a package of their own.

### Fixes

- Guides draw 2-pixel lines in the color of the native grid guide, at the positions of the native thirds.
- GRKit starts 2 seconds later after power-on, which avoids a stop of the camera when the script talks to the camera before it is ready.
- Playback shows the Fn adjustments of the effect that took the photo, not the parameters of the camera's Image Control setting.
- The date stamp is white on monochrome pictures, including the camera's monochrome Image Control types and GR IV Monochrome.

## 1.1.1 (2026-10-21)

### Fixes

- On another build of firmware 1.11, GR IV and GR IV HDF no longer stop when GRKit loads.
- The GRK icon in the ADJ toolbar has the size of the native toolbar icons.
- Guides hide while DISP hides the live-view information, and the Framing ratio label shows only with the full display.
- The GRKit menu icons have the size of the native list icons, and their text keeps clear of the frame.

## 1.1.0 (2026-10-20)

### Features

- Left and Right on a GRKit page move to the next ADJ entry, as they do on the GRKit root.
- OK or an ADJ press on a GRKit page selects the highlighted row and closes ADJ.
- GRKit opens on the row that was last used, and ADJ opens on GRKit if ADJ closed while GRKit was open.
- GRKit in ADJ shows all of its rows and scrolls a long page one row at a time, as a native ADJ entry does.
- Add a Date Stamp style with the day first and the time: `D M 'YY  H:MM`.
- Add a Guides style that shows the thirds grid and the center cross together.

### Fixes

- The About row of the GRKit menu shows a VER icon, so the GRKit root shows only one GRK icon.
- An ADJ press or one MENU press on the GRKit root closes ADJ.
- GR IV and GR IV HDF install effects from an exFAT card with long file names, such as the `._` files that macOS writes.
- On another build of firmware 1.11, GR IV and GR IV HDF install GRKit, or stop with status 47 and list missing addresses in `INSTALL.ERR`.
- The camera runtime has room for more plugins, and saved selections, Fn adjustments, and effects keep working after the update.

## 1.0.1 (2026-10-19)

### Fixes

- Show the film numbers 5219, 5207, and 5293 on the icons of Vision3 500T, Vision3 250D, and EXR 200T, in the camera menu, the viewfinder, and playback.
- Remove the experimental label from GR IV and GR IV HDF firmware 1.11, after a user tested GRKit on a GR IV.
- Support RICOH GR III HDF and GR IIIx HDF. They run the GR III 2.10 and GR IIIx 1.60 firmware and use the same packages, and the site shows each with its base camera.
- Draw the film icons on the site with web fonts, so they stay sharp. The camera keeps its 40 x 40 pixel icons.
- Show the film number under the name of each motion picture and aerial film look on the site, for example "5219 · 2393 print" under Vision3 500T.

## 1.0.0 (2026-10-19)

### Features

- Add GRK, a sixth entry of ADJ mode, with five modules: Image Control, Framing, Guides, Date Stamp, and Grain.
- Add 16 film looks to Image Control, with film cover icons and a live-view preview.
- Adjust each film look with Fn. The adjustments stay with that look and do not change the native Image Control settings.
- Show the GRKit version in the last row of GRK.
- Support RICOH GR III firmware 2.10 and GR IIIx firmware 1.60. GR IV and GR IV HDF firmware 1.11 support is experimental.
- Add the GRKit site, which builds a package in the browser from the selected camera, modules, and effects.
- Preview the effects on a GR IIIx sample or on your own DNG. The site develops the RAW data in the browser.
- Show the camera menu, the site, and the guides in English, Simplified Chinese, Traditional Chinese, and Japanese.
