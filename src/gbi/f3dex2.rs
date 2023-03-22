use super::super::rcp::RCP;
use super::super::rdp::RDP;
use super::super::rsp::RSP;
use super::{
    utils::get_c0,
    GBI, GBIDefinition, GBIResult,
    defines::{Gfx, Vertex, G_MTX}
};

pub enum F3DEX2 {
    // DMA
    G_VTX = 0x01,
    G_MODIFYVTX = 0x02,
    G_CULLDL = 0x03,
    G_BRANCH_Z = 0x04,
    G_TRI1 = 0x05,
    G_TRI2 = 0x06,
    G_QUAD = 0x07,
    G_LINE3D = 0x08,

    G_TEXTURE = 0xD7,
    G_POPMTX = 0xD8,
    G_GEOMETRYMODE = 0xD9,
    G_MTX = 0xDA,
    G_LOAD_UCODE = 0xDD,
    G_DL = 0xDE,
    G_ENDDL = 0xDF,

    // RDP
    G_SETCIMG = 0xFF,
    G_SETZIMG = 0xFE,
    G_SETTIMG = 0xFD,
    G_SETCOMBINE = 0xFC,
    G_SETENVCOLOR = 0xFB,
    G_SETPRIMCOLOR = 0xFA,
    G_SETBLENDCOLOR = 0xF9,
    G_SETFOGCOLOR = 0xF8,
    G_SETFILLCOLOR = 0xF7,
    G_FILLRECT = 0xF6,
    G_SETTILE = 0xF5,
    G_LOADTILE = 0xF4,
    G_LOADBLOCK = 0xF3,
    G_SETTILESIZE = 0xF2,
    G_LOADTLUT = 0xF0,
    G_RDPSETOTHERMODE = 0xEF,
    G_SETPRIMDEPTH = 0xEE,
    G_SETSCISSOR = 0xED,
    G_SETCONVERT = 0xEC,
    G_SETKEYR = 0xEB,
    G_SETKEYFB = 0xEA,
    G_RDPFULLSYNC = 0xE9,
    G_RDPTILESYNC = 0xE8,
    G_RDPPIPESYNC = 0xE7,
    G_RDPLOADSYNC = 0xE6,
    G_TEXRECTFLIP = 0xE5,
    G_TEXRECT = 0xE4,
    G_SETOTHERMODE_H = 0xE3,
    G_SETOTHERMODE_L = 0xE2,
}

impl GBIDefinition for F3DEX2 {
    fn setup(gbi: &mut GBI) {
        gbi.register(F3DEX2::G_DL as usize, F3DEX2::gsp_display_list);
        gbi.register(F3DEX2::G_GEOMETRYMODE as usize, F3DEX2::gsp_geometry_mode);
        gbi.register(F3DEX2::G_VTX as usize, F3DEX2::gsp_vertex);
        gbi.register(F3DEX2::G_MTX as usize, F3DEX2::gsp_matrix);
        gbi.register(F3DEX2::G_TRI1 as usize, F3DEX2::gsp_triangles1);

        gbi.register(F3DEX2::G_LOAD_UCODE as usize, F3DEX2::gsp_load_ucode);
    }
}

impl F3DEX2 {
    pub fn gsp_display_list(_rdp: &mut RDP, _rsp: &mut RSP, command: *const Gfx) -> GBIResult {
        let c0 = unsafe { get_c0((*command).words.w0, 16, 1) };
        if c0 == 0 {
            // Push return address
            let return_address = unsafe { (*command).words.w1 };
            let return_address = RCP::segmented_address(return_address);
            let command = return_address as *const Gfx;

            GBIResult::Recurse(command)
        } else {
            let return_address = unsafe { (*command).words.w1 };
            let return_address = RCP::segmented_address(return_address);
            GBIResult::SetAddressWithDecrease(return_address)
        }
    }

    pub fn gsp_geometry_mode(_rdp: &mut RDP, rsp: &mut RSP, command: *const Gfx) -> GBIResult {
        let clear_bits = unsafe { get_c0((*command).words.w0, 0, 24) };
        let set_bits = unsafe { (*command).words.w1 };

        rsp.geometry_mode &= !clear_bits as u32;
        rsp.geometry_mode |= set_bits as u32;
        rsp.state_changed = true;

        GBIResult::Continue
    }

    pub fn gsp_vertex(_rdp: &mut RDP, rsp: &mut RSP, command: *const Gfx) -> GBIResult {
        let p0 = unsafe { get_c0((*command).words.w0, 12, 8) };
        let p1 = unsafe { get_c0((*command).words.w0, 1, 7) };
        let p2 = unsafe { get_c0((*command).words.w0, 12, 8) };
        let segmented_address = unsafe { RCP::segmented_address((*command).words.w1) };
        let vertex_pointer = segmented_address as *const Vertex;
        rsp.gsp_vertex(vertex_pointer, p0, p1 - p2);

        GBIResult::Continue
    }

    pub fn gsp_matrix(_rdp: &mut RDP, rsp: &mut RSP, command: *const Gfx) -> GBIResult {
        let mtx_address = unsafe { (*command).words.w1 };
        let c0 = unsafe { get_c0((*command).words.w0, 0, 8) } ^ G_MTX::PUSH as usize;
        rsp.gsp_matrix(mtx_address, c0);

        GBIResult::Continue
    }

    pub fn gsp_triangles1(_rdp: &mut RDP, rsp: &mut RSP, command: *const Gfx) -> GBIResult {
        let p0 = unsafe { get_c0((*command).words.w0, 16, 8) } / 2;
        let p1 = unsafe { get_c0((*command).words.w0, 8, 8) } / 2;
        let p2 = unsafe { get_c0((*command).words.w0, 0, 8) } / 2;

        rsp.gsp_triangles(
            p0.try_into().unwrap(),
            p1.try_into().unwrap(),
            p2.try_into().unwrap(),
        );

        GBIResult::Continue
    }

    pub fn gsp_load_ucode(_rdp: &mut RDP, rsp: &mut RSP, _command: *const Gfx) -> GBIResult {
        // TODO: Do a fuller reset?
        rsp.fog_multiplier = 0;
        rsp.fog_offset = 0;

        GBIResult::Continue
    }
}

// MARK: - Custom RSP State Mutators

impl F3DEX2 {}
