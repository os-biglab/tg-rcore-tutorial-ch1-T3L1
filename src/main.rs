//! # 第一章：应用程序与基本执行环境
//!
//! 本章实现了一个最简单的 RISC-V S 态裸机程序，展示操作系统的最小执行环境。
//!
//! ## 关键概念
//!
//! - `#![no_std]`：不使用 Rust 标准库，改用不依赖操作系统的核心库 `core`
//! - `#![no_main]`：不使用标准的 `main` 入口，自定义裸函数 `_start` 作为入口
//! - 裸函数（naked function）：不生成函数序言/尾声，可在无栈环境下执行
//! - SBI（Supervisor Binary Interface）：S 态软件向 M 态固件请求服务的标准接口
//!
//! 教程阅读建议：
//!
//! - 先看 `_start`：理解无运行时情况下的最小启动流程；
//! - 再看 `rust_main`：理解最小 I/O 路径（SBI 输出 + 关机）；
//! - 最后看 `panic_handler`：理解 no_std 程序的异常收口方式。

// 不使用标准库，因为裸机环境没有操作系统提供系统调用支持
#![no_std]
// 不使用标准入口，因为裸机环境没有 C runtime 进行初始化
#![no_main]
// RISC-V64 架构下启用严格警告和文档检查
#![cfg_attr(target_arch = "riscv64", deny(warnings, missing_docs))]
// 非 RISC-V64 架构允许死代码（用于 cargo publish --dry-run 在主机上通过编译）
#![cfg_attr(not(target_arch = "riscv64"), allow(dead_code))]

#[macro_use]
extern crate tg_console;

// 引入 SBI 调用库，提供 console_putchar（输出字符）和 shutdown（关机）功能
// 启用 nobios 特性后，tg_sbi 内建了 M-mode 启动代码，无需外部 SBI 固件
#[cfg(target_arch = "riscv64")]
use tg_console::log;
use tg_sbi::shutdown;
#[cfg(target_arch = "riscv64")]
use virtio_drivers::{Hal, MmioTransport, PhysAddr, VirtAddr, VirtIOGpu, VirtIOHeader};

#[cfg(target_arch = "riscv64")]
mod tangram;

#[cfg(target_arch = "riscv64")]
const VIRTIO_MMIO_BASE: usize = 0x1000_1000;

#[cfg(target_arch = "riscv64")]
const VIRTIO_MMIO_STRIDE: usize = 0x1000;

#[cfg(target_arch = "riscv64")]
const VIRTIO_MMIO_COUNT: usize = 8;

#[cfg(target_arch = "riscv64")]
const DMA_PAGE_SIZE: usize = 4096;

#[cfg(target_arch = "riscv64")]
const DMA_POOL_PAGES: usize = 2048;

#[cfg(target_arch = "riscv64")]
const FRAMEBUFFER_BACKGROUND: u32 = 0xff1d2128;

#[cfg(target_arch = "riscv64")]
const DEFAULT_WIDTH: usize = 1280;

#[cfg(target_arch = "riscv64")]
const DEFAULT_HEIGHT: usize = 800;

#[cfg(target_arch = "riscv64")]
const HEAP_SIZE: usize = 2 * 1024 * 1024;

#[cfg(target_arch = "riscv64")]
#[repr(align(4096))]
struct DmaPool([u8; DMA_POOL_PAGES * DMA_PAGE_SIZE]);

#[cfg(target_arch = "riscv64")]
#[repr(align(4096))]
struct KernelHeap([u8; HEAP_SIZE]);

#[cfg(target_arch = "riscv64")]
#[unsafe(link_section = ".bss.uninit")]
static mut DMA_POOL: DmaPool = DmaPool([0; DMA_POOL_PAGES * DMA_PAGE_SIZE]);

#[cfg(target_arch = "riscv64")]
#[unsafe(link_section = ".bss.uninit")]
static mut KERNEL_HEAP: KernelHeap = KernelHeap([0; HEAP_SIZE]);

