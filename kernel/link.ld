OUTPUT_FORMAT("elf64-x86-64")
ENTRY(_start)

SECTIONS
{
    . = 0x100000; /* dirección base del kernel */

    .text : {
        __text_start = .;
        *(.text*)
        __text_end = .;
    }

    .rodata : { *(.rodata*) }

    .data : {
        __data_start = .;
        *(.data*)
        __data_end = .;
    }

    .bss : {
        __bss_start = .;
        *(.bss*)
        *(COMMON)
        __bss_end = .;
    }

    /* Reservar espacio para la pila */
    .stack (NOLOAD) : {
        __stack_start = .;
        . += 16K;  /* tamaño de la pila */
        __stack_end = .;
    }
}

