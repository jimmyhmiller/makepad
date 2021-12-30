use {
    std::{
        ops::{Index, IndexMut},
        collections::{
            HashMap,
            HashSet,
            BTreeSet,
        },
    },
    makepad_shader_compiler::makepad_live_compiler::*,
    makepad_shader_compiler::*,
    crate::{
        platform::{
            CxPlatformDrawShader,
        },
        cx::Cx
    }
};

#[derive(Default)]
pub struct CxDrawShaders{
    pub shaders: Vec<CxDrawShader>,
    pub platform: Vec<CxPlatformDrawShader>,
    pub generation: u64,
    pub ptr_to_id: HashMap<DrawShaderPtr, usize>,
    pub compile_set: BTreeSet<DrawShaderPtr>,
    pub fingerprints: Vec<DrawShaderFingerprint>,
    pub error_set: HashSet<DrawShaderPtr>,
    pub error_fingerprints: Vec<Vec<LiveNode>>,
} 

impl Cx{
    pub fn flush_draw_shaders(&mut self){
        self.draw_shaders.generation += 1;
        self.shader_registry.flush_registry();
        self.draw_shaders.shaders.clear();
        self.draw_shaders.ptr_to_id.clear();
        self.draw_shaders.fingerprints.clear();
        self.draw_shaders.error_set.clear();
        self.draw_shaders.error_fingerprints.clear();
    }
}

impl Index<usize> for CxDrawShaders {
    type Output = CxDrawShader;
    fn index(&self, index: usize) -> &Self::Output {
        &self.shaders[index]
    }
}

impl IndexMut<usize> for CxDrawShaders {
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        &mut self.shaders[index]
    }
}

#[derive(Copy, Clone, PartialEq, Debug)]
pub struct DrawShader {
    pub draw_shader_generation: u64,
    pub draw_shader_id: usize,
    pub draw_shader_ptr: DrawShaderPtr
}

pub struct CxDrawShader {
    pub field: LiveId,
    pub type_name: LiveId,
    pub platform: Option<usize>,
    pub mapping: CxDrawShaderMapping
}

#[derive(Debug, PartialEq)]
pub struct DrawShaderFingerprint {
    pub fingerprint: Vec<LiveNode>,
    pub draw_shader_id: usize
}

#[derive(Clone, Debug)]
pub struct DrawShaderInputs {
    pub inputs: Vec<DrawShaderInput>,
    pub packing_method: DrawShaderInputPacking,
    pub total_slots: usize,
}

#[derive(Clone, Copy, Debug)]
pub enum DrawShaderInputPacking {
    Attribute,
    UniformsGLSL,
    UniformsHLSL,
    UniformsMetal
}


#[derive(Clone, Debug)]
pub struct DrawShaderInput {
    pub id: LiveId,
    pub ty: ShaderTy,
    pub offset: usize,
    pub slots: usize,
    pub live_ptr: Option<LivePtr>
}


#[cfg(any(target_os = "linux", target_arch = "wasm32", test))]
pub const DRAW_SHADER_INPUT_PACKING: DrawShaderInputPacking = DrawShaderInputPacking::UniformGLSL;
#[cfg(any(target_os = "macos", test))]
pub const DRAW_SHADER_INPUT_PACKING: DrawShaderInputPacking = DrawShaderInputPacking::UniformsMetal;
#[cfg(any(target_os = "windows", test))]
pub const DRAW_SHADER_INPUT_PACKING: DrawShaderInputPacking = DrawShaderInputPacking::UniformsHLSL;

impl DrawShaderInputs {
    pub fn new(packing_method: DrawShaderInputPacking) -> Self {
        Self {
            inputs: Vec::new(),
            packing_method,
            total_slots: 0
        }
    }
    
