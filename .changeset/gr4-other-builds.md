---
default: patch
---

Some GR IV and GR IV HDF cameras run another build of firmware 1.11. On these cameras, GRKit now finds the firmware addresses in the running build. Before, the installation stopped with status 10. If GRKit cannot find an address, it stops with status 47 and changes nothing. `INSTALL.ERR` then lists what it could not find. No camera with another build of 1.11 has been tested yet.
