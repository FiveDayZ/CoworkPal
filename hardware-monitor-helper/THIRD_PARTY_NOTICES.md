# CoworkPal 内置硬件温度监控第三方声明

CoworkPal 的硬件温度助手直接引用以下 NuGet 包，并作为自包含单文件随应用分发。CoworkPal 未修改这些包的源代码。

## 直接依赖

- LibreHardwareMonitorLib 0.9.6
- 用途：访问 CPU / GPU 硬件传感器，与 TrafficMonitor 使用的硬件分类和温度传感器方案一致。
- 项目：https://github.com/LibreHardwareMonitor/LibreHardwareMonitor
- 许可证：Mozilla Public License 2.0 (MPL-2.0)
- 对应源代码：https://github.com/LibreHardwareMonitor/LibreHardwareMonitor/tree/adf717d75a17f107629f63755f0e08b992e43ca9

## 随应用分发的硬件访问驱动安装程序

- PawnIO 2.2.0
- 用途：为 CPU MSR 温度传感器提供 Windows 内核级硬件访问；仅在用户主动开启内置硬件温度监控且系统尚未安装兼容版本时静默安装。
- 项目：https://pawnio.eu/
- 发布页：https://github.com/namazso/PawnIO.Setup/releases/tag/2.2.0
- 许可证：Proprietary (Freeware)
- 官方安装程序 SHA-256：`1F519A22E47187F70A1379A48CA604981C4FCF694F4E65B734AAA74A9FBA3032`
- 官方 Authenticode 签名者：`namazso.eu`

## 传递依赖

LibreHardwareMonitorLib 0.9.6 声明的直接传递依赖如下，具体版权及许可证以各 NuGet 包内声明为准：

- DiskInfoToolkit 1.1.2 (MPL-2.0)
- HidSharp 2.6.4 (包内 LICENSE.txt)
- Mono.Posix.NETStandard 1.0.0 (Microsoft .NET Library license)
- RAMSPDToolkit-NDD 1.4.2 (MPL-2.0)
- System.IO.Ports 10.0.3 (MIT)
- System.Management 10.0.2 (MIT)
- System.Threading.AccessControl 10.0.3 (MIT)

MPL-2.0 全文：https://www.mozilla.org/MPL/2.0/
