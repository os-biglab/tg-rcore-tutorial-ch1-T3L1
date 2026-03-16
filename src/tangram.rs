//! 七巧板“OS”图案数据与渲染。
//!
//! 本模块将图案拆分为多个“块”（block）：
//! - 每块包含局部几何形状（由三角形网格表示）
//! - 每块包含放置位置与颜色
//! - 所有数据均为全局静态数组，避免运行期在栈上构造大对象

/// 像素格式：`virtio-gpu` 的 `B8G8R8A8UNORM`，按 little-endian 写入 `u32`。
const fn bgra(r: u8, g: u8, b: u8) -> u32 {
    ((0xff_u32) << 24) | ((r as u32) << 16) | ((g as u32) << 8) | (b as u32)
}

#[derive(Copy, Clone)]
struct Point {
    x: i32,
    y: i32,
}

#[derive(Copy, Clone)]
struct Block {
    pos: Point,
    color: u32,
    /// 旋转步数：每步 45°，取值范围 0..=7。
    rot45: u8,
    /// 是否做左右镜像（用于平行四边形翻转）。
    flip_x: bool,
    mesh: &'static [[Point; 3]],
}

const SMALL_HALF_LEG: i32 = 32;
const MEDIUM_HALF_LEG: i32 = 45;
const LARGE_HALF_LEG: i32 = 64;
const SQUARE_HALF_DIAG: i32 = 64;

const TRI_LARGE: [[Point; 3]; 1] = [[
    Point {
        x: -LARGE_HALF_LEG,
        y: -LARGE_HALF_LEG,
    },
    Point {
        x: LARGE_HALF_LEG,
        y: -LARGE_HALF_LEG,
    },
    Point {
        x: -LARGE_HALF_LEG,
        y: LARGE_HALF_LEG,
    },
]];

const TRI_MEDIUM: [[Point; 3]; 1] = [[
    Point {
        x: -MEDIUM_HALF_LEG,
        y: -MEDIUM_HALF_LEG,
    },
    Point {
        x: MEDIUM_HALF_LEG,
        y: -MEDIUM_HALF_LEG,
    },
    Point {
        x: -MEDIUM_HALF_LEG,
        y: MEDIUM_HALF_LEG,
    },
]];

const TRI_SMALL: [[Point; 3]; 1] = [[
    Point {
        x: -SMALL_HALF_LEG,
        y: -SMALL_HALF_LEG,
    },
    Point {
        x: SMALL_HALF_LEG,
        y: -SMALL_HALF_LEG,
    },
    Point {
        x: -SMALL_HALF_LEG,
        y: SMALL_HALF_LEG,
    },
]];

const SQUARE: [[Point; 3]; 2] = [
    [
        Point {
            x: -SQUARE_HALF_DIAG,
            y: 0,
        },
        Point {
            x: 0,
            y: -SQUARE_HALF_DIAG,
        },
        Point {
            x: SQUARE_HALF_DIAG,
            y: 0,
        },
    ],
    [
        Point {
            x: -SQUARE_HALF_DIAG,
            y: 0,
        },
        Point {
            x: SQUARE_HALF_DIAG,
            y: 0,
        },
        Point {
            x: 0,
            y: SQUARE_HALF_DIAG,
        },
    ],
];

const PARALLELOGRAM: [[Point; 3]; 2] = [
    [
        Point { x: -96, y: -32 },
        Point { x: 32, y: -32 },
        Point { x: 96, y: 32 },
    ],
    [
        Point { x: -96, y: -32 },
        Point { x: 96, y: 32 },
        Point { x: -32, y: 32 },
    ],
];

/// 全局分块数据：左侧 7 块拼 “O”，右侧 7 块拼 “S”。
static OS_TANGRAM_BLOCKS: [Block; 14] = [
    // ----- O -----
    Block {
        pos: Point { x: 210, y: 210 },
        color: bgra(255, 84, 84),
        rot45: 0,
        flip_x: false,
        mesh: &TRI_SMALL,
    },
    Block {
        pos: Point { x: 354, y: 270 },
        color: bgra(255, 170, 60),
        rot45: 2,
        flip_x: false,
        mesh: &TRI_LARGE,
    },
    Block {
        pos: Point { x: 350, y: 400 },
        color: bgra(255, 236, 96),
        rot45: 4,
        flip_x: false,
        mesh: &TRI_MEDIUM,
    },
    Block {
        pos: Point { x: 300, y: 430 },
        color: bgra(130, 220, 92),
        rot45: 6,
        flip_x: false,
        mesh: &TRI_MEDIUM,
    },
    Block {
        pos: Point { x: 210, y: 274 },
        color: bgra(76, 201, 240),
        rot45: 2,
        flip_x: false,
        mesh: &PARALLELOGRAM,
    },
    Block {
        pos: Point { x: 280, y: 180 },
        color: bgra(114, 137, 218),
        rot45: 2,
        flip_x: false,
        mesh: &TRI_SMALL,
    },
    Block {
        pos: Point { x: 230, y: 410 },
        color: bgra(196, 110, 245),
        rot45: 2,
        flip_x: true,
        mesh: &PARALLELOGRAM,
    },
    // ----- S -----
    Block {
        pos: Point { x: 930, y: 330 },
        color: bgra(255, 99, 132),
        rot45: 3,
        flip_x: false,
        mesh: &TRI_LARGE,
    },
    Block {
        pos: Point { x: 900, y: 150 },
        color: bgra(255, 159, 64),
        rot45: 1,
        flip_x: false,
        mesh: &TRI_MEDIUM,
    },
    Block {
        pos: Point { x: 790, y: 230 },
        color: bgra(255, 205, 86),
        rot45: 6,
        flip_x: false,
        mesh: &PARALLELOGRAM,
    },
    Block {
        pos: Point { x: 870, y: 300 },
        color: bgra(75, 192, 192),
        rot45: 1,
        flip_x: false,
        mesh: &SQUARE,
    },
    Block {
        pos: Point { x: 940, y: 120 },
        color: bgra(54, 162, 235),
        rot45: 2,
        flip_x: false,
        mesh: &TRI_SMALL,
    },
    Block {
        pos: Point { x: 850, y: 460 },
        color: bgra(153, 102, 255),
        rot45: 5,
        flip_x: false,
        mesh: &TRI_MEDIUM,
    },
    Block {
        pos: Point { x: 900, y: 465 },
        color: bgra(201, 203, 207),
        rot45: 4,
        flip_x: false,
        mesh: &TRI_SMALL,
    },
];

