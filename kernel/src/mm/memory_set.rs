use super::{frame_alloc, translated_refmut, FrameTracker};
use super::{PTEFlags, PageTable, PageTableEntry};
use super::{PhysAddr, PhysPageNum, VirtAddr, VirtPageNum};
use super::{StepByOne, VPNRange};
use crate::config::{
    AT_BASE, AT_CLKTCK, AT_EGID, AT_ENTRY, AT_EUID, AT_EXECFN, AT_FLAGS, AT_GID, AT_HWCAP,
    AT_NOELF, AT_NULL, AT_PAGESIZE, AT_PHDR, AT_PHENT, AT_PHNUM, AT_PLATFORM, AT_RANDOM, AT_SECURE,
    AT_UID, MEMORY_END, MMAP_BASE, MMIO, PAGE_SIZE, STACK_TOP, TRAMPOLINE,
};
use crate::sync::UPSafeCell;
use crate::syscall::errno::{EPERM, SUCCESS};
use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::sync::Arc;
use alloc::vec;
use alloc::vec::Vec;
use core::arch::asm;
use core::fmt::Display;
use core::fmt::Formatter;
use lazy_static::*;
use riscv::register::satp;

extern "C" {
    fn stext();
    fn etext();
    fn srodata();
    fn erodata();
    fn sdata();
    fn edata();
    fn sbss_with_stack();
    fn ebss();
    fn ekernel();
    fn strampoline();
}

lazy_static! {
    pub static ref KERNEL_SPACE: Arc<UPSafeCell<MemorySet>> =
        Arc::new(unsafe { UPSafeCell::new(MemorySet::new_kernel()) });
}

pub fn kernel_token() -> usize {
    KERNEL_SPACE.exclusive_access().token()
}

pub struct MemorySet {
    page_table: PageTable,
    areas: Vec<MapArea>,
    // heap_area are often modified, so BTreeMap is better
    // the address about heap and mmap are saved in ProcessControlBlockInner
    heap_area: BTreeMap<VirtPageNum, FrameTracker>,
    // The memory area formed by mmap does not need to be modified
    // we can use MapArea in Vec to hold FramTracker
    // we set a fixed address as the start address for mmap_area
    // the virtual memorySet is big enough to use it that doesnt concern address conflicts
    mmap_area: BTreeMap<VirtPageNum, FrameTracker>,
    // mmap_base will never change
    pub mmap_base: VirtAddr,
    // always aligh to PAGE_SIZE
    pub mmap_end: VirtAddr,
}

bitflags! {
    pub struct Flags: u32 {
        const MAP_SHARED = 0x01;
        const MAP_PRIVATE = 0x02;
        const MAP_FIXED = 0x10;
        const MAP_ANONYMOUS = 0x20;
        const MAP_GROWSDOWN = 0x0100;
        const MAP_DENYWRITE = 0x0800;
        const MAP_EXECUTABLE = 0x1000;
        const MAP_LOCKED = 0x2000;
        const MAP_NORESERVE = 0x4000;
        const MAP_POPULATE = 0x8000;
        const MAP_NONBLOCK = 0x10000;
        const MAP_STACK = 0x20000;
        const MAP_HUGETLB = 0x40000;
        const MAP_SYNC = 0x80000;
        const MAP_FIXED_NOREPLACE = 0x100000;
        const MAP_UNINITIALIZED = 0x4000000;
    }
}

impl MemorySet {
    pub fn new_bare() -> Self {
        Self {
            page_table: PageTable::new(),
            areas: Vec::new(),
            heap_area: BTreeMap::new(),
            mmap_area: BTreeMap::new(),
            mmap_base: MMAP_BASE.into(),
            mmap_end: MMAP_BASE.into(),
        }
    }
    pub fn token(&self) -> usize {
        self.page_table.token()
    }
    /// Assume that no conflicts.
    pub fn insert_framed_area(
        &mut self,
        start_va: VirtAddr,
        end_va: VirtAddr,
        permission: MapPermission,
    ) {
        self.push(
            MapArea::new(start_va, end_va, MapType::Framed, permission),
            None,
        );
    }
    pub fn remove_area_with_start_vpn(&mut self, start_vpn: VirtPageNum) {
        if let Some((idx, area)) = self
            .areas
            .iter_mut()
            .enumerate()
            .find(|(_, area)| area.vpn_range.get_start() == start_vpn)
        {
            area.unmap(&mut self.page_table);
            self.areas.remove(idx);
        }
    }
    fn push(&mut self, mut map_area: MapArea, data: Option<&[u8]>) {
        map_area.map(&mut self.page_table);
        if let Some(data) = data {
            map_area.copy_data(&mut self.page_table, data, 0);
        }
        self.areas.push(map_area);
    }

