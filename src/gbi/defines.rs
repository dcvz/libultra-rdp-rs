use self::RSPGeometry::{G_CULL_BACK, G_CULL_FRONT};

/*
 * Generic Gfx Packet
 */
#[repr(C)]
#[derive(Clone, Copy)]
pub struct GWords {
    pub w0: libc::uintptr_t,
    pub w1: libc::uintptr_t,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub union Gfx {
    pub words: GWords,
    pub force_structure_alignment: libc::c_longlong,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct VertexColor {
    pub position: [libc::c_short; 3], /* x, y, z (signed 16-bit integer) */
    pub flag: libc::c_ushort,         /* Currently has no meaning */
    pub texture_coordinates: [libc::c_short; 3], /* Texture coordinates (s10.5) */
    pub color: [libc::c_uchar; 4],    /* Color & alpha (0~255, unsigned 8-bit) */
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct VertexNormal {
    pub position: [libc::c_short; 3], /* x, y, z (signed 16-bit integer) */
    pub flag: libc::c_ushort,         /* Currently has no meaning */
    pub texture_coordinates: [libc::c_short; 3], /* Texture coordinates (s10.5) */
    pub normal: [libc::c_uchar; 3],   /* Normal vector (x, y, z) */
    pub alpha: libc::c_uchar,         /* Alpha (0~255, unsigned 8-bit) */
}

#[repr(C)]
pub union Vertex {
    pub color: VertexColor,
    pub normal: VertexNormal,
    pub force_structure_alignment: libc::c_longlong,
}

#[repr(C)]
pub struct Light {
    pub col: [libc::c_uchar; 3], /* diffuse light value (rgba) */
    pad1: libc::c_char,
    pub colc: [libc::c_uchar; 3], /* copy of diffuse light value (rgba) */
    pad2: libc::c_char,
    pub dir: [libc::c_char; 3], /* direction of light (normalized) */
    pad3: libc::c_char,
}

impl Light {
    pub const ZERO: Self = Self {
        col: [0, 0, 0],
        pad1: 0,
        colc: [0, 0, 0],
        pad2: 0,
        dir: [0, 0, 0],
        pad3: 0,
    };

    pub const fn new(
        col: [libc::c_uchar; 3],
        colc: [libc::c_uchar; 3],
        dir: [libc::c_char; 3],
    ) -> Self {
        Self {
            col,
            pad1: 0,
            colc,
            pad2: 0,
            dir,
            pad3: 0,
        }
    }
}

#[cfg(feature = "f3dex2")]
pub enum G_MTX {
    NOPUSH_MUL_MODELVIEW = 0x00,
    PUSH = 0x01,
    // MUL = 0x00,
    LOAD = 0x02,
    // MODELVIEW = 0x00,
    PROJECTION = 0x04,
}

#[cfg(not(feature = "f3dex2"))]
pub enum G_MTX {
    NOPUSH_MUL_MODELVIEW = 0x00,
    PUSH = 0x04,
    // MUL = 0x00,
    LOAD = 0x02,
    // MODELVIEW = 0x00,
    PROJECTION = 0x01,
}

pub enum RSPGeometry {
    G_ZBUFFER = 1 << 0,
    G_SHADE = 1 << 2,
    G_CULL_FRONT = 1 << 9,
    G_CULL_BACK = 1 << 10,
    G_CULL_BOTH = G_CULL_FRONT as isize | G_CULL_BACK as isize,
    G_FOG = 1 << 16,
    G_LIGHTING = 1 << 17,
    G_TEXTURE_GEN = 1 << 18,
    G_TEXTURE_GEN_LINEAR = 1 << 19,
    G_SHADING_SMOOTH = 1 << 21,
    G_CLIPPING = 1 << 23,
}