pub(crate) fn render_os_tangram(framebuffer: &mut [u8], width: usize, height: usize) {
    for block in &OS_TANGRAM_BLOCKS {
        for triangle in block.mesh {
            draw_triangle(
                framebuffer,
                width,
                height,
                [
                    translate(
                        rotate_45(maybe_flip_x(triangle[0], block.flip_x), block.rot45),
                        block.pos,
                    ),
                    translate(
                        rotate_45(maybe_flip_x(triangle[1], block.flip_x), block.rot45),
                        block.pos,
                    ),
                    translate(
                        rotate_45(maybe_flip_x(triangle[2], block.flip_x), block.rot45),
                        block.pos,
                    ),
                ],
                block.color,
            );
        }
    }
}

fn maybe_flip_x(p: Point, flip_x: bool) -> Point {
    if flip_x {
        Point { x: -p.x, y: p.y }
    } else {
        p
    }
}

fn translate(p: Point, offset: Point) -> Point {
    Point {
        x: p.x + offset.x,
        y: p.y + offset.y,
    }
}

fn rotate_45(p: Point, rot45: u8) -> Point {
    const K: i32 = 724;
    match rot45 & 0x7 {
        0 => p,
        1 => Point {
            x: ((p.x - p.y) * K) >> 10,
            y: ((p.x + p.y) * K) >> 10,
        },
        2 => Point { x: -p.y, y: p.x },
        3 => Point {
            x: ((-p.x - p.y) * K) >> 10,
            y: ((p.x - p.y) * K) >> 10,
        },
        4 => Point { x: -p.x, y: -p.y },
        5 => Point {
            x: ((p.y - p.x) * K) >> 10,
            y: ((-p.x - p.y) * K) >> 10,
        },
        6 => Point { x: p.y, y: -p.x },
        _ => Point {
            x: ((p.x + p.y) * K) >> 10,
            y: ((p.y - p.x) * K) >> 10,
        },
    }
}

fn draw_triangle(framebuffer: &mut [u8], width: usize, height: usize, tri: [Point; 3], color: u32) {
    let (mut min_x, mut max_x) = (tri[0].x, tri[0].x);
    let (mut min_y, mut max_y) = (tri[0].y, tri[0].y);
    for p in &tri[1..] {
        min_x = min_x.min(p.x);
        max_x = max_x.max(p.x);
        min_y = min_y.min(p.y);
        max_y = max_y.max(p.y);
    }

    min_x = min_x.max(0);
    min_y = min_y.max(0);
    max_x = max_x.min(width.saturating_sub(1) as i32);
    max_y = max_y.min(height.saturating_sub(1) as i32);

    if min_x > max_x || min_y > max_y {
        return;
    }

    let area = edge(tri[0], tri[1], tri[2]);
    if area == 0 {
        return;
    }

    for y in min_y..=max_y {
        for x in min_x..=max_x {
            let p = Point { x, y };
            let w0 = edge(tri[1], tri[2], p);
            let w1 = edge(tri[2], tri[0], p);
            let w2 = edge(tri[0], tri[1], p);

            if same_sign(area, w0) && same_sign(area, w1) && same_sign(area, w2) {
                put_pixel(framebuffer, width, x as usize, y as usize, color);
            }
        }
    }
}

fn edge(a: Point, b: Point, p: Point) -> i64 {
    (b.x - a.x) as i64 * (p.y - a.y) as i64 - (b.y - a.y) as i64 * (p.x - a.x) as i64
}

fn same_sign(a: i64, b: i64) -> bool {
    (a >= 0 && b >= 0) || (a <= 0 && b <= 0)
}

fn put_pixel(framebuffer: &mut [u8], width: usize, x: usize, y: usize, color: u32) {
    let idx = (y * width + x) * 4;
    if idx + 4 <= framebuffer.len() {
        framebuffer[idx..idx + 4].copy_from_slice(&color.to_le_bytes());
    }
}
