__stack_size = 0x10000;

MEMORY
{
  /* Apparently the bootloader takes the first page as well as the last 12 pages
     of the flash.
     See https://github.com/RIOT-OS/RIOT/blob/master/boards/nrf52840-mdk-dongle/Makefile.include
   */
  FLASH : ORIGIN = 0x00001000, LENGTH = 0x00100000 - 0xd000
  RAM   : ORIGIN = 0x20000000 + __stack_size, LENGTH = 0x00040000 - __stack_size
}

_stack_start = ORIGIN(RAM);
__eheap = ORIGIN(RAM) + LENGTH(RAM);
