//
//  SOS: the Stupid Operating System
//  by Eliza Weisman (hi@hawkweisman.me)
//
//  Copyright (c) 2015-2017 Eliza Weisman
//  Released under the terms of the MIT license. See `LICENSE` in the root
//  directory of this repository for more information.
//
use super::{ElfResult, ElfWord, Section, section};

use core::{fmt, mem};

pub trait Header {
    type Word: ElfWord;

    /// Attempt to extract an ELF file header from a slice of bytes.
    /// TODO: should this be a From impl maybe?
    //          - eliza, 03/08/2017
    fn from_slice<'a>(input: &'a [u8]) -> ElfResult<&'a Self>;

    /// Attempt to extract a section header from a slice of bytes.
    /// TODO: can/should the index be `usize`?
    //          - eliza, 03/08/2017
    fn parse_section<'a>(&'a self, input: &'a [u8], idx: u16)
                            -> ElfResult<&'a Section>;

    // Field accessors -------------------------------------------
    fn ident(&self) -> Ident;
    fn get_type(&self) -> Type;
    fn machine(&self) -> Machine;
    /// Offset of the program entry point
    fn entry_point(&self) -> usize;
    /// Offset of the start of program headers
    fn ph_offset(&self) -> usize;
    /// Offset of the start of [section header]s.
    ///
    /// [section header]: ../section/struct.Header.html
    fn sh_offset(&self) -> usize;
    /// TODO: can this return the flags type?
    //          - eliza, 03/08/2017
    fn flags(&self) -> u32;
    fn sh_str_idx(&self) -> usize;

}

/// An ELF file header
#[derive(Copy, Clone, Debug)]
#[repr(C, packed)]
pub struct HeaderRepr<W: ElfWord> {
    /// the ELF [file identifier](struct.Ident.html)
    pub ident: Ident
  , elftype: TypeRepr
  , pub machine: Machine
  , /// Program entry point
    /// TODO: getters for turning these into `usize`?
    //          - eliza, 03/08/2017
    entry_point: W
  , /// Offset for start of program headers
    ph_offset: W
  , /// Offset for start of [section header]s.
    /// [section header]: ../section/struct.Header.html
    sh_offset: W
  , pub flags: u32
  , pub header_size: u16
  , pub ph_entry_size: u16
  , pub ph_count: u16
  , pub sh_entry_size: u16
  , pub sh_count: u16
  , /// Index of the section header string table
    sh_str_idx: u16
}

