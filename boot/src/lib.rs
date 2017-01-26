//
//  SOS: the Stupid Operating System
//  by Eliza Weisman (hi@hawkweisman.me)
//
//  Copyright (c) 2016-2017 Eliza Weisman
//  Released under the terms of the MIT license. See `LICENSE` in the root
//  directory of this repository for more information.
//
//! Initial 32-bit bootloader for x86_64
#![crate_name = "boot"]
#![crate_type = "staticlib"]
#![feature(asm)]
#![feature(lang_items)]
#![feature(naked_functions)]
#![feature(static_recursion)]
#![feature(linkage)]
#![feature(struct_field_attributes)]
#![feature(stmt_expr_attributes)]
#![no_std]

const TABLE_LENGTH: usize = 512;

type Table = [u64; TABLE_LENGTH];

use core::ptr;
//
// #[lang = "panic_fmt"]
// #[cold]
// unsafe extern fn panic_fmt() -> ! {
//     boot_write(b"boot panic occurred!");
//     die();
// }


#[repr(C, packed)]
pub struct Gdt { _null: u64
               , code: u64
               , data: u64
               , _pad: u16
               , pub ptr: GdtPointer
        //    , ptr: GdtPointer
           }

#[repr(C, packed)]
pub struct GdtPointer { /// the length of the descriptor table
                     pub limit: u16
                   , /// pointer to the region in memory
                     /// containing the descriptor table.
                     pub base: &'static Gdt
                   }

// TODO: this sucks please fix
#[link_name = ".gdt64"]
#[link_section = ".gdt"]
#[no_mangle]
pub static GDT: Gdt
    = Gdt { _null: 0
          , #[export_name = "gdt64.code"] #[no_mangle]
            code: (1<<44) | (1<<47) | (1<<41) | (1<<43) | (1<<53)
          , data: (1 << 44) | (1 << 47) | (1 << 41)
          , _pad: 0
          , ptr:
                #[no_mangle]
                GdtPointer { limit: 23
                            , base: &GDT
                            }
          };

// #[no_mangle]
// static mut GDT_PTR: GdtPointer
//     = GdtPointer { limit: 15
//                  , base: &GDT
//                  };

#[inline(always)]
#[naked]
fn boot_write(s: &[u8]) {
	// use core::ptr;
    let mut offset = 0;
    unsafe {
        let vga_buf: *mut u16 = 0xb8000 as *mut u16;
    	for c in s {
    		ptr::write_volatile( vga_buf.offset(offset)
                               , 0x0200 + *c as u16);
    		offset += 1;
    	}

    }

}
macro_rules! write_char {
    ($c:expr) => {
        ptr::write_volatile(0xb8000 as *mut u16, 0x0200 + $c as u16)
    }
}


extern "C" {
    static mut pml4_table: Table;
    static mut pdp_table: Table;
    static mut pd_table: Table;    // static mut page_table: Table;
    fn arch_init(mboot: u64);
    static mut stack_top: *mut u8;
}

// #[naked]
// #[inline(always)]
// unsafe fn set_cs(cs: u16) {
//     asm!("ljmpl $0, $$fake_label
//           fake_label: \n\t"
//         :
//         : "i"(cs)
//         : "ax"
//         : "volatile"
//         );
// }

#[naked]
#[inline(always)]
pub unsafe fn create_page_tables() {
    // 3. if everything is okay, create the page tables and start long mode
    const HUGE_PAGE_SIZE: u64 = 2 * 1024 * 1024; // 2 MiB

    //-- map the PML4 and PDP tables -----------------------------------------
    // recursive map last PML4 entry
    pml4_table[511] = (&pml4_table[0] as *const _ as u64) | 3;
    // map first PML4 entry to PDP table
    pml4_table[0] = (&pdp_table[0] as *const _ as u64) | 3;
    // map first PDPT entry to PD table
    pdp_table[0] = (&pd_table[0] as *const _ as u64) | 3;

    boot_write(b"3.1");

    //-- map the PD table ----------------------------------------------------
    for (number, entry) in pd_table.iter_mut().enumerate() {
        // set each PD entry equal to the start address of the page (the page
        // number times the page's size)
        let addr = number as u64 * HUGE_PAGE_SIZE;
        // with the appropriate flags (present + writable + huge)
        // TODO: do we want to do this using bitflags, or is that too
        //       heavyweight for the boot module?
        //          - eliza, 1/23/2017
        *entry = addr | 0b10000011;
    }
}

