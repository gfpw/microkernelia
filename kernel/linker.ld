/* Linker script para kernel x86_64 no_std, 2MiB alineado */
OUTPUT_FORMAT(elf64-x86-64)
ENTRY(_start)
PHDRS {
  text PT_LOAD FLAGS(5);
  data PT_LOAD FLAGS(7);
}
SECTIONS {
  . = 0x200000;
  .text : {
    *(.text .text.*)
  } :text
  .rodata : {
    *(.rodata .rodata.*)
  } :text
  .data : {
    *(.data .data.*)
  } :data
  .bss : {
    *(.bss .bss.* COMMON)
    . = ALIGN(8);
  } :data
  /DISCARD/ : { *(.eh_frame .note .comment) }
}
