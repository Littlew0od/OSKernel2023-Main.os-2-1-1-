# TODO

## 内存

- [x] mmap and ummap

## 信号

- [x] 对信号机制的支持
- [ ] 对实时信号的支持
- [ ] syscall具体作用于process还是thread

## 硬件

- [x] 烧录k210的购物清单
- [x] 烧录k210流程
- [ ] 支持k210

## 测评

- [ ] 对shell的sh支持
- [ ] 运行测试机程序
- [ ] 动态链接

## 文件系统

- [ ] 文件系统重构

## syscall

- SYSCALL_SET_TID_ADDRESS
  - [ ] futex(clear_child_tid, FUTEX_WAKE, 1, NULL, NULL, 0);
  - [ ] support for CLONE_CHILD_CLEARTID flag