#[naked]
pub unsafe fn set_long_mode() {
    // load PML4 addr to cr3
    asm!( "mov   cr3, $0" :: "r"(&pml4_table) :: "intel");
    boot_write(b"3.2");

    // enable PAE flag in cr4
    let mut cr4: usize;
    asm!("mov $0, cr4" : "=r"(cr4) ::: "intel");
    cr4 |= 1 << 5;
    asm!("mov cr4, $0" :: "r"(cr4) :: "intel");
    boot_write(b"3.3");

    // set the long mode bit in EFER MSR (model specific register)
    asm!( "mov   ecx, 0xC0000080
           rdmsr
           or    eax, 1 << 8
           wrmsr"
        :::: "intel");
    boot_write(b"3.4");

    // enable paging in cr0
    let mut cr0: usize;
    asm!( "mov $0, cr0" : "=r"(cr0) ::: "intel");
    cr0 |= 1 << 31;
    cr0 |= 1 << 16;
    asm!( "mov cr0, $0" :: "r"(cr0) ::: "intel");
    boot_write(b"3.5");
}


#[cold]
#[no_mangle]
#[naked]
pub unsafe extern "C" fn _start() {
    // boot_write(b"0");
    asm!("cli");

    // 1. Move Multiboot info pointer to edi
    // let multiboot: usize;
    asm!("mov edi, edx"
         : //"=r"(multiboot)
         ::: "intel"
    );
    // boot_write(b"1");

    // 2. make sure the system supports SOS
    // TODO: port this from boot.asm
    // boot_write(b"2");

    create_page_tables();
    set_long_mode();


    // 4. load the 64-bit GDT
    asm!(  "lgdt ($0)"
        :: "r"(&GDT.ptr)
        :  "memory"
        );
    // boot_write(b"4");

    // 5. update selectors
    asm!("mov ax, 16" :::: "intel");
    // boot_write(b"5.1");
    // stack selector
    asm!("mov ss, ax" :::: "intel");
    // boot_write(b"5.2");
    // data selector
    asm!("mov ds, ax" :::: "intel");
    // boot_write(b"5.3");
    // extra selector
    asm!("mov es, ax" :::: "intel");
    // boot_write(b"5.4");

    // 6. jump to the 64-bit boot subroutine.
    // asm!("jmp $0:$1" :: "X"(&GDT.descriptors[1]), "X"(arch_init as unsafe extern "C" fn() -> !) :: "intel");
    // asm!("jmp $0:arch_init" :: "r"(&GDT.descriptors[1] as *const u64) :: "intel");
    // arch_init();
    // asm!(  "jmp $0:arch_init"
    //     :: "r"(&GDT.code as *const _ as usize -
    //            &GDT as *const _ as usize)
    //     :: "intel");
    // asm!( "ljmp $0:$1" :: "X"(8), "X"("arch_init"):: "volatile", "intel");


    // asm!("ljmpw $$0x0008, $$arch_init"
    //     :: // "i"(arch_init)
    //     :: "volatile");
    asm!(  "ljmpl $$8, $$arch_init"
        :: //"I"((&GDT.code as *const _ as usize) - &GDT as *const _ as usize)
        :: "volatile"
       );
    // asm!("jmp far 0x0008:arch_init" :::: "intel", "volatile");
    // set_cs(0x0008);
    // loop { boot_write(b"hey i set the code segment selector"); }
    // asm!("calll $$arch_init");
    // arch_init(multiboot as u64);
    // asm!("ljmpl $$0x0008, $$0x106000");
    // asm!("ljmpl *(arch_init)");
    // asm!("ljmpl [arch_init]" );

}
