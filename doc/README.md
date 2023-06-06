# Main.os(2)(1)(1)操作系统内核设计文档

## 队伍信息
参赛队名： Main.os(2)(1)(1)
<br>
参赛学校：北京科技大学
<br>
队伍成员：丁博宁

![rank](./image/rank.png)

## 操作系统概要
Main.os 是一个在RISC-V架构上运行的简单内核，由Rust以及汇编语言实现，使用RustSBI作为底层支持，目前支持在qemu7.0.0上运行，现阶段已通过全部测试点。
该操作系统的实现采取高内聚低耦合的思想，使得每个模块开放尽可能少的接口以实现最多的功能，内核的主要模块有：

- drivers 外设
- fs 文件系统
- sync 并发
- syscall 系统调用
- task 进程
- trap 陷入的上下文保存与恢复
- 以及一系列文件
    - config.rs 保存系统中的各种常量
    - console.rs 提供控制台输出
    - entry.asm 定位内核的入口地址
    - lang_item.rs 提供panic——handler
    - link_initial_apps.S 挂载初始进程
    - linker-qemu.ld 链接文件，以及提供各个段的起始终止地址的全局变量
    - main.rs 内核入口
    - sbi.rs 调用sbi完成一系列基础操作
    - timer.rs 时钟相关

## 项目结构

- bootloader SBI的二进制文件
- dependency 部分依赖模块
- doc 设计文档
- doc-in-xmind rCore与npucore fat32的结构记录
- fs-fuse 磁盘文件
- initial_apps 初试进程
- os 源代码
- riscv-syscalls-testing 初赛测试程序
- user 用户态程序
