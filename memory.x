/* Based on https://github.com/rp-rs/rp-hal/blob/c8bb2e43c792dd3975a255d7eba479547411aec6/memory.x */
MEMORY {
    BOOT2 : ORIGIN = 0x10000000, LENGTH = 0x100
    FLASH : ORIGIN = 0x10000100, LENGTH = 4096K - 0x100
    STATIC_FLASH : ORIGIN = 0x10400000, LENGTH = 16384K - 4096K
    RAM   : ORIGIN = 0x20000000, LENGTH = 256K
}

SECTIONS {
    /* ### Boot loader */
    .boot2 ORIGIN(BOOT2) :
    {
        KEEP(*(.boot2));
    } > BOOT2

    .static_rodata ORIGIN(STATIC_FLASH):
    {
        *(.static_rodata)
    } > STATIC_FLASH
} INSERT BEFORE .text;
