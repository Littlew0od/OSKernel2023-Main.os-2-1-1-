# 记录在编写过程中遇到的bug以及解决方案

## fork时fd_table浅拷贝导致空文件描述符

错误的使用clone浅拷贝文件描述符，导致一个进程释放导致其他进程空文件描述符

**ERROR**

```rust
// os/src/task/process.rs
let new_fd_table = parent.fd_table.clone();
```

**RIGHT**

```rust
// os/src/task/process.rs
let mut new_fd_table_inner: Vec<Option<FileDescriptor>> = Vec::new();
for fd in parent.fd_table.lock().iter() {
    if let Some(file) = fd {
        new_fd_table_inner.push(Some(file.clone()));
    } else {
        new_fd_table_inner.push(None);
    }
}
let new_fd_table = Arc::new(MutexSpin::new(FdTable::new(new_fd_table_inner)));
```

## 获取了错误的时间

调用时间函数错误

**ERROR**

```rust
//os/src/syscall/process.rs
pub fn sys_get_time(time_return: *mut u64) -> isize {
    let token = current_user_token();
    if time_return as usize != 0 {
        *translated_refmut(token, time_return) = get_time() as u64;
        *translated_refmut(token, unsafe { time_return.add(1) }) = 0;
    }
    0
}


```

**RIGHT**

```rust
//os/src/syscall/process.rs
pub fn sys_get_time(time_return: *mut u64) -> isize {
    let token = current_user_token();
    if time_return as usize != 0 {
        *translated_refmut(token, time_return) = get_time_sec() as u64;
        *translated_refmut(token, unsafe { time_return.add(1) }) = get_time_ns() as u64;
    }
    0
}


```

## 文件表长度限制导致复制描述符错误

目标描述符为100，但设置的最大描述符为64

**ERROR**

```rust
//os/src/fs/mod.rs
pub const DEFAULT_FD_LIMIT: usize = 64;
```

**RIGHT**

```rust
//os/src/fs/mod.rs
pub const DEFAULT_FD_LIMIT: usize = 128;
```

## 当前实现的wait为不等待直接退出，导致测试程序并行执行，在错误的进程上执行exec

**ERROR**

```rust
//os/src/syscall/process.rs
pub fn sys_waitpid(pid: isize, exit_code_ptr: *mut i32) -> isize {
    let process = current_process();
    // find a child process

    let mut inner = process.inner_exclusive_access();
    if !inner
        .children
        .iter()
        .any(|p| pid == -1 || pid as usize == p.getpid())
    {
        return -1;
        // ---- release current PCB
    }
    let pair = inner.children.iter().enumerate().find(|(_, p)| {
        // ++++ temporarily access child PCB exclusively
        p.inner_exclusive_access().is_zombie && (pid == -1 || pid as usize == p.getpid())
        // ++++ release child PCB
    });
    if let Some((idx, _)) = pair {
        let child = inner.children.remove(idx);
        // confirm that child will be deallocated after being removed from children list
        assert_eq!(Arc::strong_count(&child), 1);
        let found_pid = child.getpid();
        // ++++ temporarily access child PCB exclusively
        let exit_code = child.inner_exclusive_access().exit_code;
        // ++++ release child PCB
        *translated_refmut(inner.memory_set.token(), exit_code_ptr) = exit_code;
        found_pid as isize
    } else {
        -2
    }
}
```

**RIGHT**

```rust
//os/src/syscall/process.rs
pub fn sys_waitpid(pid: isize, exit_code_ptr: *mut i32) -> isize {
    loop {
        let process = current_process();
        // find a child process

        let mut inner = process.inner_exclusive_access();
        if !inner
            .children
            .iter()
            .any(|p| pid == -1 || pid as usize == p.getpid())
        {
            return -1;
            // ---- release current PCB
        }
        let pair = inner.children.iter().enumerate().find(|(_, p)| {
            // ++++ temporarily access child PCB exclusively
            p.inner_exclusive_access().is_zombie && (pid == -1 || pid as usize == p.getpid())
            // ++++ release child PCB
        });
        if let Some((idx, _)) = pair {
            let child = inner.children.remove(idx);
            // confirm that child will be deallocated after being removed from children list
            assert_eq!(Arc::strong_count(&child), 1);
            let found_pid = child.getpid();
            // ++++ temporarily access child PCB exclusively
            let exit_code = child.inner_exclusive_access().exit_code;
            // ++++ release child PCB
            *translated_refmut(inner.memory_set.token(), exit_code_ptr) = exit_code;
            return found_pid as isize;
        } else {
            drop(inner);
            drop(process);
            suspend_current_and_run_next();
        }
    }
    // ---- release current PCB automatically
}

```

## 在initproc中直接运行测试程序，由于initproc的fork返回的pid为-1，导致程序中的测试程序获取pid错误。

让initproc运行user_shell，user_shell负责运行具体的程序

**RIGHT**

```c
#![no_std]
#![no_main]

#[macro_use]
extern crate user_lib;

use user_lib::{exec, fork, wait, yield_};

#[no_mangle]
fn main() -> i32 {
    // we shouldn't use it to run test apps, initproc is just initproc
    println!("[initproc] starting");
    if fork() == 0 {
        exec("/test_shell\0", &[core::ptr::null::<u8>()]); // user_shell
    } else {
        loop {
            let mut exit_code: i32 = 0;
            let pid = wait(&mut exit_code);
            if pid == -1 {
                yield_();
                continue;
            }
        }
    }
    0
}

```

