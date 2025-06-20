use super::id::RecycleAllocator;
use super::manager::insert_into_pid2process;
use super::rusage::Rusage;
use super::{add_task, current_task, SignalFlags};
use super::{pid_alloc, PidHandle};
use super::{SignalActions, TaskControlBlock};
use crate::config::{MMAP_BASE, PAGE_SIZE};
use crate::fs::{FdTable, FileDescriptor, OpenFlags, ROOT_FD};
use crate::fs::{File, Stdin, Stdout};
use crate::mm::{
    kernel_token, translated_refmut, AuxHeader, MemorySet, PageTable, VirtAddr, KERNEL_SPACE,
};
use crate::sync::{Condvar, Futex, Mutex, Semaphore, UPSafeCell};
use crate::syscall::errno::{EPERM, SUCCESS};
use crate::timer::Times;
use crate::trap::{trap_handler, TrapContext};
use alloc::string::String;
use alloc::sync::{Arc, Weak};
use alloc::vec;
use alloc::vec::Vec;
use core::cell::RefMut;
use spin::Mutex as MutexSpin;

pub struct ProcessControlBlock {
    // immutable
    pub pid: PidHandle,
    // mutable
    inner: UPSafeCell<ProcessControlBlockInner>,
}

#[derive(Clone)]
pub struct FsStatus {
    pub working_inode: Arc<FileDescriptor>,
}

pub struct ProcessControlBlockInner {
    pub is_zombie: bool,
    pub memory_set: MemorySet,
    pub parent: Option<Weak<ProcessControlBlock>>,
    pub children: Vec<Arc<ProcessControlBlock>>,
    pub exit_code: i32,
    pub fd_table: Arc<MutexSpin<FdTable>>,
    pub work_path: Arc<MutexSpin<FsStatus>>,
    pub signals_pending: SignalFlags,
    // the signal to mask
    pub signal_mask: SignalFlags,
    pub tasks: Vec<Option<Arc<TaskControlBlock>>>,
    pub task_res_allocator: RecycleAllocator,
    pub mutex_list: Vec<Option<Arc<dyn Mutex>>>,
    pub semaphore_list: Vec<Option<Arc<Semaphore>>>,
    pub condvar_list: Vec<Option<Arc<Condvar>>>,
    // Record the usage of heap_area in MemorySet
    pub heap_base: VirtAddr,
    pub heap_end: VirtAddr,
    // Signal actions
    pub signal_actions: SignalActions,
    // for times syscall
    pub tms: Times,
    pub exit_signal: SignalFlags,
    pub futex: Futex,
    pub self_exe: String,
    pub rusage: Rusage,
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

impl ProcessControlBlockInner {
    #[allow(unused)]
    pub fn get_user_token(&self) -> usize {
        self.memory_set.token()
    }

    // pub fn alloc_fd(&mut self) -> usize {
    //     if let Some(fd) = (0..self.fd_table.len()).find(|fd| self.fd_table[*fd].is_none()) {
    //         fd
    //     } else {
    //         self.fd_table.push(None);
    //         self.fd_table.len() - 1
    //     }
    // }

    // pub fn align_up(addr: usize) -> usize {
    //     ((addr) + PAGE_SIZE - 1) & (!(PAGE_SIZE - 1))
    // }

    pub fn mmap(
        &mut self,
        start_addr: usize,
        len: usize,
        prot: u32,
        flags: u32,
        fd: usize,
        offset: usize,
    ) -> isize {
        let flags = Flags::from_bits(flags).unwrap();
        let (context, length) = if flags.contains(Flags::MAP_ANONYMOUS) {
            (Vec::new(), len)
        } else {
            let file_descriptor = match self.fd_table.lock().get_ref(fd) {
                Ok(file_descriptor) => file_descriptor.clone(),
                Err(errno) => return errno,
            };
            let context = file_descriptor.read_all();
            let file_len = context.len();
            let length = len.min(file_len - offset);
            if file_len <= offset {
                return EPERM;
            };
            (context, length)
        };

        self.memory_set
            .mmap(start_addr, length, offset, context, flags)
    }

    pub fn munmap(&mut self, start_addr: usize, len: usize) -> isize {
        self.memory_set.munmap(start_addr, len)
    }

    pub fn alloc_tid(&mut self) -> usize {
        self.task_res_allocator.alloc()
    }

    pub fn dealloc_tid(&mut self, tid: usize) {
        self.task_res_allocator.dealloc(tid)
    }

    pub fn thread_count(&self) -> usize {
        self.tasks.len()
    }

