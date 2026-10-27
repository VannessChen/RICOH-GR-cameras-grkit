#ifndef GRKIT_GRF_H
#define GRKIT_GRF_H

/* A GRFX0001 record has GRKIT_GRF_SIZE bytes, or GRKIT_GRF_MAX_SIZE with hue
   levels (feature 16). runtime/abi.h has the layout. */
#include "abi.h"

/* Return one only for a complete, supported GRFX0001 record of `size` bytes. */
int grkit_grf_validate(const unsigned char *data, unsigned long size);

#endif
