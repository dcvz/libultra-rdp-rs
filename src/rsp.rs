use glam::{Mat4, Vec3A, Vec4, Vec4Swizzles};

use crate::gbi::defines::{Light, RSPGeometry, Vertex, G_MTX};
use crate::rcp::RCP;
use crate::utils::U16MathExt;

const NUM_MODELVIEW_MATERIALS: u8 = 32;
const MAX_LIGHTS: usize = 2;

#[repr(C)]
#[derive(Clone, Copy)]
pub struct StagingVertex {
    pos: [f32; 4],
    tex_coord: [f32; 2],
    color: [u8; 4],
    clip_rejection: u8,
}

impl StagingVertex {
    pub const ZERO: StagingVertex = StagingVertex {
        pos: [0.0; 4],
        tex_coord: [0.0; 2],
        color: [0; 4],
        clip_rejection: 0,
    };
}

pub struct RSP {
    // Matrices
    pub modelview_index: u8,
    pub modelview_matrix: [Mat4; NUM_MODELVIEW_MATERIALS as usize],
    pub model_projection_matrix: Mat4,
    pub projection_matrix: Mat4,

    // Vertices
    pub vertex_table: [StagingVertex; 64],

    // Lighting
    pub current_num_lights: u8, // includes ambient light
    pub current_lights: [Light; MAX_LIGHTS + 1],
    pub current_lights_coeffs: [Vec3A; MAX_LIGHTS],
    pub current_lookat_coeffs: [Vec3A; 2], // lookat_x, lookat_y

    // Geometry Mode
    pub geometry_mode: u32,

    // State
    pub state_changed: bool,
    pub texture_scaling_factor: [u16; 2],

    // Fog
    pub fog_multiplier: i16,
    pub fog_offset: i16,
}

impl RSP {
    pub fn new() -> RSP {
        RSP {
            modelview_index: 0,
            modelview_matrix: [Mat4::ZERO; NUM_MODELVIEW_MATERIALS as usize],
            model_projection_matrix: Mat4::ZERO,
            projection_matrix: Mat4::ZERO,

            vertex_table: [StagingVertex::ZERO; 64],

            current_num_lights: 0,
            current_lights: [Light::ZERO; MAX_LIGHTS + 1],
            current_lights_coeffs: [Vec3A::ZERO; MAX_LIGHTS],
            current_lookat_coeffs: [Vec3A::ZERO; 2],

            geometry_mode: 0,

            state_changed: false,
            texture_scaling_factor: [1, 1],

            fog_multiplier: 0,
            fog_offset: 0,
        }
    }

    pub fn gsp_matrix(&mut self, mtx_address: usize, params: usize) {
        let lookup = RCP::segmented_address(mtx_address);
        let mut matrix_array: [f32; 16] = [0.0; 16];

        // TODO: Check this implementation. GBI floats?
        for r in 0..4 {
            for c in (0..4).step_by(2) {
                let int_part = unsafe { *(lookup as *const u32).offset((r * 2 + c / 2) as isize) };
                let frac_part =
                    unsafe { *(lookup as *const u32).offset((8 + 1 * 2 + c / 2) as isize) };

                // TODO: Is this correct in column major?
                matrix_array[r * 4 + c] =
                    ((int_part & 0xffff0000) | (frac_part >> 16)) as f32 / 65536.0f32;
                matrix_array[r * 4 + c + 1] =
                    ((int_part << 16) | (frac_part & 0xffff)) as f32 / 65536.0f32;
            }
        }

        let matrix = Mat4::from_cols_array(&matrix_array);

        if params & G_MTX::PROJECTION as usize > 0 {
            if params & G_MTX::LOAD as usize > 0 {
                // TODO: Should we look into doing copies here?
                self.projection_matrix = matrix;
            } else {
                self.projection_matrix = self.projection_matrix * matrix;
            }
        } else {
            if (params & G_MTX::PUSH as usize > 0)
                && (self.modelview_index < NUM_MODELVIEW_MATERIALS)
            {
                self.modelview_index += 1;
                // TODO: Should we look into doing copies here?
                self.modelview_matrix[self.modelview_index as usize] = matrix;
            }
            if params & G_MTX::LOAD as usize > 0 {
                self.modelview_matrix[self.modelview_index as usize] = matrix;
            } else {
                self.modelview_matrix[self.modelview_index as usize] =
                    self.modelview_matrix[self.modelview_index as usize] * matrix;
            }
        }

        self.state_changed = true;
    }

