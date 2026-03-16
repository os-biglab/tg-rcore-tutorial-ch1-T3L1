# ch1-T3L1 GPU/七巧板支持实现报告

> 文件名沿用 T2L3 文档风格；本报告内容针对 T3L1 的 VirtIO-GPU + 七巧板渲染实现。

## 1. 目标与约束

本次需求：在 `ch1-T3L1` 中基于 `ch1` 扩展最小图形能力，完成以下闭环：

1. 初始化 VirtIO-GPU；
2. 获取并操作 framebuffer；
3. 将代码中数组定义的彩色七巧板数据渲染到屏幕；
4. 左侧形成 `O` 图案，右侧形成 `S` 图案；
5. 图块可旋转，平行四边形可翻转；
6. 内核运行在 S 态，图案与渲染数据使用全局变量，避免爆栈。

约束：
- 保持 `ch1` 风格（`no_std` / `no_main`，最小功能）；
- 可使用轮询，不引入复杂中断框架；
- 不引入多余页面/模块复杂度。

---

## 2. 规划（Plan）

按最小可行路径拆分为 6 步：

1. 运行环境：QEMU 挂载 `virtio-gpu-device`，打开串口输出；
2. 依赖：引入 `virtio-drivers` 与 `tg-kernel-alloc`；
3. GPU 接入：实现 `SimpleHal` + DMA 池，接通 `VirtIOGpu`；
4. 图案模型：块级全局数据（位置/颜色/旋转/翻转/网格）；
5. 渲染流程：清屏 -> 绘制 -> `flush` -> 轮询保持；
6. 验证调优：修复编译/运行问题并持续微调图案布局。

---

## 3. 实现（Implementation）

### 3.1 依赖与运行配置

- 在 `Cargo.toml` 增加：
  - `virtio-drivers = 0.1.0`
  - `tg-kernel-alloc`（用于 `#[global_allocator]`）
- 在 `.cargo/config.toml` 的 runner 增加：
  - `-device virtio-gpu-device`
  - `-serial mon:stdio`

### 3.2 入口与 GPU 初始化

`main.rs` 中主路径：
- `init_allocator()`：初始化全局堆；
- `find_virtio_gpu_mmio()`：扫描 8 个 VirtIO MMIO 槽位并匹配 `device_id=16`；
- 先构造 `MmioTransport::new(NonNull<VirtIOHeader>)`；
- 再调用 `VirtIOGpu::<SimpleHal, MmioTransport>::new(transport)` 与 `setup_framebuffer()`；
- 清屏与图案渲染后 `flush()`；
- 进入轮询循环维持显示。

### 3.3 HAL 与内存

- 全局 DMA 池：`DMA_POOL` + `DMA_NEXT_PAGE` 原子递增分配；
- 全局堆区：`KERNEL_HEAP`（避免运行期大对象压栈）；
- 地址映射：当前采用恒等映射（物理/虚拟同址）。

### 3.4 七巧板渲染模型

`tangram.rs` 采用块级数据驱动：
- `Block { pos, color, rot45, flip_x, mesh }`；
- `mesh` 指向三角形数组（大/中/小三角、正方形、平行四边形）；
- 变换顺序固定：`flip_x -> rotate_45 -> translate`；
- 填充算法：边函数 + 包围盒扫描，写入 `BGRA` 像素。

---

## 4. 测试与验证（Test）

### 4.1 编译验证

- 多轮 `cargo run`/`cargo build` 通过；
- 严格 lint 场景下针对相关错误已逐项修复。

### 4.2 运行验证

串口日志稳定到达：
1. `[ch1-T3L1] init virtio-gpu...`
2. `[ch1-T3L1] tangram rendered. polling GPU...`

说明主路径（初始化 -> 渲染 -> 刷新 -> 保持）闭环完成。

---

## 5. 调试过程（Debug）

### 问题 1：`no global memory allocator found`

现象：
- 编译报错缺少全局分配器。

修复：
- 接入 `tg-kernel-alloc`；
- 在 `rust_main` 开始处执行 `init + transfer` 初始化可用堆区。

### 问题 2：`E0133` 访问 `static mut` 需 `unsafe`

现象：
- 对 `DMA_POOL` 取地址时报 `use of mutable static is unsafe`。

修复：
- 显式 `unsafe` 访问并保持访问点最小化。

### 问题 3：`E0502` `gpu` 借用冲突

现象：
- `setup_framebuffer()` 返回 `&mut [u8]` 时再次借用 `gpu` 导致冲突。

修复：
- 渲染阶段只使用 framebuffer 与固定分辨率常量，避免在同借用期访问 `gpu`。

### 问题 4：运行后无窗口/秒退

现象：
- 无 GUI 或进程很快结束。

定位：
- 早期硬编码 MMIO 地址在不同设备枚举下不稳定。

修复：
- 改为运行时扫描 VirtIO 槽位定位 GPU；
- 增加串口日志便于确认执行阶段；
- 结果稳定进入渲染轮询。

### 问题 5：图案形态与目标不一致

现象：
- 初版更接近“散开的彩块”，与目标 `O/S` 拼图差距大。

修复：
- 重构图元比例、块级旋转与平行四边形翻转参数；
- 多轮坐标微调，逐步靠近“左 O 右 S”的目标布局。人工调整。

### 问题 6：升级 `virtio-drivers` 后 GPU 泛型不匹配

现象：
- 报错 `VirtIOGpu` 需要 2 个泛型参数（`H, T`），只传了 `SimpleHal`；
- 同时触发 `E0282` 类型推断失败。

原因：
- `virtio-drivers 0.1.0` 的 GPU 驱动 API 为 `VirtIOGpu<'a, H, T>`，
  其中 `T` 是 `Transport`（如 `MmioTransport`），不再是旧接口风格。

修复：
- 增加 `MmioTransport` 引入并先构造 transport；
- 改为 `VirtIOGpu::<SimpleHal, MmioTransport>::new(transport)`。

---

## 6. 最终结果

已完成：
- S 态最小图形环境可运行；
- VirtIO-GPU framebuffer 可写可刷；
- 七巧板数据全局化、块级化，可旋转/可翻转；
- 已具备后续“逐块依次渲染”演进基础。

当前状态：
- 工程可稳定运行并显示图案；
- 图案位置仍可继续按视觉目标进行小步参数精调。

---

## 7. 后续建议

1. 将 `OS_TANGRAM_BLOCKS` 参数表拆成单独“标定表”，便于快速迭代；
2. 增加 `DEBUG_OVERLAY`（绘制块中心点/包围盒）提高调参效率；
3. 引入逐帧渲染接口，实现“按块依次出现”的动画版 T3L1；
4. 统一背景色与配色方案，贴近目标示例图。