## 启用gdk调试，初试端口被占用

修改指定端口

**ERROR**

```Makefile
run-inner: build
	@qemu-system-riscv64 \
		-machine virt \
		-nographic \
		-bios $(BOOTLOADER) \
		-device loader,file=$(KERNEL_BIN),addr=$(KERNEL_ENTRY_PA) \
		-drive file=$(U_FAT32),if=none,format=raw,id=x0 \
        -device virtio-blk-device,drive=x0,bus=virtio-mmio-bus.0 \
		-s -S

gdbclient:
	@riscv64-unknown-elf-gdb -ex 'file $(KERNEL_ELF)' -ex 'set arch riscv:rv64' -ex 'target remote localhost:1234'
```

**RIGHT**

```Makefile
run-inner: build
	@qemu-system-riscv64 \
		-machine virt \
		-nographic \
		-bios $(BOOTLOADER) \
		-device loader,file=$(KERNEL_BIN),addr=$(KERNEL_ENTRY_PA) \
		-drive file=$(U_FAT32),if=none,format=raw,id=x0 \
        -device virtio-blk-device,drive=x0,bus=virtio-mmio-bus.0 \
		# -gdb tcp::1122 -S

gdbclient:
	@riscv64-unknown-elf-gdb -ex 'file $(KERNEL_ELF)' -ex 'set arch riscv:rv64' -ex 'target remote localhost:1122'
```

## pipe返回的文件描述符错误

错误的以为返回值大小为64位，使得在写入返回值时错误，只能返回一个，另一个始终为0

**ERROR**

```rust
// os/src/syscall/fs.rs
pub fn sys_pipe(pipe: *mut usize) -> isize {
    let token = current_user_token();
    let process = current_process();
    let inner = process.inner_exclusive_access();
        Ok(fd) => fd,
        Err(errno) => return errno,
    };
    *translated_refmut(token, pipe) = read_fd;
    *translated_refmut(token, unsafe { pipe.add(1) }) = write_fd;
    SUCCESS
}
```

**RIGHT**

```rust
// os/src/syscall/fs.rs
pub fn sys_pipe(pipe: *mut u32) -> isize {
    let token = current_user_token();
    let process = current_process();
    let inner = process.inner_exclusive_access();
        Ok(fd) => fd,
        Err(errno) => return errno,
    };
    *translated_refmut(token, pipe) = read_fd as u32;
    *translated_refmut(token, unsafe { pipe.add(1) }) = write_fd as u32;
    SUCCESS
}
```

## 在测试pipe发生多次借用

在调用管道的sys_read时，如果无写入内容，会让出程序，执行yield，这时在sys_read中对进程的借用并没有释放，在执行sys_waitpid时，会搜索所有的进程来寻找指定的pid，这时之前未释放的进程就无法借用，出现多次借用的panic，我们需要在调用具体的read接口是，释放借用。


**ERROR**

```rust
// os/src/syscall/fs.rs
pub fn sys_read(fd: usize, buf: *const u8, len: usize) -> isize {
    let token = current_user_token();
    let process = current_process();
    let inner = process.inner_exclusive_access();
    let fd_table = inner.fd_table.lock();
    let file_descriptor = match fd_table.get_ref(fd) {
        Ok(file_descriptor) => file_descriptor.clone(),
        Err(errno) => return errno,
    };
    if !file_descriptor.readable() {
        return EBADF;
    }
    file_descriptor.read_user(
        None,
        UserBuffer::new(translated_byte_buffer(token, buf, len)),
    ) as isize
}
```

**RIGHT**

```rust
// os/src/syscall/fs.rs
pub fn sys_read(fd: usize, buf: *const u8, len: usize) -> isize {
    let token = current_user_token();
    let process = current_process();
    let inner = process.inner_exclusive_access();
    let fd_table = inner.fd_table.lock();
    let file_descriptor = match fd_table.get_ref(fd) {
        Ok(file_descriptor) => file_descriptor.clone(),
        Err(errno) => return errno,
    };
    if !file_descriptor.readable() {
        return EBADF;
    }
    // release current task TCB manually to avoid multi-borrow
    // yield will happend while pipe reading, which will cause multi-borrow
    drop(fd_table);
    drop(inner);
    drop(process);
    file_descriptor.read_user(
        None,
        UserBuffer::new(translated_byte_buffer(token, buf, len)),
    ) as isize
}
```

## 无法获取正确的返回值

返回值总是为0，研究后发现，在程序中,用exit来设置进程的退出值时,虽然该函数的参数类型为int型,但再父进程中只能取到其值的低8位.所以用exit返回值时,高于255的值是没有意义的.对于system函数,返回值是由两部分组成的,低8位值表示所执行的脚本在执行过程中所接收到的信号值,其余的位表示的脚本exit退出时所设置的值,即脚本内exit退出是的值的低8位,在system返回值的低9-16位.

**ERROR**

```rust
// os/src/syscall/process.rs
pub fn sys_exit(exit_code: i32) -> ! {
    exit_current_and_run_next(exit_code);
    panic!("Unreachable in sys_exit!");
}
```

**RIGHT**

```rust
// os/src/syscall/process.rs
pub fn sys_exit(exit_code: i32) -> ! {
    // the lower 8 bits of return value is for return in function
    // the lower 9-16 bits is for the return value in the system
    exit_current_and_run_next((exit_code & 0xff) << 8);
    panic!("Unreachable in sys_exit!");
}
```