    pub fn get_task(&self, tid: usize) -> Arc<TaskControlBlock> {
        self.tasks[tid].as_ref().unwrap().clone()
    }
}

pub const CSIGNAL: usize = 0x000000ff; /* signal mask to be sent at exit */
bitflags! {
    pub struct CloneFlags: u32 {
        const CLONE_VM	            = 0x00000100;/* set if VM shared between processes */
        const CLONE_FS	            = 0x00000200;/* set if fs info shared between processes */
        const CLONE_FILES	        = 0x00000400;/* set if open files shared between processes */
        const CLONE_SIGHAND	        = 0x00000800;/* set if signal handlers and blocked signals shared */
        const CLONE_PIDFD	        = 0x00001000;/* set if a pidfd should be placed in parent */
        const CLONE_PTRACE	        = 0x00002000;/* set if we want to let tracing continue on the child too */
        const CLONE_VFORK	        = 0x00004000;/* set if the parent wants the child to wake it up on mm_release */
        const CLONE_PARENT	        = 0x00008000;/* set if we want to have the same parent as the cloner */
        const CLONE_THREAD	        = 0x00010000;/* Same thread group? */
        const CLONE_NEWNS	        = 0x00020000;/* New mount namespace group */
        const CLONE_SYSVSEM	        = 0x00040000;/* share system V SEM_UNDO semantics */
        const CLONE_SETTLS	        = 0x00080000;/* create a new TLS for the child */
        const CLONE_PARENT_SETTID	= 0x00100000;/* set the TID in the parent */
        const CLONE_CHILD_CLEARTID	= 0x00200000;/* clear the TID in the child */
        const CLONE_DETACHED		= 0x00400000;/* Unused, ignored */
        const CLONE_UNTRACED		= 0x00800000;/* set if the tracing process can't force CLONE_PTRACE on this clone */
        const CLONE_CHILD_SETTID	= 0x01000000;/* set the TID in the child */
        const CLONE_NEWCGROUP		= 0x02000000;/* New cgroup namespace */
        const CLONE_NEWUTS		    = 0x04000000;/* New utsname namespace */
        const CLONE_NEWIPC		    = 0x08000000;/* New ipc namespace */
        const CLONE_NEWUSER		    = 0x10000000;/* New user namespace */
        const CLONE_NEWPID		    = 0x20000000;/* New pid namespace */
        const CLONE_NEWNET		    = 0x40000000;/* New network namespace */
        const CLONE_IO		        = 0x80000000;/* Clone io context */
    }
}

impl ProcessControlBlock {
    pub fn inner_exclusive_access(&self) -> RefMut<'_, ProcessControlBlockInner> {
        self.inner.exclusive_access()
    }

    // only for initproc
    pub fn new(elf_fd: FileDescriptor) -> Arc<Self> {
        // memory_set with elf program headers/trampoline/trap context/user stack
        let elf_data = elf_fd.map_to_kernel_space(MMAP_BASE);
        // let elf_data = &elf_fd.read_all();
        let (memory_set, uheap_base, ustack_top, entry_point, auxv, _) =
            MemorySet::from_elf(elf_data);
        crate::mm::KERNEL_SPACE
            .exclusive_access()
            .remove_area_with_start_vpn(VirtAddr::from(MMAP_BASE).floor());
        // allocate a pid
        let pid_handle = pid_alloc();
        let process = Arc::new(Self {
            pid: pid_handle,
            inner: unsafe {
                UPSafeCell::new(ProcessControlBlockInner {
                    is_zombie: false,
                    memory_set,
                    parent: None,
                    children: Vec::new(),
                    exit_code: 0,
                    // initialize the stdio, Use stdout to implement stderr
                    fd_table: Arc::new(MutexSpin::new(FdTable::new({
                        let mut vec = Vec::with_capacity(144);
                        let stdin = Some(FileDescriptor::new(false, false, Arc::new(Stdin)));
                        let stdout = Some(FileDescriptor::new(false, false, Arc::new(Stdout)));
                        let stderr = Some(FileDescriptor::new(false, false, Arc::new(Stdout)));
                        vec.push(stdin);
                        vec.push(stdout);
                        vec.push(stderr);
                        vec
                    }))),
                    work_path: Arc::new(MutexSpin::new(FsStatus {
                        working_inode: Arc::new(
                            ROOT_FD
                                .open(".", OpenFlags::O_RDONLY | OpenFlags::O_DIRECTORY, true)
                                .unwrap(),
                        ),
                    })),
                    signals_pending: SignalFlags::empty(),
                    signal_mask: SignalFlags::empty(),
                    tasks: Vec::new(),
                    task_res_allocator: RecycleAllocator::new(),
                    mutex_list: Vec::new(),
                    semaphore_list: Vec::new(),
                    condvar_list: Vec::new(),
                    heap_base: uheap_base.into(),
                    heap_end: uheap_base.into(),
                    signal_actions: SignalActions::default(),
                    tms: Times::new(),
                    exit_signal: SignalFlags::empty(),
                    futex: Futex::new(),
                    self_exe: String::new(),
                    rusage: Rusage::new(),
                })
            },
        });
        // create a main thread, we should allocate ustack and trap_cx here
        let task = Arc::new(TaskControlBlock::new(
            Arc::clone(&process),
            ustack_top,
            true,
            true,
        ));
        // prepare trap_cx of main thread
        let task_inner = task.inner_exclusive_access();
        let trap_cx = task_inner.get_trap_cx();
        // let ustack_top = task_inner.res.as_ref().unwrap().ustack_top();
        let kstack_top = task.kstack.get_top();
        drop(task_inner);
        *trap_cx = TrapContext::app_init_context(
            entry_point,
            ustack_top,
            KERNEL_SPACE.exclusive_access().token(),
            kstack_top,
            trap_handler as usize,
        );
        // add main thread to the process
        let mut process_inner = process.inner_exclusive_access();
        process_inner.tasks.push(Some(Arc::clone(&task)));
        drop(process_inner);
        insert_into_pid2process(process.getpid(), Arc::clone(&process));
        // add main thread to scheduler
        add_task(task);
        process
    }

