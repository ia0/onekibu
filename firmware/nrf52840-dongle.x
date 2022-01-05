__stack_size = 0x10000;

MEMORY
{
  /* Apparently the bootloader takes the first and last page of the flash.
     See https://github.com/ferrous-systems/embedded-trainings-2020/blob/main/boards/dongle/memory.x
   */
  FLASH : ORIGIN = 0x00001000, LENGTH = 0x00100000 - 0x2000
  RAM   : ORIGIN = 0x20000000 + __stack_size, LENGTH = 0x00040000 - __stack_size
}

_stack_start = ORIGIN(RAM);
__eheap = ORIGIN(RAM) + LENGTH(RAM);