    fn push_with_offset(&mut self, mut map_area: MapArea, offset: usize, data: Option<&[u8]>) {
        map_area.map(&mut self.page_table);
        if let Some(data) = data {
            map_area.copy_data(&mut self.page_table, data, offset)
        }
        self.areas.push(map_area);
    }

    /// Mention that trampoline is not collected by areas.
    fn map_trampoline(&mut self) {
        self.page_table.map(
            VirtAddr::from(TRAMPOLINE).into(),
            PhysAddr::from(strampoline as usize).into(),
            PTEFlags::R | PTEFlags::X,
        );
    }
    /// Without kernel stacks.
    pub fn new_kernel() -> Self {
        let mut memory_set = Self::new_bare();
        // map trampoline
        memory_set.map_trampoline();
        // map kernel sections
        println!(".text [{:#x}, {:#x})", stext as usize, etext as usize);
        println!(".rodata [{:#x}, {:#x})", srodata as usize, erodata as usize);
        println!(".data [{:#x}, {:#x})", sdata as usize, edata as usize);
        println!(
            ".bss [{:#x}, {:#x})",
            sbss_with_stack as usize, ebss as usize
        );
        println!("mapping .text section");
        memory_set.push(
            MapArea::new(
                (stext as usize).into(),
                (etext as usize).into(),
                MapType::Identical,
                MapPermission::R | MapPermission::X,
            ),
            None,
        );
        println!("mapping .rodata section");
        memory_set.push(
            MapArea::new(
                (srodata as usize).into(),
                (erodata as usize).into(),
                MapType::Identical,
                MapPermission::R,
            ),
            None,
        );
        println!("mapping .data section");
        memory_set.push(
            MapArea::new(
                (sdata as usize).into(),
                (edata as usize).into(),
                MapType::Identical,
                MapPermission::R | MapPermission::W,
            ),
            None,
        );
        println!("mapping .bss section");
        memory_set.push(
            MapArea::new(
                (sbss_with_stack as usize).into(),
                (ebss as usize).into(),
                MapType::Identical,
                MapPermission::R | MapPermission::W,
            ),
            None,
        );
        println!("mapping physical memory");
        memory_set.push(
            MapArea::new(
                (ekernel as usize).into(),
                MEMORY_END.into(),
                MapType::Identical,
                MapPermission::R | MapPermission::W,
            ),
            None,
        );
        println!("mapping memory-mapped registers");
        for pair in MMIO {
            memory_set.push(
                MapArea::new(
                    (*pair).0.into(),
                    ((*pair).0 + (*pair).1).into(),
                    MapType::Identical,
                    MapPermission::R | MapPermission::W,
                ),
                None,
            );
        }
        memory_set
    }
    /// Include sections in elf and trampoline,
    /// also returns user_sp_top and entry point.
    pub fn from_elf(elf_data: &[u8]) -> (Self, usize, usize, usize, Vec<AuxHeader>) {
        let mut memory_set = Self::new_bare();
        // map trampoline
        memory_set.map_trampoline();
        // map program headers of elf, with U flag
        let elf = xmas_elf::ElfFile::new(elf_data).unwrap();
        let elf_header = elf.header;
        // auxv
        let mut auxv = vec![
            AuxHeader::new(AT_PHENT, elf_header.pt2.ph_entry_size() as usize),
            AuxHeader::new(AT_PHNUM, elf_header.pt2.ph_count() as usize),
            AuxHeader::new(AT_PAGESIZE, PAGE_SIZE as usize),
            AuxHeader::new(AT_FLAGS, 0),
            AuxHeader::new(AT_ENTRY, elf_header.pt2.entry_point() as usize),
            AuxHeader::new(AT_UID, 0),
            AuxHeader::new(AT_EUID, 0),
            AuxHeader::new(AT_GID, 0),
            AuxHeader::new(AT_EGID, 0),
            AuxHeader::new(AT_PLATFORM, 0),
            AuxHeader::new(AT_HWCAP, 0),
            AuxHeader::new(AT_CLKTCK, 100usize),
            AuxHeader::new(AT_SECURE, 0),
            AuxHeader::new(AT_NOELF, 0x112d),
        ];

        let magic = elf_header.pt1.magic;
        assert_eq!(magic, [0x7f, 0x45, 0x4c, 0x46], "invalid elf!");
        let ph_count = elf_header.pt2.ph_count();
        let mut max_end_vpn = VirtPageNum(0);
        let mut head_va: usize = 0;

        for i in 0..ph_count {
            let ph = elf.program_header(i).unwrap();
            if ph.get_type().unwrap() == xmas_elf::program::Type::Load {
                let start_addr = ph.virtual_addr() as usize;
                let end_addr = (ph.virtual_addr() + ph.mem_size()) as usize;
                let start_va: VirtAddr = start_addr.into();
                let end_va: VirtAddr = end_addr.into();
                // println!("[app_map] .{} [{:#x}, {:#x})", ph, start_addr, end_addr,);
                let offset = start_va.0 - start_va.floor().0 * PAGE_SIZE;
                let mut map_perm = MapPermission::U;
                let ph_flags = ph.flags();
                if head_va == 0 {
                    head_va = start_va.0;
                }
                if ph_flags.is_read() {
                    map_perm |= MapPermission::R;
                }
                if ph_flags.is_write() {
                    map_perm |= MapPermission::W;
                }
                if ph_flags.is_execute() {
                    map_perm |= MapPermission::X;
                }
                let map_area = MapArea::new(start_va, end_va, MapType::Framed, map_perm);
                max_end_vpn = map_area.vpn_range.get_end();

                if offset == 0 {
                    memory_set.push(
                        map_area,
                        Some(
                            &elf.input
                                [ph.offset() as usize..(ph.offset() + ph.file_size()) as usize],
                        ),
                    )
                } else {
                    memory_set.push_with_offset(
                        map_area,
                        offset,
                        Some(
                            &elf.input
                                [ph.offset() as usize..(ph.offset() + ph.file_size()) as usize],
                        ),
                    );
                }
            }
        }
        if elf.find_section_by_name(".interp").is_some() {
            println!("not static");
        }
        auxv.push(AuxHeader::new(AT_BASE, 0));
        auxv.push(AuxHeader::new(
            AT_PHDR,
            head_va + elf_header.pt2.ph_offset() as usize,
        ));
        let max_end_va: VirtAddr = max_end_vpn.into();
        let mut user_heap_base: usize = max_end_va.into();
        user_heap_base += PAGE_SIZE;
        // initial heap area
        // memory_set.heap_area = Some(MapArea::new(
        //     user_heap_base.into(),
        //     STACK_BASE.into(),
        //     MapType::Framed,
        //     MapPermission::R | MapPermission::W | MapPermission::U,
        // ));
        (
            memory_set,
            user_heap_base,
            STACK_TOP,
            elf.header.pt2.entry_point() as usize,
            auxv,
        )
    }
    pub fn from_existed_user(user_space: &MemorySet) -> MemorySet {
        let mut memory_set = Self::new_bare();
        // map trampoline
        memory_set.map_trampoline();
        memory_set.mmap_end = user_space.mmap_end;
        // copy data sections/trap_context/user_stack
        for area in user_space.areas.iter() {
            let new_area = MapArea::from_another(area);
            memory_set.push(new_area, None);
            // copy data from another space
            for vpn in area.vpn_range {
                let src_ppn = user_space.translate(vpn).unwrap().ppn();
                let dst_ppn = memory_set.translate(vpn).unwrap().ppn();
                dst_ppn
                    .get_bytes_array()
                    .copy_from_slice(src_ppn.get_bytes_array());
            }
        }
        memory_set
    }
    pub fn activate(&self) {
        let satp = self.page_table.token();
        unsafe {
            satp::write(satp);
            asm!("sfence.vma");
        }
    }
    pub fn translate(&self, vpn: VirtPageNum) -> Option<PageTableEntry> {
        self.page_table.translate(vpn)
    }
    pub fn recycle_data_pages(&mut self) {
        //*self = Self::new_bare();
        self.areas.clear();
    }
    pub fn map_heap(&mut self, mut current_addr: VirtAddr, aim_addr: VirtAddr) -> isize {
        log!("[map_heap] start_addr = {:#x}, end_addr = {:#x}", current_addr.0, aim_addr.0);
        loop {
            if current_addr.0 >= aim_addr.0 {
                break;
            }
            // We use BTreeMap to save FrameTracker which makes management quite easy
            // alloc a new FrameTracker
            let frame = frame_alloc().unwrap();
            let ppn = frame.ppn;
            let vpn: VirtPageNum = current_addr.floor();
            // log!("[map_heap] map vpn = {:#x}, ppn = {:#x}", vpn.0, ppn.0);
            self.page_table
                .map(vpn, ppn, PTEFlags::U | PTEFlags::R | PTEFlags::W);
            self.heap_area.insert(vpn, frame);
            current_addr = VirtAddr::from(current_addr.0 + PAGE_SIZE);
        }
        SUCCESS
    }

