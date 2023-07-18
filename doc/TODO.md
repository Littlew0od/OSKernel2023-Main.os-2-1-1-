# TODO

## 内存

- [x] mmap and ummap

## 信号

- [x] 对信号机制的支持
- [ ] 对实时信号的支持
- [x] syscall具体作用于process还是thread

## 硬件

- [x] 烧录k210的购物清单
- [x] 烧录k210流程
- [x] 支持k210

## 测评

- [x] 对shell的sh支持
- [x] 运行测试机程序
- [ ] 动态链接

## 文件系统

- [ ] 文件系统重构

## 内存

- [ ] 部分代码可以使用floor以及ceil进行优化
- [ ] mmap中的vpn_range可能存在问题

## syscall

- SYSCALL_SET_TID_ADDRESS
  - [ ] futex(clear_child_tid, FUTEX_WAKE, 1, NULL, NULL, 0);
  - [ ] support for CLONE_CHILD_CLEARTID flag

- SYSCALL_WRITEV
  - [ ] 将多个vec连接，只调用一次write()

- 返回值
  - [ ] 大部分syscall的错误返回值很草率

## 进程

- [ ] exit_current_and_run_next 只支持单线程

## run-static.sh

- [ ] fscanf
- [ ]  fwscanf
- [ ]  pthread_cancel_points
- [ ]  pthread_cancel
- [ ]  pthread_cond
- [ ]  pthread_tsd
- [ ]  socket
- [ ]  sscanf_long
- [ ]  stat
- [ ]  utime
- [ ]  fflush_exit
- [ ]  fgetwc_buffering
- [ ]  pthread_robust_detach
- [ ]  pthread_cancel_sem_wait
- [ ]  pthread_cond_smasher
- [ ]  pthread_condattr_setclock
- [ ]  pthread_exit_cancel
- [ ]  pthread_once_deadlock
- [ ]  pthread_rwlock_ebusy
- [ ]  rewind_clear_error
- [ ]  rlimit_open_files
- [ ]  setvbuf_unget
- [ ]  statvfs

## run-dynamic.sh