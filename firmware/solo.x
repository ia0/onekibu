MEMORY
{
  /* See https://github.com/solokeys/solo/blob/master/targets/stm32l432/linker/stm32l4xx.ld */
  FLASH : ORIGIN = 0x08005000, LENGTH = 196K - 8
  RAM   : ORIGIN = 0x20000000, LENGTH = 48K
  SRAM2 : ORIGIN = 0x10000000, LENGTH = 16K
}
