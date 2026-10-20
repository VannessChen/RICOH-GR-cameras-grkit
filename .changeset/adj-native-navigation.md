---
default: minor
---

Make the GRKit menu in ADJ work as the native ADJ entries do:

- All GRKit rows show, also when the native entry before GRKit has fewer values. Before, some rows were visible only after scrolling.
- A long page, such as Image Control, scrolls one row at a time and shows a scroll bar. Before, it changed six rows at a time with no scroll bar.
- OK on a page row selects that row and closes ADJ. Before, ADJ stayed open on the page.
- An ADJ press on GRKit closes ADJ, as on a native entry. On a page, it also selects the row. Before, an ADJ press on the GRKit root opened the page of the row.
- Left and Right on a page move to the next ADJ entry. Before, Left returned to the GRKit root. MENU still returns to the root.
- ADJ opens GRKit on the row that was last used. Before, it always opened on the first row.
- If ADJ closes while GRKit is open, ADJ opens on GRKit the next time. Before, it opened on the native entry next to GRKit.