    pub fn mmap(
        &mut self,
        start_addr: usize,
        len: usize,
        offset: usize,
        context: Vec<u8>,
        flags: u32,
    ) -> isize {
        let flags = Flags::from_bits(flags).unwrap();
        let start_addr_align: usize;
        let end_addr_align: usize;
        if flags.contains(Flags::MAP_FIXED) && start_addr != 0 {
            // MAP_FIXED
            // alloc page one by one
            start_addr_align = ((start_addr) + PAGE_SIZE - 1) & (!(PAGE_SIZE - 1));
            end_addr_align = ((start_addr + len) + PAGE_SIZE - 1) & (!(PAGE_SIZE - 1));
        } else {
            start_addr_align = ((self.mmap_end.0) + PAGE_SIZE - 1) & (!(PAGE_SIZE - 1));
            end_addr_align = ((self.mmap_end.0 + len) + PAGE_SIZE - 1) & (!(PAGE_SIZE - 1));
        }
        self.mmap_end = (end_addr_align + PAGE_SIZE).into();
        let vpn_range = VPNRange::new(
            VirtAddr::from(start_addr_align).floor(),
            VirtAddr::from(end_addr_align).floor(),
        );
        if flags.contains(Flags::MAP_FIXED) && start_addr != 0 {
            // alloc memory
            for vpn in vpn_range {
                // let frame = frame_alloc().unwrap();
                match self.mmap_area.get(&vpn) {
                    Some(_) => {
                        println!("Found page");
                    }
                    None => {
                        let frame = frame_alloc().unwrap();
                        let ppn = frame.ppn;
                        self.mmap_area.insert(vpn, frame);
                        self.page_table
                            .map(vpn, ppn, PTEFlags::R | PTEFlags::W | PTEFlags::U);
                    }
                }
            }
        } else {
            // alloc memory
            for vpn in vpn_range {
                let frame = frame_alloc().unwrap();
                let ppn = frame.ppn;
                self.mmap_area.insert(vpn, frame);
                self.page_table
                    .map(vpn, ppn, PTEFlags::R | PTEFlags::W | PTEFlags::U);
            }
        }
        // write context
        let mut start: usize = offset;
        let mut current_vpn = vpn_range.get_start();
        loop {
            let src = &context[start..len.min(start + PAGE_SIZE)];
            let dst = &mut self
                .page_table
                .translate(current_vpn)
                .unwrap()
                .ppn()
                .get_bytes_array()[..src.len()];
            dst.copy_from_slice(src);
            start += PAGE_SIZE;
            if start >= len {
                break;
            }
            current_vpn.step();
        }
        start_addr_align as isize
    }
    pub fn munmap(&mut self, start_addr: usize, len: usize) -> isize {
        let start_addr_align = ((start_addr) + PAGE_SIZE - 1) & (!(PAGE_SIZE - 1));
        let end_addr_align = ((start_addr + len) + PAGE_SIZE - 1) & (!(PAGE_SIZE - 1));
        let vpn_range = VPNRange::new(
            VirtAddr::from(start_addr_align).floor(),
            VirtAddr::from(end_addr_align).floor(),
        );
        for vpn in vpn_range {
            self.mmap_area.remove(&vpn);
        }
        SUCCESS
    }
    /// If MapArea.map_perm is useful. We have to split MapArea.
    pub fn mprotect(&self, start: VirtAddr, end: VirtAddr, new_flags: PTEFlags) -> isize {
        let start_vpn = start.floor();
        let end_vpn = end.ceil();

        // let result: Vec<usize> = self
        //     .areas
        //     .iter()
        //     .enumerate()
        //     .filter(|(_, area)| {
        //         (area.vpn_range.get_start() <= start_vpn && start_vpn < area.vpn_range.get_end())
        //             || (area.vpn_range.get_start() <= end_vpn && end_vpn < area.vpn_range.get_end())
        //             || (start_vpn <= area.vpn_range.get_start()
        //                 && end_vpn >= area.vpn_range.get_end())
        //     })
        //     .map(|(idx, _)| idx)
        //     .collect();
        // for mut idx in result {
        //     let area_start_vpn = self.areas[idx].vpn_range.get_start();
        //     let area_end_vpn = self.areas[idx].vpn_range.get_end();
        // }

        let vpn_range = VPNRange::new(start_vpn, end_vpn);
        for vpn in vpn_range {
            if !self.page_table.set_pte_flags(vpn, new_flags) {
                return EPERM;
            }
        }
        SUCCESS
    }

