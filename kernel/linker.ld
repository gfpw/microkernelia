/* Linker script para kernel x86_64 no_std, 2MiB alineado */
OUTPUT_FORMAT(elf64-x86-64)
ENTRY(_start)
PHDRS {
  text PT_LOAD FLAGS(5);
  data PT_LOAD FLAGS(7);
}
SECTIONS {
  . = 0x200000;
  __text_start = .;
  .text : {
    *(.text .text.*)
  } :text
  __text_end = .;
  .rodata : {
    *(.rodata .rodata.*)
  } :text
  __data_start = .;
  .data : {
    *(.data .data.*)
  } :data
  __data_end = .;
  __bss_start = .;
  .bss : {
    *(.bss .bss.* COMMON)
    . = ALIGN(8);
  } :data
  __bss_end = .;
  /* Stack principal: 64KiB, alineado a 16 */
  . = ALIGN(16);
  __stack_start = .;
  . += 0x10000;
  __stack_end = .;
  /DISCARD/ : { *(.eh_frame .note .comment) }
}