    /// Only support processes with a single thread.
    pub fn exec(
        self: &Arc<Self>,
        file: FileDescriptor,
        argv_vec: Vec<String>,
        envp_vec: Vec<String>,
    ) {
        // elf_data: &[u8]
        assert_eq!(self.inner_exclusive_access().thread_count(), 1);
        // release memory before loading the application
        self.inner_exclusive_access().memory_set = MemorySet::new_bare();
        // memory_set with elf program headers/trampoline/trap context/user stack
        // let elf_data = &file.read_all();
        let elf_data = file.map_to_kernel_space(MMAP_BASE);
        let (memory_set, uheap_base, ustack_top, entry_point, mut auxv, interp_entry) =
            MemorySet::from_elf(elf_data);
        crate::mm::KERNEL_SPACE
            .exclusive_access()
            .remove_area_with_start_vpn(VirtAddr::from(MMAP_BASE).floor());
        let new_token = memory_set.token();
        // substitute memory_set
        self.inner_exclusive_access().memory_set = memory_set;
        // heap position
        self.inner_exclusive_access().heap_base = uheap_base.into();
        self.inner_exclusive_access().heap_end = uheap_base.into();
        // then we alloc user resource for main thread again
        // since memory_set has been changed
        let task = self.inner_exclusive_access().get_task(0);
        let mut task_inner = task.inner_exclusive_access();
        task_inner.res.as_mut().unwrap().ustack_top = ustack_top;
        // println!("[exec] alloc user res at ustack_top :{:#x}", ustack_top);
        task_inner.res.as_mut().unwrap().alloc_user_res(true);
        task_inner.trap_cx_ppn = task_inner.res.as_mut().unwrap().trap_cx_ppn();
        // push arguments on user stack
        // let mut user_sp = ustack_top;
        let (user_sp, argc, argv_base, envp_base, aux_base) = self
            .inner_exclusive_access()
            .memory_set
            .build_stack(ustack_top, argv_vec, envp_vec, auxv);
        // initialize trap_cx
        // println!("[exec] user_sp : {:#x}", user_sp);
        let mut trap_cx = TrapContext::app_init_context(
            if let Some(interp_entry) = interp_entry {
                interp_entry
            } else {
                entry_point
            },
            user_sp,
            KERNEL_SPACE.exclusive_access().token(),
            task.kstack.get_top(),
            trap_handler as usize,
        );
        // tip!(
        //     "[exec] argv_base: {:#x}, envp_base: {:#x}, aux_base: {:#x}, entry_point: {:#x}",
        //     argv_base,
        //     envp_base,
        //     aux_base,
        //     entry_point,
        // );
        trap_cx.x[10] = argc; //argc
        trap_cx.x[11] = argv_base; //argv
        trap_cx.x[12] = envp_base; //envp
        trap_cx.x[13] = aux_base; //auxv
        *task_inner.get_trap_cx() = trap_cx;
    }