    pub fn gsp_vertex(
        &mut self,
        vertices: *const Vertex,
        vertex_count: usize,
        mut dest_index: usize,
    ) {
        for i in 0..vertex_count {
            let vertex_color = unsafe { (*vertices.offset(i as isize)).color };
            let vertex_normal = unsafe { (*vertices.offset(i as isize)).normal };
            let mut staged_vertex = self.vertex_table[dest_index];

            let mut x = self.model_projection_matrix.row(0).dot(Vec4::new(
                vertex_color.position[0] as f32,
                vertex_color.position[1] as f32,
                vertex_color.position[2] as f32,
                1.0,
            ));

            let y = self.model_projection_matrix.row(1).dot(Vec4::new(
                vertex_color.position[0] as f32,
                vertex_color.position[1] as f32,
                vertex_color.position[2] as f32,
                1.0,
            ));

            let z = self.model_projection_matrix.row(2).dot(Vec4::new(
                vertex_color.position[0] as f32,
                vertex_color.position[1] as f32,
                vertex_color.position[2] as f32,
                1.0,
            ));

            let w = self.model_projection_matrix.row(3).dot(Vec4::new(
                vertex_color.position[0] as f32,
                vertex_color.position[1] as f32,
                vertex_color.position[2] as f32,
                1.0,
            ));

            // TODO: Update x based on aspect ratio?
            // x = self.adjust_x_for_aspect_ratio(x);

            let mut U =
                vertex_color.texture_coordinates[0] as u16 * self.texture_scaling_factor[0].shr(16);
            let mut V =
                vertex_color.texture_coordinates[1] as u16 * self.texture_scaling_factor[1].shr(16);

            if self.geometry_mode & RSPGeometry::G_LIGHTING as u32 > 0 {
                if self.state_changed {
                    for i in 0..self.current_num_lights - 1 {
                        calculate_normal_dir(
                            &self.current_lights[i as usize],
                            &self.modelview_matrix[self.modelview_index as usize - 1],
                            &mut self.current_lights_coeffs[i as usize],
                        );
                    }

                    static LOOKAT_X: Light = Light::new([0, 0, 0], [0, 0, 0], [127, 0, 0]);
                    static LOOKAT_Y: Light = Light::new([0, 0, 0], [0, 0, 0], [0, 127, 0]);

                    calculate_normal_dir(
                        &LOOKAT_X,
                        &self.modelview_matrix[self.modelview_index as usize - 1],
                        &mut self.current_lookat_coeffs[0],
                    );

                    calculate_normal_dir(
                        &LOOKAT_Y,
                        &self.modelview_matrix[self.modelview_index as usize - 1],
                        &mut self.current_lookat_coeffs[1],
                    );

                    self.state_changed = false;
                }

                let mut r = self.current_lights[self.current_num_lights as usize - 1].col[0] as f32;
                let mut g = self.current_lights[self.current_num_lights as usize - 1].col[1] as f32;
                let mut b = self.current_lights[self.current_num_lights as usize - 1].col[2] as f32;

                for i in 0..self.current_num_lights - 1 {
                    let mut intensity = self.current_lights_coeffs[i as usize].dot(Vec3A::new(
                        vertex_normal.normal[0] as f32,
                        vertex_normal.normal[1] as f32,
                        vertex_normal.normal[2] as f32,
                    ));

                    intensity /= 127.0;

                    if intensity > 0.0 {
                        r += intensity * self.current_lights[i as usize].col[0] as f32;
                        g += intensity * self.current_lights[i as usize].col[1] as f32;
                        b += intensity * self.current_lights[i as usize].col[2] as f32;
                    }
                }

                staged_vertex.color[0] = if r > 255.0 { 255 } else { r as u8 };
                staged_vertex.color[1] = if g > 255.0 { 255 } else { g as u8 };
                staged_vertex.color[2] = if b > 255.0 { 255 } else { b as u8 };

                if self.geometry_mode & RSPGeometry::G_TEXTURE_GEN as u32 > 0 {
                    let dotx = self.current_lookat_coeffs[0 as usize].dot(Vec3A::new(
                        vertex_normal.normal[0] as f32,
                        vertex_normal.normal[1] as f32,
                        vertex_normal.normal[2] as f32,
                    ));

                    let doty = self.current_lookat_coeffs[1 as usize].dot(Vec3A::new(
                        vertex_normal.normal[0] as f32,
                        vertex_normal.normal[1] as f32,
                        vertex_normal.normal[2] as f32,
                    ));

                    U = ((dotx / 127.0 + 1.0) / 4.0) as u16 * self.texture_scaling_factor[0];
                    V = ((doty / 127.0 + 1.0) / 4.0) as u16 * self.texture_scaling_factor[1];
                }
            } else {
                staged_vertex.color[0] = vertex_color.color[0];
                staged_vertex.color[1] = vertex_color.color[1];
                staged_vertex.color[2] = vertex_color.color[2];
            }

            staged_vertex.tex_coord[0] = U as f32;
            staged_vertex.tex_coord[1] = V as f32;

            // trivial clip rejection
            staged_vertex.clip_rejection = 0;
            if x < -w {
                staged_vertex.clip_rejection |= 1;
            }
            if x > w {
                staged_vertex.clip_rejection |= 2;
            }
            if y < -w {
                staged_vertex.clip_rejection |= 4;
            }
            if y > w {
                staged_vertex.clip_rejection |= 8;
            }
            if z < -w {
                staged_vertex.clip_rejection |= 16;
            }
            if z > w {
                staged_vertex.clip_rejection |= 32;
            }

            staged_vertex.pos[0] = x;
            staged_vertex.pos[1] = y;
            staged_vertex.pos[2] = z;
            staged_vertex.pos[3] = w;

            if self.geometry_mode & RSPGeometry::G_FOG as u32 > 0 {
                let w = if w.abs() < 0.001 { 0.001 } else { w };

                let winv = 1.0 / w;
                let winv = if winv < 0.0 { 32767.0 } else { winv };

                let fog = z * winv * self.fog_multiplier as f32 + self.fog_offset as f32;
                let fog = if fog < 0.0 { 0.0 } else { fog };
                let fog = if fog > 255.0 { 255.0 } else { fog };

                staged_vertex.color[3] = fog as u8;
            } else {
                staged_vertex.color[3] = vertex_color.color[3];
            }

            dest_index += 1;
        }
    }