    pub fn push(&mut self, id: LiveId, ty: ShaderTy, live_ptr: Option<LivePtr>) {
        let slots = ty.slots();
        match self.packing_method {
            DrawShaderInputPacking::Attribute => {
                self.inputs.push(DrawShaderInput {
                    id,
                    offset: self.total_slots,
                    slots,
                    ty,
                    live_ptr
                });
                self.total_slots += slots;
            }
            DrawShaderInputPacking::UniformsGLSL => {
                self.inputs.push(DrawShaderInput {
                    id,
                    offset: self.total_slots,
                    slots,
                    ty,
                    live_ptr
                });
                self.total_slots += slots;
            }
            DrawShaderInputPacking::UniformsHLSL => {
                if (self.total_slots & 3) + slots > 4 { // goes over the boundary
                    self.total_slots += 4 - (self.total_slots & 3); // make jump to new slot
                }
                self.inputs.push(DrawShaderInput {
                    id,
                    offset: self.total_slots,
                    slots,
                    ty,
                    live_ptr
                });
                self.total_slots += slots;
            }
            DrawShaderInputPacking::UniformsMetal => {
                let aligned_slots = if slots == 3 {4} else {slots};
                if (self.total_slots & 3) + aligned_slots > 4 { // goes over the boundary
                    self.total_slots += 4 - (self.total_slots & 3); // make jump to new slot
                }
                self.inputs.push(DrawShaderInput {
                    id,
                    offset: self.total_slots,
                    slots,
                    ty,
                    live_ptr
                });
                self.total_slots += aligned_slots;
            }
        }
    }
    
    pub fn finalize(&mut self) {
        match self.packing_method {
            DrawShaderInputPacking::Attribute => (),
            DrawShaderInputPacking::UniformsGLSL |
            DrawShaderInputPacking::UniformsHLSL |
            DrawShaderInputPacking::UniformsMetal => {
                if self.total_slots & 3 > 0 {
                    self.total_slots += 4 - (self.total_slots & 3);
                }
            }
        }
    }
}

#[derive(Clone)]
pub struct DrawShaderTextureInput {
    id: LiveId,
    ty: ShaderTy
}

#[derive(Clone)]
pub struct CxDrawShaderMapping {
    pub flags: DrawShaderFlags,
    pub const_table: DrawShaderConstTable,
    
    pub geometries: DrawShaderInputs,
    pub instances: DrawShaderInputs,
    pub var_instances: DrawShaderInputs,
    pub live_instances: DrawShaderInputs,
    pub live_uniforms: DrawShaderInputs,
    pub user_uniforms: DrawShaderInputs,
    pub draw_uniforms: DrawShaderInputs,
    pub view_uniforms: DrawShaderInputs,
    pub pass_uniforms: DrawShaderInputs,
    pub textures: Vec<DrawShaderTextureInput>,
    pub rect_pos: Option<usize>,
    pub rect_size: Option<usize>,
    pub live_uniforms_buf: Vec<f32>
}

impl CxDrawShaderMapping {
    