    pub fn build_stack(
        &self,
        mut user_sp: usize,
        argv_vec: Vec<String>,
        envp_vec: Vec<String>,
        mut auxv_vec: Vec<AuxHeader>,
    ) -> (usize, usize, usize, usize, usize) {
        // The structure of the user stack
        // STACK TOP (low address)
        //      argc
        //      *argv [] (with NULL as the end) 8 bytes each
        //      *envp [] (with NULL as the end) 8 bytes each
        //      auxv[] (with NULL as the end) 16 bytes each: now has PAGESZ(6)
        //      padding (16 bytes-align)
        //      rand bytes: Now set 0x00 ~ 0x0f (not support random) 16bytes
        //      String: platform "RISC-V64"
        //      Argument string(argv[])
        //      Environment String (envp[]): now has SHELL, PWD, LOGNAME, HOME, USER, PATH
        // STACK BOTTOM (high address)

        let push_stack = |parms: Vec<String>, user_sp: &mut usize| {
            //record parm ptr
            let mut ptr_vec: Vec<usize> = (0..=parms.len()).collect();

            //end with null
            ptr_vec[parms.len()] = 0;

            for index in 0..parms.len() {
                *user_sp -= parms[index].len() + 1;
                ptr_vec[index] = *user_sp;
                let mut p = *user_sp;

                //write chars to [user_sp,user_sp + len]
                for c in parms[index].as_bytes() {
                    *translated_refmut(self.token(), p as *mut u8) = *c;
                    p += 1;
                }
                *translated_refmut(self.token(), p as *mut u8) = 0;
            }
            ptr_vec
        };
        // unkonwn use
        // user_sp -= 2 * core::mem::size_of::<usize>();

        //////////////////////// envp[] ////////////////////////////////
        let envp = push_stack(envp_vec, &mut user_sp);
        // make sure aligned to 8b for k210
        user_sp -= user_sp % core::mem::size_of::<usize>();

        ///////////////////// argv[] /////////////////////////////////
        let argc = argv_vec.len();
        let argv = push_stack(argv_vec, &mut user_sp);
        // make the user_sp aligned to 8B for k210 platform
        user_sp -= user_sp % core::mem::size_of::<usize>();

        ///////////////////// platform ///////////////////////////////
        let platform = "RISC-V64";
        user_sp -= platform.len() + 1;
        user_sp -= user_sp % core::mem::size_of::<usize>();
        let mut p = user_sp;
        for &c in platform.as_bytes() {
            *translated_refmut(self.token(), p as *mut u8) = c;
            p += 1;
        }
        *translated_refmut(self.token(), p as *mut u8) = 0;

        ///////////////////// rand bytes ////////////////////////////
        user_sp -= 16;
        auxv_vec.push(AuxHeader::new(AT_RANDOM, user_sp));
        *translated_refmut(self.token(), user_sp as *mut usize) = 0x01020304050607;
        *translated_refmut(
            self.token(),
            (user_sp + core::mem::size_of::<usize>()) as *mut usize,
        ) = 0x08090a0b0c0d0e0f;

        ///////////////////// padding ////////////////////////////////
        user_sp -= user_sp % 16;

        ///////////////////// auxv[] //////////////////////////////////
        auxv_vec.push(AuxHeader::new(AT_EXECFN, argv[0]));
        auxv_vec.push(AuxHeader::new(AT_NULL, 0));
        user_sp -= auxv_vec.len() * core::mem::size_of::<AuxHeader>();
        let aux_base = user_sp;
        let mut addr = aux_base;
        for aux_header in auxv_vec {
            *translated_refmut(self.token(), addr as *mut usize) = aux_header._type;
            *translated_refmut(
                self.token(),
                (addr + core::mem::size_of::<usize>()) as *mut usize,
            ) = aux_header.value;
            addr += core::mem::size_of::<AuxHeader>();
        }

        ///////////////////// *envp[] /////////////////////////////////
        user_sp -= envp.len() * core::mem::size_of::<usize>();
        let envp_base = user_sp;
        let mut ustack_ptr = envp_base;
        for env_ptr in envp {
            *translated_refmut(self.token(), ustack_ptr as *mut usize) = env_ptr;
            ustack_ptr += core::mem::size_of::<usize>();
        }

        ///////////////////// *argv[] ////////////////////////////////
        user_sp -= argv.len() * core::mem::size_of::<usize>();
        let argv_base = user_sp;
        let mut ustack_ptr = argv_base;
        for argv_ptr in argv {
            *translated_refmut(self.token(), ustack_ptr as *mut usize) = argv_ptr;
            ustack_ptr += core::mem::size_of::<usize>();
        }

        ///////////////////// argc ///////////////////////////////////
        user_sp -= core::mem::size_of::<usize>();
        *translated_refmut(self.token(), user_sp as *mut usize) = argc;

        (user_sp, argc, argv_base, envp_base, aux_base)
    }
}