    pub fn gsp_triangles(&self, vertex_id1: u8, vertex_id2: u8, vertex_id3: u8) {
        let v1 = self.vertex_table[vertex_id1 as usize];
        let v2 = self.vertex_table[vertex_id2 as usize];
        let v3 = self.vertex_table[vertex_id3 as usize];
        let vertex_array = [v1, v2, v3];

        if (v1.clip_rejection & v2.clip_rejection & v3.clip_rejection) > 0 {
            // ...whole tri is offscreen, cull.
            return;
        }

        if (self.geometry_mode & RSPGeometry::G_CULL_BOTH as u32) > 0 {
            let dx1 = v1.pos[0] / v1.pos[3] - v2.pos[0] / v2.pos[3];
            let dy1 = v1.pos[1] / v1.pos[3] - v2.pos[1] / v2.pos[3];
            let dx2 = v3.pos[0] / v3.pos[3] - v2.pos[0] / v2.pos[3];
            let dy2 = v3.pos[1] / v3.pos[3] - v2.pos[1] / v2.pos[3];
            let mut cross = dx1 * dy2 - dy1 * dx2;

            // If any verts are past any clipping plane..
            if (v1.pos[3] < 0.0) ^ (v2.pos[3] < 0.0) ^ (v3.pos[3] < 0.0) {
                // If one vertex lies behind the eye, negating cross will give the correct result.
                // If all vertices lie behind the eye, the triangle will be rejected anyway.
                cross = -cross;
            }

            if (self.geometry_mode & RSPGeometry::G_CULL_BOTH as u32)
                == RSPGeometry::G_CULL_FRONT as u32
            {
                if cross < 0.0 {
                    return;
                }
            } else if (self.geometry_mode & RSPGeometry::G_CULL_BOTH as u32)
                == RSPGeometry::G_CULL_BACK as u32
            {
                if cross > 0.0 {
                    return;
                }
            } else {
                // TODO: Safe to ignore?
                return;
            }
        }

        // TODO: Produce draw calls for RDP to process later?
    }

    // MARK: - Helpers
}

fn calculate_normal_dir(light: &Light, matrix: &Mat4, coeffs: &mut Vec3A) {
    let light_dir = Vec3A::new(
        light.dir[0] as f32 / 127.0,
        light.dir[1] as f32 / 127.0,
        light.dir[2] as f32 / 127.0,
    );

    // transmpose and multiply by light dir
    coeffs[0] = matrix.col(0).xyz().dot(light_dir.into());
    coeffs[1] = matrix.col(1).xyz().dot(light_dir.into());
    coeffs[2] = matrix.col(2).xyz().dot(light_dir.into());

    // normalize coeffs
    normalize_vector(coeffs);
}

fn normalize_vector(vector: &mut Vec3A) {
    let magnitude = vector.length();
    vector[0] /= magnitude;
    vector[1] /= magnitude;
    vector[2] /= magnitude;
}