    pub fn from_draw_shader_def(draw_shader_def: &DrawShaderDef, const_table: DrawShaderConstTable, uniform_packing: DrawShaderInputPacking) -> CxDrawShaderMapping { //}, options: ShaderCompileOptions, metal_uniform_packing:bool) -> Self {
        
        let mut geometries = DrawShaderInputs::new(DrawShaderInputPacking::Attribute);
        let mut instances = DrawShaderInputs::new(DrawShaderInputPacking::Attribute);
        let mut var_instances = DrawShaderInputs::new(DrawShaderInputPacking::Attribute);
        let mut live_instances = DrawShaderInputs::new(DrawShaderInputPacking::Attribute);
        let mut user_uniforms = DrawShaderInputs::new(uniform_packing);
        let mut live_uniforms = DrawShaderInputs::new(uniform_packing);
        let mut draw_uniforms = DrawShaderInputs::new(uniform_packing);
        let mut view_uniforms = DrawShaderInputs::new(uniform_packing);
        let mut pass_uniforms = DrawShaderInputs::new(uniform_packing);
        let mut textures = Vec::new();
        let mut rect_pos = None;
        let mut rect_size = None;
        
        for field in &draw_shader_def.fields {
            let ty = field.ty_expr.ty.borrow().as_ref().unwrap().clone();
            match &field.kind {
                DrawShaderFieldKind::Geometry {..} => {
                    geometries.push(field.ident.0, ty, None);
                }
                DrawShaderFieldKind::Instance {var_def_ptr, live_field_kind, ..} => {
                    if field.ident.0 == id!(rect_pos) {
                        rect_pos = Some(instances.total_slots);
                    }
                    if field.ident.0 == id!(rect_size) {
                        rect_size = Some(instances.total_slots);
                    }
                    if var_def_ptr.is_some() {
                        var_instances.push(field.ident.0, ty.clone(), None,);
                    }
                    instances.push(field.ident.0, ty, None);
                    if let LiveFieldKind::Live = live_field_kind {
                        live_instances.inputs.push(instances.inputs.last().unwrap().clone());
                    }
                }
                DrawShaderFieldKind::Uniform {block_ident, ..} => {
                    match block_ident.0 {
                        id!(draw) => {
                            draw_uniforms.push(field.ident.0, ty, None);
                        }
                        id!(view) => {
                            view_uniforms.push(field.ident.0, ty, None);
                        }
                        id!(pass) => {
                            pass_uniforms.push(field.ident.0, ty, None);
                        }
                        id!(user) => {
                            user_uniforms.push(field.ident.0, ty, None);
                        }
                        _ => ()
                    }
                }
                DrawShaderFieldKind::Texture {..} => {
                    textures.push(DrawShaderTextureInput {
                        ty,
                        id: field.ident.0,
                    });
                }
                _ => ()
            }
        }
        
        geometries.finalize();
        instances.finalize();
        var_instances.finalize();
        user_uniforms.finalize();
        live_uniforms.finalize();
        draw_uniforms.finalize();
        view_uniforms.finalize();
        pass_uniforms.finalize();
        
        // ok now the live uniforms
        for (value_node_ptr, ty) in draw_shader_def.all_live_refs.borrow().iter() {
            live_uniforms.push(LiveId(0), ty.clone(), Some(value_node_ptr.0));
        }
        
        CxDrawShaderMapping {
            const_table,
            flags: draw_shader_def.flags,
            geometries,
            instances,
            live_uniforms_buf: {let mut r = Vec::new(); r.resize(live_uniforms.total_slots, 0.0); r},
            var_instances,
            live_instances,
            user_uniforms,
            live_uniforms,
            draw_uniforms,
            view_uniforms,
            pass_uniforms,
            textures,
            rect_pos,
            rect_size,
        }
    }
    
    pub fn update_live_uniforms(&mut self, live_registry: &LiveRegistry) {
        // and write em into the live_uniforms buffer
        for input in &self.live_uniforms.inputs {
            match input.slots {
                1 => { // float
                    let node = live_registry.ptr_to_node(input.live_ptr.unwrap());
                    if let LiveValue::Float(float) = node.value {
                        let o = input.offset;
                        self.live_uniforms_buf[o] = float as f32;
                    }
                },
                2 => { // float
                    let node = live_registry.ptr_to_node(input.live_ptr.unwrap());
                    if let LiveValue::Vec2(value) = node.value {
                        let o = input.offset;
                        self.live_uniforms_buf[o + 0] = value.x;
                        self.live_uniforms_buf[o + 1] = value.y;
                    }
                },
                3 => { // float
                    let node = live_registry.ptr_to_node(input.live_ptr.unwrap());
                    if let LiveValue::Vec3(value) = node.value {
                        let o = input.offset;
                        self.live_uniforms_buf[o + 0] = value.x;
                        self.live_uniforms_buf[o + 1] = value.y;
                        self.live_uniforms_buf[o + 2] = value.z;
                    }
                },
                4 => { // color
                    let node = live_registry.ptr_to_node(input.live_ptr.unwrap());
                    if let LiveValue::Color(color_u32) = node.value {
                        let o = input.offset;
                        let color = Vec4::from_u32(color_u32);
                        self.live_uniforms_buf[o + 0] = color.x;
                        self.live_uniforms_buf[o + 1] = color.y;
                        self.live_uniforms_buf[o + 2] = color.z;
                        self.live_uniforms_buf[o + 3] = color.w;
                    }
                },
                _ => panic!()
            }
        }
    }
}