    /// Only support processes with a single thread.
    pub fn fork(self: &Arc<Self>) -> usize {
        let mut parent = self.inner_exclusive_access();
        assert_eq!(parent.thread_count(), 1);
        // clone parent's memory_set completely including trampoline/ustacks/trap_cxs
        let memory_set = MemorySet::from_existed_user(&parent.memory_set);
        let signals_pending = parent.signals_pending;
        // alloc a pid
        let pid = pid_alloc();
        // copy fd table
        let mut new_fd_table_inner: Vec<Option<FileDescriptor>> = Vec::new();
        // we should to push None to guarantee the right file id for file_descriptor
        for fd in parent.fd_table.lock().iter() {
            if let Some(file) = fd {
                new_fd_table_inner.push(Some(file.clone()));
            } else {
                new_fd_table_inner.push(None);
            }
        }
        let new_fd_table = Arc::new(MutexSpin::new(FdTable::new(new_fd_table_inner)));
        // create child process pcb
        let child = Arc::new(Self {
            pid,
            inner: unsafe {
                UPSafeCell::new(ProcessControlBlockInner {
                    is_zombie: false,
                    memory_set,
                    parent: Some(Arc::downgrade(self)),
                    children: Vec::new(),
                    exit_code: 0,
                    fd_table: new_fd_table,
                    work_path: Arc::new(MutexSpin::new(FsStatus {
                        working_inode: Arc::new(
                            ROOT_FD
                                .open(".", OpenFlags::O_RDONLY | OpenFlags::O_DIRECTORY, true)
                                .unwrap(),
                        ),
                    })),
                    signals_pending,
                    signal_mask: SignalFlags::empty(),
                    tasks: Vec::new(),
                    task_res_allocator: RecycleAllocator::new(),
                    mutex_list: Vec::new(),
                    semaphore_list: Vec::new(),
                    condvar_list: Vec::new(),
                    heap_base: parent.heap_base,
                    heap_end: parent.heap_base,
                    signal_actions: SignalActions::default(),
                    tms: Times::new(),
                    exit_signal: SignalFlags::SIGCHLD,
                    futex: Futex::new(),
                    self_exe: parent.self_exe.clone(),
                    rusage: Rusage::new(),
                })
            },
        });
        // add child
        parent.children.push(Arc::clone(&child));
        // create main thread of child process
        let task = Arc::new(TaskControlBlock::new(
            Arc::clone(&child),
            parent
                .get_task(0)
                .inner_exclusive_access()
                .res
                .as_ref()
                .unwrap()
                .ustack_top(),
            // here we do not allocate trap_cx or ustack again
            // but mention that we allocate a new kstack here
            false,
            false,
        ));
        // attach task to child process
        let mut child_inner = child.inner_exclusive_access();
        child_inner.tasks.push(Some(Arc::clone(&task)));
        drop(child_inner);
        // modify kstack_top in trap_cx of this thread
        let task_inner = task.inner_exclusive_access();
        let trap_cx = task_inner.get_trap_cx();
        trap_cx.kernel_sp = task.kstack.get_top();
        // we do not have to move to next instruction since we have done it before
        // for child process, fork returns 0
        trap_cx.x[10] = 0;
        drop(task_inner);
        let pid = child.getpid();
        insert_into_pid2process(pid, Arc::clone(&child));
        // add this thread to scheduler
        add_task(task);
        pid
    }

    pub fn clone2(
        self: &Arc<Self>,
        exit_signals: SignalFlags,
        clone_signals: CloneFlags,
        stack_ptr: usize,
        tls: usize,
    ) -> Arc<TaskControlBlock> {
        let task = current_task().unwrap();
        let process = task.process.upgrade().unwrap();
        // create a new thread.
        // We did not alloc for stack space here
        let thread_stack = if stack_ptr != 0 {
            stack_ptr
        } else {
            task.inner_exclusive_access()
                .res
                .as_ref()
                .unwrap()
                .ustack_top
        };
        let new_task = Arc::new(TaskControlBlock::new(
            Arc::clone(&process),
            thread_stack,
            true,
            false,
        ));
        let new_task_inner = new_task.inner_exclusive_access();
        let new_task_res = new_task_inner.res.as_ref().unwrap();
        let new_task_tid = new_task_res.tid;
        let mut process_inner = process.inner_exclusive_access();
        // add new thread to current process
        let tasks = &mut process_inner.tasks;
        while tasks.len() < new_task_tid + 1 {
            tasks.push(None);
        }
        tasks[new_task_tid] = Some(Arc::clone(&new_task));
        let new_task_trap_cx = new_task_inner.get_trap_cx();

        // I don't know if this is correct
        *new_task_trap_cx = *task.inner_exclusive_access().get_trap_cx();

        // for child process, fork returns 0
        new_task_trap_cx.x[10] = 0;
        // set tp reg
        new_task_trap_cx.x[4] = tls;
        // set sp reg
        new_task_trap_cx.set_sp(new_task_res.ustack_top());
        // modify kernel_sp in trap_cx
        new_task_trap_cx.kernel_sp = new_task.kstack.get_top();

        // add new task to scheduler
        add_task(Arc::clone(&new_task));

        drop(new_task_inner);
        new_task
    }

    pub fn getpid(&self) -> usize {
        self.pid.0
    }
}