/// FIXME(style): generate more stuff with macros/use `macro_attr` to derive
///               these...
//                  - eliza, 03/08/2017
macro_rules! impl_getters {
    ($(#[$attr:meta])* pub fn $name:ident(&self) -> $ty:ident; $($rest:tt)*) => {
        $(#[$attr])* #[inline] pub fn $name(&self) -> $ty { self.$name as $ty }
        impl_getters!{ $( $rest )* }
    };
    ($(#[$attr:meta])* fn $name:ident(&self) -> $ty:ident; $($rest:tt)*) => {
        $(#[$attr])* #[inline] fn $name(&self) -> $ty { self.$name as $ty }
        impl_getters!{ $( $rest )* }
    };
    ( $(#[$attr:meta])* pub fn $name: ident (&self)-> $ty: ident; ) => {
        $(#[$attr])* #[inline] pub fn $name(&self) -> $ty { self.$name as $ty }
    };
    ( $(#[$attr:meta])* fn $name: ident (&self)-> $ty: ident; ) => {
        $(#[$attr])* #[inline] fn $name(&self) -> $ty { self.$name as $ty }
    };
    () => {};
}


impl Header for HeaderRepr<u64> {
    type Word = u64;

    /// Attempt to extract an ELF file header from a slice of bytes.
    /// TODO: can this also be macro-generated?
    ///         - eliza, 03/08/2017
    fn from_slice<'a>(input: &'a [u8]) -> ElfResult<&'a Self> {
        if input.len() < mem::size_of::<Self>() {
            Err("Input too short to extract ELF header")
        } else {
            unsafe { Ok(&super::extract_from_slice::<Self>(input, 0, 1)[0]) }
        }
    }

    /// Attempt to extract a section header from a slice of bytes.
    /// TODO: can this also be macro-generated?
    ///         - eliza, 03/08/2017
    fn parse_section<'a>(&'a self, input: &'a [u8], idx: u16)
                            -> ElfResult<&'a Section>
    {
        if idx < section::SHN_LORESERVE {
            Err("Cannot parse reserved section.")
        } else {
            let start: u64// start offset for section
                = self.sh_offset + idx as u64 * self.sh_entry_size as u64;
            let end: u64 // end offset for section
                = start + self.sh_entry_size as u64;
            let raw
                = &input[start as usize .. end as usize];

            match self.ident.class {
                Class::None => Err("Invalid ELF class (ELFCLASSNONE).")
              , Class::Elf32 => Err("Cannot parse 32-bit section from 64-bit \
                                     ELF file.")
              , Class::Elf64 => unsafe {
                    Ok(&*(raw as *const [u8] as *const u8 as *const Section))
                }
            }
        }
    }

    #[inline] fn get_type(&self) -> Type { self.elftype.as_type() }

    impl_getters! {
        #[doc = "Index for the start of [section header]s. \
                 [section header]: ../section/struct.Header.html"]
        fn sh_offset(&self) -> usize;
        #[doc = "Index for the start of program headers"]
        fn ph_offset(&self) -> usize;
        #[doc = "Index for the program entry point"]
        fn entry_point(&self) -> usize;
        #[doc = "Index of the section header [string table] \
                 [string table]: ../section/struct.StrTable.html"]
        fn sh_str_idx(&self) -> usize;
        fn flags(&self) -> u32;
        fn ident(&self) -> Ident;
        fn machine(&self) -> Machine;
    }

}

impl Header for HeaderRepr<u32> {

    type Word = u32;
    /// Attempt to extract an ELF file header from a slice of bytes.
    fn from_slice<'a>(input: &'a [u8]) -> ElfResult<&'a Self> {
        if input.len() < mem::size_of::<Self>() {
            Err("Input too short to extract ELF header")
        } else {
            unsafe { Ok(&super::extract_from_slice::<Self>(input, 0, 1)[0]) }
        }
    }

    /// Attempt to extract a section header from a slice of bytes.
    fn parse_section<'a>(&'a self, input: &'a [u8], idx: u16)
                            -> ElfResult<&'a Section>
    {
        if idx < section::SHN_LORESERVE {
            Err("Cannot parse reserved section.")
        } else {
            let start: u32// start offset for section
                = self.sh_offset + idx as u32 * self.sh_entry_size as u32;
            let end: u32 // end offset for section
                = start + self.sh_entry_size as u32;
            let raw
                = &input[start as usize .. end as usize];

            match self.ident.class {
                Class::None => Err("Invalid ELF class (ELFCLASSNONE).")
              , Class::Elf32 => unsafe {
                    Ok(&*(raw as *const [u8] as *const u8 as *const Section))
                }
              , Class::Elf64 => Err("Cannot parse 64-bit section from 32-bit \
                                     ELF file.")
            }
        }
    }

    #[inline] fn get_type(&self) -> Type { self.elftype.as_type() }

    impl_getters! {
        #[doc = "Index for the start of [section header]s. \
                 [section header]: ../section/struct.Header.html"]
        fn sh_offset(&self) -> usize;
        #[doc = "Index for the start of program headers"]
        fn ph_offset(&self) -> usize;
        #[doc = "Index for the program entry point"]
        fn entry_point(&self) -> usize;
        #[doc = "Index of the section header [string table] \
                 [string table]: ../section/struct.StrTable.html"]
        fn sh_str_idx(&self) -> usize;
        fn flags(&self) -> u32;
        fn ident(&self) -> Ident;
        fn machine(&self) -> Machine;
    }

}

/// ELF header magic
pub const MAGIC: Magic = [0x7f, b'E', b'L', b'F'];

/// Type of header magic
pub type Magic = [u8; 4];


/// ELF identifier (`e_ident` in the ELF standard)
#[derive(Copy, Clone, Debug)]
#[repr(C, packed)]
pub struct Ident {
    /// ELF magic numbers. Must be equal to the [ELF magic], `[0x7, E, L, F]`.
    ///
    /// [ELF magic]: constant.MAGIC.html
    pub magic: Magic
  , /// ELF [file class](enum.Class.html) (32- or 64-bit)
    pub class: Class
  , /// ELF [data encoding](enum.DataEncoding.html) (big- or little-endian)
    pub encoding: DataEncoding
  , /// ELF file [version](enum.Version.html)
    pub version: Version
  , /// What [operating system ABI] this file was compiled for.
    ///
    /// [operating system ABI]: enum.OsAbi.html
    pub abi: OsAbi
  , /// ABI version (often this is just padding)
    pub abi_version: u8
  , _padding: [u8; 7]
}

impl Ident {
    #[inline] pub fn check_magic(&self) -> bool { self.magic == MAGIC }

    /// Returns true if the identifier section identifies a valid ELF file.
    #[inline] pub fn is_valid(&self) -> bool {
        // the ELF magic number is correct
        self.check_magic() &&
        // the file class is either 32- or 64-bits
        self.class.is_valid() &&
        // the data encoding is either big- or little-endian
        self.encoding.is_valid()
    }
}

/// Identifies the class of the ELF file
#[derive(Copy, Clone, PartialEq, Debug)]
#[repr(u8)]
pub enum Class {
    /// Invalid ELF class file (`ELFCLASSNONE` in the standard)
    None  = 0
  , /// 32-bit ELF file (`ELFCLASS32` in the standard)
    Elf32 = 1
  , /// 64-bit ELF file (`ELFCLASS64` in the standard)
    Elf64 = 2
}

impl Class {
    /// Returns true if the class field for this file is valid.
    #[inline]
    pub fn is_valid(&self) -> bool {
        match *self { Class::None => false
                    , _ => true
                    }
    }
}

/// Identifies the data encoding of the ELF file
#[derive(Copy, Clone, PartialEq, Debug)]
#[repr(u8)]
pub enum DataEncoding {
    /// Invalid data encoding (`ELFDATANONE` in the standard)
    None  = 0
  , /// Twos-complement little-endian data encoding
    /// (`ELFDATA2LSB` in the standard)
    LittleEndian = 1
  , /// Twos-complement big-endian data encoding
    /// (`ELFDATA2MSB` in the standard)
    BigEndian = 2
}

impl DataEncoding {
    /// Returns true if the data encoding field for this file is valid.
    #[inline]
    pub fn is_valid(&self) -> bool {
        match *self { DataEncoding::None => false
                    , _ => true
                    }
    }
}

/// Operating system ABI
#[derive(Copy, Clone, Debug)]
#[repr(u8)]
pub enum OsAbi { /// Ox00 also represents "none"
                 SystemV = 0x00
               , HpUx    = 0x01
               , NetBsd  = 0x02
               , Linux   = 0x03
               , Solaris = 0x06
               , Aix     = 0x07
               , Irix    = 0x08
               , FreeBsd = 0x09
               , OpenBsd = 0x0C
               , OpenVms = 0x0D
               }

/// Identifies the version of the ELF file
#[derive(Copy, Clone, PartialEq, Debug)]
#[repr(u8)]
pub enum Version { None = 0, Current = 1 }

#[derive(Clone, Copy, PartialEq)]
struct TypeRepr(u16);

impl TypeRepr {
    pub fn as_type(&self) -> Type {
        match self.0 {
            0 => Type::None
          , 1 => Type::Relocatable
          , 2 => Type::Executable
          , 3 => Type::SharedObject
          , 4 => Type::Core
          , anything => Type::Other(anything)
        }
    }
}

impl fmt::Debug for TypeRepr {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.as_type().fmt(f)
    }
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Type { None
              , Relocatable
              , Executable
              , SharedObject
              , Core
              , Other(u16)
              }

#[allow(non_camel_case_types)]
#[derive(Clone, Copy, PartialEq, Debug)]
#[repr(u16)]
pub enum Machine { None    = 0
                 , Sparc   = 0x02
                 , X86     = 0x03
                 , Mips    = 0x08
                 , PowerPc = 0x14
                 , Arm     = 0x28
                 , SuperH  = 0x2A
                 , Ia64    = 0x32
                 , X86_64  = 0x3E
                 , AArch64 = 0xB7
                 }
