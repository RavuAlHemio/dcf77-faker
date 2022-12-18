/* ATSAML21G18B memory layout */

MEMORY {
    /* K = 1024 bytes */

    FLASH : ORIGIN = 0x00000000, LENGTH = 256K
    RAM : ORIGIN = 0x20000000, LENGTH = 32K
}