bitflags! {
    pub struct MPROCTECTPROT: u32 {
        /// page can not be accessed
        const PROT_NONE = 0x00;
        /// page can be read
        const PROT_READ = 0x01;
        /// page can be written
        const PROT_WRITE = 0x02;
        /// page can be executed
        const PROT_EXEC = 0x04;
        /// page may be used for atomic ops
        const PROT_SEM	= 0x10;
        /// mprotect flag: extend change to start of growsdown vma
        const PROT_GROWSDOWN = 0x01000000;
        /// mprotect flag: extend change to end of growsup vma
        const PROT_GROWSUP = 0x02000000;
    }
}

impl Into<PTEFlags> for MPROCTECTPROT {
    fn into(self) -> PTEFlags {
        let mut flag = PTEFlags::U;
        if self.contains(MPROCTECTPROT::PROT_READ) {
            flag |= PTEFlags::R;
        }
        if self.contains(MPROCTECTPROT::PROT_WRITE) {
            flag |= PTEFlags::W;
        }
        if self.contains(MPROCTECTPROT::PROT_EXEC) {
            flag |= PTEFlags::X;
        }
        flag
    }
}

pub struct MapArea {
    vpn_range: VPNRange,
    data_frames: BTreeMap<VirtPageNum, FrameTracker>,
    map_type: MapType,
    map_perm: MapPermission,
}