#[cfg(target_arch = "riscv64")]
static DMA_NEXT_PAGE: core::sync::atomic::AtomicUsize =
    core::sync::atomic::AtomicUsize::new(0);

#[cfg(target_arch = "riscv64")]
struct SimpleHal;

#[cfg(target_arch = "riscv64")]
impl Hal for SimpleHal {
    fn dma_alloc(pages: usize) -> PhysAddr {
        use core::sync::atomic::Ordering;

        if pages == 0 {
            return 0;
        }

        loop {
            let current = DMA_NEXT_PAGE.load(Ordering::Relaxed);
            let next = match current.checked_add(pages) {
                Some(v) => v,
                None => return 0,
            };
            if next > DMA_POOL_PAGES {
                return 0;
            }
            if DMA_NEXT_PAGE
                .compare_exchange(current, next, Ordering::SeqCst, Ordering::Relaxed)
                .is_ok()
            {
                let base = unsafe { core::ptr::addr_of_mut!(DMA_POOL.0) as usize };
                return base + current * DMA_PAGE_SIZE;
            }
        }
    }

    fn dma_dealloc(_paddr: PhysAddr, _pages: usize) -> i32 {
        0
    }

    fn phys_to_virt(paddr: PhysAddr) -> VirtAddr {
        paddr
    }

    fn virt_to_phys(vaddr: VirtAddr) -> PhysAddr {
        vaddr
    }
}

/// S 态程序入口点。
///
/// 这是一个裸函数（naked function），放置在 `.text.entry` 段，
/// 链接脚本将其安排在地址 `0x80200000`。
///
/// 裸函数不生成函数序言和尾声，因此可以在没有栈的情况下执行。
/// 它完成两件事：
/// 1. 设置栈指针 `sp`，指向栈顶（栈从高地址向低地址增长）
/// 2. 跳转到 Rust 主函数 `rust_main`
#[cfg(target_arch = "riscv64")]
#[unsafe(naked)]
#[unsafe(no_mangle)]
#[unsafe(link_section = ".text.entry")]
unsafe extern "C" fn _start() -> ! {
    // 栈大小：8 页（32 KiB）
    const STACK_SIZE: usize = 8 * 4096;

    // 在 .boot.stack 段中分配启动栈，避免被 zero_bss() 清零
    #[unsafe(link_section = ".boot.stack")]
    static mut STACK: [u8; STACK_SIZE] = [0u8; STACK_SIZE];

    core::arch::naked_asm!(
        "la sp, {stack} + {stack_size}", // 将 sp 设置为栈顶地址
        "j  {main}",                      // 跳转到 rust_main
        stack_size = const STACK_SIZE,
        stack      =   sym STACK,
        main       =   sym rust_main,
    )
}

