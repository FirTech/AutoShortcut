# AutoShortcut

## 介绍

`AutoShortcut` 是用于自动创建软件快捷方式的工具。

### `AutoShortcut`有什么用？

`AutoShortcut`可以本机所安装的软件，自动创建快捷方式到指定目录（如开始菜单、桌面等）。

### `AutoShortcut`的逻辑是什么？

1. 根据配置文件查找指定程序名
2. 程序名包含目录名即判断为主程序
3. 程序目录下大小最大即判断为主程序

### 配置文件有什么用？

为了更有效的创建快捷方式，**可选**引入配置文件。配置文件包括以下功能：

- 为未创建快捷方式的程序匹配创建快捷方式;
- 为创建的快捷方式指定别名;
- 为创建的快捷方式指定命令行;

### 配置文件示例

```json
{
  "ignore": [
    "忽略目录",
    "忽略文件.exe"
  ],
  "lnkInfo": [
    {
      "name": "程序名.exe",
      "alia": "程序别名",
      "cmdline": "命令行参数"
    }
  ]
}
```

```json
{
  "ignore": [
    "WeChat.exe"
  ],
  "lnkInfo": [
    {
      "name": "QQ.exe",
      "alia": "腾讯QQ",
      "cmdline": ""
    }
  ]
}
```

## 软件架构

使用`Rust`编写，`VC-LTL`编译。

## 使用说明

本程序为命令行程序，故需要在其后面接参数运行，如直接双击程序将会出现“闪退”现象，您可通过`cmd`、`PowerShell`等终端来运行。  
注意：请使用**管理员身份**运行终端。

`AutoShortcut.exe 目标路径 快捷方式路径 [配置文件路径]`

### 基本使用

>温馨提示：如路径中含有空格请使用双引号

`AutoShortcut.exe 目标路径 快捷方式路径`

- 创建桌面快捷方式: `AutoShortcut.exe "C:\Program Files" %Desktop%`
- 创建开始菜单快捷方式: `AutoShortcut.exe "C:\Program Files" "%Programs%"`
- 创建程序文件夹: `AutoShortcut.exe -c "C:\Program Files" "%Programs%"`
- 创建快捷方式时尝试安装软件: `AutoShortcut.exe -i "C:\Program Files" "%Programs%"`

### 指定配置文件

`AutoShortcut.exe 目标路径 快捷方式路径 [配置文件路径]`

- `AutoShortcut.exe "C:\Program Files" %Desktop% D:\config.json`

D:\config.json:

```json
{
  "ignore": [
    "忽略目录",
    "忽略文件.exe"
  ],
  "lnkInfo": [
    {
      "name": "程序名.exe",
      "alia": "程序别名",
      "cmdline": "命令行参数"
    }
  ]
}
```

## 开源许可

`AutoShortcut` 使用 GPL V3.0 协议开源，请尽量遵守开源协议。

## 致谢

- slore
- qq826773297

## 参与贡献

1. Fork 本仓库
2. 新建 Feat_xxx 分支
3. 提交代码
4. 新建 Pull Request