impl MapArea {
    pub fn new(
        start_va: VirtAddr,
        end_va: VirtAddr,
        map_type: MapType,
        map_perm: MapPermission,
    ) -> Self {
        let start_vpn: VirtPageNum = start_va.floor();
        let end_vpn: VirtPageNum = end_va.ceil();
        Self {
            vpn_range: VPNRange::new(start_vpn, end_vpn),
            data_frames: BTreeMap::new(),
            map_type,
            map_perm,
        }
    }
    pub fn from_another(another: &MapArea) -> Self {
        Self {
            vpn_range: VPNRange::new(another.vpn_range.get_start(), another.vpn_range.get_end()),
            data_frames: BTreeMap::new(),
            map_type: another.map_type,
            map_perm: another.map_perm,
        }
    }
    pub fn map_one(&mut self, page_table: &mut PageTable, vpn: VirtPageNum) {
        let ppn: PhysPageNum;
        match self.map_type {
            MapType::Identical => {
                ppn = PhysPageNum(vpn.0);
            }
            MapType::Framed => {
                let frame = frame_alloc().unwrap();
                ppn = frame.ppn;
                self.data_frames.insert(vpn, frame);
            }
        }
        let pte_flags = PTEFlags::from_bits(self.map_perm.bits).unwrap();
        page_table.map(vpn, ppn, pte_flags);
    }
    pub fn unmap_one(&mut self, page_table: &mut PageTable, vpn: VirtPageNum) {
        if self.map_type == MapType::Framed {
            self.data_frames.remove(&vpn);
        }
        page_table.unmap(vpn);
    }
    pub fn map(&mut self, page_table: &mut PageTable) {
        for vpn in self.vpn_range {
            // println!("[memory_set] map vpn = {:#x}", vpn.0);
            self.map_one(page_table, vpn);
        }
    }
    pub fn unmap(&mut self, page_table: &mut PageTable) {
        for vpn in self.vpn_range {
            // println!("[memory_set] unmap vpn = {:#x}", vpn.0);
            self.unmap_one(page_table, vpn);
        }
    }
    /// data: start-aligned but maybe with shorter length
    /// assume that all frames were cleared before
    pub fn copy_data(&mut self, page_table: &mut PageTable, data: &[u8], offset: usize) {
        assert_eq!(self.map_type, MapType::Framed);
        let mut start: usize = 0;
        let mut page_offset = offset;
        let mut current_vpn = self.vpn_range.get_start();
        let len = data.len();
        loop {
            let src = &data[start..len.min(start + PAGE_SIZE - page_offset)];
            let dst = &mut page_table
                .translate(current_vpn)
                .unwrap()
                .ppn()
                .get_bytes_array()[page_offset..(page_offset + src.len())];
            dst.copy_from_slice(src);
            start += PAGE_SIZE - page_offset;
            page_offset = 0;
            if start >= len {
                break;
            }
            current_vpn.step();
        }
    }
}

