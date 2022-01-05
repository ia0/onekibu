__stack_size = 0x1000;

MEMORY
{
  /* See https://github.com/solokeys/solo/blob/master/targets/stm32l432/linker/stm32l4xx.ld */
  FLASH : ORIGIN = 0x08000000 + 0x5000, LENGTH = 0x00040000 - 0x10000
  RAM   : ORIGIN = 0x20000000 + __stack_size, LENGTH = 0x0000c000 - __stack_size
  SRAM2 : ORIGIN = 0x10000000, LENGTH = 0x00004000
}

_stack_start = ORIGIN(RAM);
__eheap = ORIGIN(RAM) + LENGTH(RAM);
