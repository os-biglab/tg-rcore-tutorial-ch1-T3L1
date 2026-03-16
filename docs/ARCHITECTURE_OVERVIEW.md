# ch1-T3L1 软件架构总览

本文描述 `tg-rcore-tutorial-ch1-T3L1` 作为独立软件包的实现结构、执行路径与模块分工。

## 1. 系统定位

`ch1-T3L1` 是一个运行在 RISC-V S 态的 `no_std` 裸机内核最小样例，功能包括：

- 最小启动（手动设栈）；
- SBI 输出与关机；
- VirtIO-GPU 初始化与 framebuffer 刷新；
- 以“分块 + 旋转/翻转”方式渲染七巧板 `O` / `S` 静态图案。

该项目强调“教学可读性与最小闭环”，不是完整图形子系统。

---

## 2. 目录与模块职责

```text
tg-rcore-tutorial-ch1-T3L1/
├── .cargo/config.toml         # 目标平台与 QEMU runner（挂载 virtio-gpu + 串口）
├── build.rs                   # 生成 linker.ld，定义 M/S 态关键段布局
├── Cargo.toml                 # 包信息与依赖（tg-sbi/tg-kernel-alloc/virtio-drivers）
├── README.md                  # 章节说明文档
└── src/
    ├── main.rs                # 入口、GPU 初始化、DMA/HAL、错误收口
    └── tangram.rs             # 七巧板图元定义、旋转/翻转、光栅化填充
```

---

## 3. 分层架构

```text
应用流程（main.rs）
      │
      ▼
图案渲染层（tangram.rs）
      │
      ▼
VirtIO-GPU 驱动（virtio-drivers::VirtIOGpu<Hal, Transport>）
      │
      ▼
MMIO 传输层（virtio_drivers::MmioTransport）
      │
      ▼
SimpleHal（DMA 分配 / 地址转换）
      │
      ▼
QEMU virtio-gpu-device + framebuffer
```

### 3.1 `main.rs`：应用编排层

负责启动后的业务流程：
1. 初始化内核堆分配器（`tg_kernel_alloc::init/transfer`）；
2. 扫描 VirtIO MMIO 槽位并定位 GPU 设备；
3. 基于 `VirtIOHeader` 构造 `MmioTransport`，再创建 `VirtIOGpu<SimpleHal, MmioTransport>`；
4. 清屏后调用 `tangram::render_os_tangram`；
5. `gpu.flush()` 刷新到屏幕；
6. 进入轮询循环（`ack_interrupt + spin_loop`）保持静态显示。

### 3.2 `tangram.rs`：图案渲染层

将图案表示为全局静态块数组 `OS_TANGRAM_BLOCKS`：
- 每块定义：位置、颜色、旋转（`rot45`）、是否镜像（`flip_x`）、几何网格（若干三角形）；
- 几何基元：大/中/小直角三角形、正方形、平行四边形；
- 变换顺序：镜像 -> 旋转 -> 平移；
- 光栅化：包围盒扫描 + 边函数，直接写 framebuffer 像素。

### 3.3 `SimpleHal`：驱动适配层

为 `virtio-drivers` 提供最小 HAL：
- `dma_alloc`：从全局 `DMA_POOL` 原子分配连续页；
- `dma_dealloc`：当前为 no-op（教学最小实现）；
- `phys_to_virt/virt_to_phys`：恒等映射（QEMU `virt` + 当前地址模型）。

---

## 4. 启动与运行流程

### 4.1 构建期（build-time）

- `build.rs` 在 RISC-V 目标下生成链接脚本；
- 链接脚本安排：
  - M-mode 区域（由 `tg-sbi` 使用）
  - S-mode 区域（本程序 `_start` 与后续代码）

### 4.2 运行期（run-time）

1. `_start` 设栈并跳转 `rust_main`；
2. `rust_main` 初始化堆分配器并输出启动日志；
3. 通过 `find_virtio_gpu_mmio()` 扫描 `0x1000_1000 + slot * 0x1000`，匹配 `device_id=16`；
4. 建立 GPU 驱动与 framebuffer；
5. 绘制七巧板 `OS` 并 `flush`；
6. 循环轮询，维持静态显示。

---

## 5. 配置与外部依赖

### 5.1 关键配置

`.cargo/config.toml`：
- 默认目标：`riscv64gc-unknown-none-elf`；
- runner 使用：
  - `qemu-system-riscv64 -machine virt`
  - `-device virtio-gpu-device`
  - `-serial mon:stdio`
  - `-bios none`

### 5.2 外部依赖

- `tg-sbi`：提供 `console_putchar` / `shutdown`；
- `tg-kernel-alloc`：提供 `#[global_allocator]` 与堆管理接口；
- `virtio-drivers = 0.1.0`：提供 `VirtIOGpu`、`MmioTransport` 与 VirtIO MMIO 协议封装。

---

## 6. 当前实现边界

为保持教学最小实现，当前未实现：
- 双缓冲与局部脏矩形刷新；
- 输入事件与交互（仅静态图）；
- VSync/定时器驱动动画；
- 中断驱动图形栈（当前为轮询）；
- 图元抗锯齿、图层系统、字体渲染。

---

## 7. 可扩展方向

后续若继续演进，可按以下顺序：

1. 在 `tangram.rs` 增加“逐块依次渲染”调度（你已预留块级数据结构）；
2. 支持插值动画（平移/旋转/翻转过渡）；
3. 从轮询改为中断驱动，减少空转；
4. 将 GPU 设备抽象成统一 `Display` trait，便于后续章节复用；
5. 增加测试图层（坐标轴/包围盒）辅助几何调参。