#[derive(Copy, Clone, PartialEq, Debug)]
pub enum MapType {
    Identical,
    Framed,
}

bitflags! {
    pub struct MapPermission: u8 {
        const R = 1 << 1;
        const W = 1 << 2;
        const X = 1 << 3;
        const U = 1 << 4;
    }
}

#[allow(unused)]
pub fn remap_test() {
    let mut kernel_space = KERNEL_SPACE.exclusive_access();
    let mid_text: VirtAddr = ((stext as usize + etext as usize) / 2).into();
    let mid_rodata: VirtAddr = ((srodata as usize + erodata as usize) / 2).into();
    let mid_data: VirtAddr = ((sdata as usize + edata as usize) / 2).into();
    assert!(!kernel_space
        .page_table
        .translate(mid_text.floor())
        .unwrap()
        .writable(),);
    assert!(!kernel_space
        .page_table
        .translate(mid_rodata.floor())
        .unwrap()
        .writable(),);
    assert!(!kernel_space
        .page_table
        .translate(mid_data.floor())
        .unwrap()
        .executable(),);
    println!("remap_test passed!");
}

pub struct AuxHeader {
    pub _type: usize,
    pub value: usize,
}

impl AuxHeader {
    #[inline]
    pub fn new(_type: usize, value: usize) -> Self {
        Self { _type, value }
    }
}

impl Display for AuxHeader {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        write!(f, "AuxHeader type: {} value: {}", self._type, self.value)
    }
}