/// S 态主函数：初始化 VirtIO-GPU，渲染静态七巧板 “OS” 图案。
extern "C" fn rust_main() -> ! {
    // 第一步：清零 BSS 段（未初始化的全局变量区域）
    unsafe { tg_linker::KernelLayout::locate().zero_bss() };

    tg_console::init_console(&impls::Console);
    tg_console::set_log_level(option_env!("LOG"));
    tg_console::test_log();

    #[cfg(target_arch = "riscv64")]
    {
        init_allocator();
        log::info!("[ch1-T3L1] init virtio-gpu...");

        let gpu_mmio = match find_virtio_gpu_mmio() {
            Some(addr) => addr,
            None => panic!("virtio-gpu mmio not found"),
        };
        let transport = unsafe {
            MmioTransport::new(core::ptr::NonNull::new(gpu_mmio as *mut VirtIOHeader).unwrap())
        }
        .unwrap_or_else(|_| panic!("failed to create MmioTransport"));
        let mut gpu = match VirtIOGpu::<SimpleHal, MmioTransport>::new(transport) {
            Ok(gpu) => gpu,
            Err(_) => panic!("failed to create VirtIOGpu"),
        };

        let (width, height, framebuffer_len) = {
            let framebuffer = match gpu.setup_framebuffer() {
                Ok(buf) => buf,
                Err(_) => panic!("failed to setup framebuffer"),
            };

            let width = DEFAULT_WIDTH;
            let height = DEFAULT_HEIGHT;
            let visible_len = width.saturating_mul(height).saturating_mul(4);
            let framebuffer_len = framebuffer.len().min(visible_len);

            clear_framebuffer(framebuffer, framebuffer_len, FRAMEBUFFER_BACKGROUND);
            tangram::render_os_tangram(framebuffer, width, height);

            (width, height, framebuffer_len)
        };

        if gpu.flush().is_err() {
            panic!("failed to flush framebuffer");
        }

        log::info!("[ch1-T3L1] tangram rendered. polling GPU...");
        let _ = (width, height, framebuffer_len);
        loop {
            let _ = gpu.ack_interrupt();
            core::hint::spin_loop();
        }
    }

    #[cfg(not(target_arch = "riscv64"))]
    {
        for c in b"Hello, world!\n" {
            tg_sbi::console_putchar(*c);
        }
        shutdown(false)
    }
}

#[cfg(target_arch = "riscv64")]
fn clear_framebuffer(framebuffer: &mut [u8], len: usize, color: u32) {
    let pixel = color.to_le_bytes();
    for chunk in framebuffer[..len].chunks_exact_mut(4) {
        chunk.copy_from_slice(&pixel);
    }
}

#[cfg(target_arch = "riscv64")]
fn init_allocator() {
    let heap_ptr = unsafe { core::ptr::addr_of_mut!(KERNEL_HEAP.0) as *mut u8 };
    tg_kernel_alloc::init(heap_ptr as usize);
    unsafe {
        tg_kernel_alloc::transfer(core::slice::from_raw_parts_mut(heap_ptr, HEAP_SIZE));
    }
}

#[cfg(target_arch = "riscv64")]
fn find_virtio_gpu_mmio() -> Option<usize> {
    const VIRTIO_MAGIC: u32 = 0x7472_6976;
    const DEVICE_ID_GPU: u32 = 16;
    for slot in 0..VIRTIO_MMIO_COUNT {
        let base = VIRTIO_MMIO_BASE + slot * VIRTIO_MMIO_STRIDE;
        let magic = unsafe { (base as *const u32).read_volatile() };
        let device_id = unsafe { ((base + 0x008) as *const u32).read_volatile() };
        if magic == VIRTIO_MAGIC && device_id == DEVICE_ID_GPU {
            return Some(base);
        }
    }
    None
}

/// panic 处理函数。
///
/// `#![no_std]` 环境下必须自行实现。发生 panic 时以异常状态关机。
#[panic_handler]
fn panic(info: &core::panic::PanicInfo) -> ! {
    println!("{info}");
    shutdown(true) // true 表示异常关机
}

mod impls {
    /// 控制台实现：通过 SBI 逐字符输出。
    pub(crate) struct Console;

    impl tg_console::Console for Console {
        #[inline]
        fn put_char(&self, c: u8) {
            tg_sbi::console_putchar(c);
        }
    }
}

/// 非 RISC-V64 架构的占位模块。
///
/// 提供 `main` 等符号，使得在主机平台（如 x86_64）上也能通过编译，
/// 满足 `cargo publish --dry-run` 和 `cargo test` 的需求。
#[cfg(not(target_arch = "riscv64"))]
mod stub {
    /// 主机平台占位入口
    #[unsafe(no_mangle)]
    pub extern "C" fn main() -> i32 {
        0
    }

    /// C 运行时占位
    #[unsafe(no_mangle)]
    pub extern "C" fn __libc_start_main() -> i32 {
        0
    }

    /// Rust 异常处理人格占位
    #[unsafe(no_mangle)]
    pub extern "C" fn rust_eh_personality() {}
}
