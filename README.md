# OSKernel2023-Main.os(2)(1)(1)

Main.os(2)(1)(1)  北京科技大学

![USTB](./doc/image/USTB.jpg)

markdown文档在[doc](./doc/)文件夹中
文件系统结构文档在[doc-in-xmind](./doc/doc-in-xmind/)文件夹中
## 队伍信息


参赛队名： Main.os(2)(1)(1)
<br>
参赛学校：北京科技大学
<br>
队伍成员：丁博宁

## 使用说明

在根目录中运行`make all`，即可在根目录获得操作系统以及SBI的二进制文件

在os文件夹中，运行`make apps`编译用户态应用

运行`make fat32`构建文件镜像

运行`make run`编译内核程序并使用qemu启动

## Tips
如何添加本地依赖
[how to add cargo vendor](https://fuchsia.googlesource.com/third_party/cargo-vendor/#:~:text=Simply%20run%20cargo%20vendor%20inside%20of%20any%20Cargo,which%20contains%20the%20source%20of%20all%20crates.io%20dependencies.)
wsl 访问USB
[how to connect usb in wsl](https://learn.microsoft.com/zh-cn/windows/wsl/connect-usb)