ENTRY(_start)

/* In Ring 3, the ELF loader maps programs usually starting at an offset 
   like 0x80000000 (2 GiB) or 0x400000 (4 MiB). For AtomicOS's Phase 5 
   the ELF loader dynamically maps based on the ELFs virtual addresses.
   We will link the userland binaries starting at 0x400000. */
   
SECTIONS {
    . = 0x400000;

    .text : ALIGN(4K) {
        *(.text .text.*)
    }

    .rodata : ALIGN(4K) {
        *(.rodata .rodata.*)
    }

    .data : ALIGN(4K) {
        *(.data .data.*)
    }

    .bss : ALIGN(4K) {
        *(.bss .bss.*)
        *(COMMON)
    }
    
    /DISCARD/ : {
        *(.eh_frame)
        *(.comment)
    }
}
