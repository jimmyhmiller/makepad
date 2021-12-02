use {
    makepad_live_compiler::*,
    makepad_shader_compiler::*,
    crate::{
        cx::{Cx, CxPlatformShader},
        area::Area,
        geometry::{Geometry,GeometryFields},
        livetraits::*
    },
    
};


#[derive(Copy, Clone, PartialEq, Debug)]
pub struct DrawShader {
    pub draw_shader_id: usize,
    pub draw_shader_ptr: DrawShaderPtr
}

pub enum ShaderCompileResult {
    Nop,
    Ok
}

#[derive(Clone)]
pub struct DrawShaderInput {
    pub id: LiveId,
    pub ty: ShaderTy,
    pub offset: usize,
    pub slots: usize,
    pub value_ptr: Option<ValuePtr>
}

#[derive(Clone, Copy)]
pub enum DrawShaderInputPacking {
    Attribute,
    UniformsGLSL,
    UniformsHLSL,
    UniformsMetal
}

#[derive(Debug, PartialEq)]
pub struct DrawShaderFingerprint {
    fingerprint: Vec<LiveNode>,
    draw_shader_id: usize
}

#[derive(Clone)]
pub struct DrawShaderInputs {
    pub inputs: Vec<DrawShaderInput>,
    pub packing_method: DrawShaderInputPacking,
    pub total_slots: usize,
}

pub const DRAW_CALL_USER_UNIFORMS: usize = 16;
pub const DRAW_CALL_TEXTURE_SLOTS: usize = 4;
pub const DRAW_CALL_VAR_INSTANCES: usize = 16;

#[cfg(any(target_os = "linux", target_arch = "wasm32", test))]
pub const DRAW_SHADER_INPUT_PACKING: DrawShaderInputPacking = DrawShaderInputPacking::UniformGLSL;
#[cfg(any(target_os = "macos", test))]
pub const DRAW_SHADER_INPUT_PACKING: DrawShaderInputPacking = DrawShaderInputPacking::UniformsMetal;
#[cfg(any(target_os = "windows", test))]
pub const DRAW_SHADER_INPUT_PACKING: DrawShaderInputPacking = DrawShaderInputPacking::UniformsHLSL;

#[derive(Default, Debug)]
pub struct DrawVars {
    pub area: Area,
    pub draw_call_group: LiveId,
    pub var_instance_start: usize,
    pub var_instance_slots: usize,
    pub draw_shader: Option<DrawShader>,
    pub geometry_id: Option<usize>,
    pub user_uniforms: [f32; DRAW_CALL_USER_UNIFORMS],
    pub texture_slots: [Option<usize>; DRAW_CALL_TEXTURE_SLOTS],
    pub var_instances: [f32; DRAW_CALL_VAR_INSTANCES]
}

impl DrawVars {
    
    pub fn can_instance(&self) -> bool {
        self.draw_shader.is_some()
    }
    
    pub fn redraw_view(&self, cx: &mut Cx) {
        cx.redraw_view_of(self.area);
    }
    
    pub fn live_type() -> LiveType {
        LiveType(std::any::TypeId::of::<DrawVars>())
    }
    
    pub fn live_type_info() -> LiveTypeInfo {
        LiveTypeInfo {
            module_id: LiveModuleId::from_str(&module_path!()).unwrap(),
            live_type: Self::live_type(),
            fields: Vec::new(),
            kind: LiveTypeKind::DrawVars,
            type_name: LiveId::from_str("DrawVars").unwrap()
        }
    }
    
    pub fn live_register(_cx: &mut Cx) {}
    
    pub fn as_slice<'a>(&'a self) -> &'a [f32] {
        unsafe {
            std::slice::from_raw_parts((&self.var_instances[self.var_instance_start - 1] as *const _ as *const f32).offset(1), self.var_instance_slots)
        }
    }
    
    pub fn init_shader(&mut self, cx: &mut Cx, draw_shader_ptr: DrawShaderPtr, geometry_fields: &dyn GeometryFields) {
        
        if let Some(draw_shader_id) = cx.draw_shader_ptr_to_id.get(&draw_shader_ptr) {
            self.draw_shader = Some(DrawShader {
                draw_shader_ptr,
                draw_shader_id: *draw_shader_id
            });
            //self.geometry_id = geometry_fields.get_geometry_id();
        }
        else {
            let live_registry_cp = cx.live_registry.clone();
            let live_registry = live_registry_cp.borrow();
            
            let doc = live_registry.ptr_to_doc(draw_shader_ptr.0);
            
            // create a fingerprint from all the dsl nodes only
            let mut node_iter = doc.nodes.first_child(draw_shader_ptr.node_index());
            let mut fingerprint = Vec::new();
            while let Some(node_index) = node_iter {
                let node = &doc.nodes[node_index];
                if node.value.is_dsl() {
                    fingerprint.push(node.clone());
                }
                if node.id == id!(draw_call_group){
                    if let LiveValue::Id(id) = node.value{
                        self.draw_call_group = id;
                    }
                }
                node_iter = doc.nodes.next_child(node_index);
            }
            
            // see if we have it already
            for fp in &cx.draw_shader_fingerprints {
                if fp.fingerprint == fingerprint {
                    self.draw_shader = Some(DrawShader {
                        draw_shader_ptr,
                        draw_shader_id: fp.draw_shader_id
                    });
                    //self.geometry_id = geometry_fields.get_geometry_id();
                    return;
                }
            }
            
            fn live_type_to_shader_ty(live_type: LiveType) -> Option<ShaderTy> {
                if live_type == f32::live_type() {Some(ShaderTy::Float)}
                else if live_type == Vec2::live_type() {Some(ShaderTy::Vec2)}
                else if live_type == Vec3::live_type() {Some(ShaderTy::Vec3)}
                else if live_type == Vec4::live_type() {Some(ShaderTy::Vec4)}
                else {None}
            }
            // ok ! we have to compile it
            //let live_factories = &cx.live_factories;
            let result = cx.shader_registry.analyse_draw_shader(&live_registry, draw_shader_ptr, | live_registry, shader_registry, span, draw_shader_query, live_type, draw_shader_def | {
                match draw_shader_query {
                    DrawShaderQuery::DrawShader => {
                        fn recur_expand(
                            live_registry: &LiveRegistry,
                            shader_registry: &ShaderRegistry,
                            level: usize,
                            after_draw_vars: &mut bool,
                            live_type: LiveType,
                            draw_shader_def: &mut DrawShaderDef,
                            span: Span
                        ) {
                            if let Some(lf) = live_registry.live_type_infos.get(&live_type) {
                                
                                let mut slots = 0;
                                for field in &lf.fields {
                                    if field.id == id!(deref_target) {
                                        recur_expand(live_registry, shader_registry, level + 1, after_draw_vars, field.live_type_info.live_type, draw_shader_def, span);
                                        continue
                                    }
                                    if field.id == id!(draw_vars) {
                                        // assert the thing to be marked correctly
                                        if let LiveFieldKind::Calc = field.live_field_kind {}
                                        else {panic!()}
                                        if field.live_type_info.live_type != DrawVars::live_type() {panic!();}
                                        
                                        *after_draw_vars = true;
                                        continue;
                                    }
                                    if *after_draw_vars {
                                        // lets count sizes
                                        //
                                        let live_type = field.live_type_info.live_type;
                                        if shader_registry.enums.get(&live_type).is_some() {
                                            slots += 1;
                                            //draw_shader_def.enums
                                            
                                            draw_shader_def.add_instance(field.id, ShaderTy::Enum(live_type), span, field.live_field_kind);
                                        }
                                        else {
                                            let ty = live_type_to_shader_ty(live_type).expect("Please only put shader-understandable instance fields after draw_vars");
                                            slots += ty.slots();
                                            draw_shader_def.add_instance(field.id, ty, span, field.live_field_kind);
                                        }
                                    }
                                }
                                // insert padding
                                if level >0 && slots % 2 == 1 {
                                    draw_shader_def.add_instance(LiveId(0), ShaderTy::Float, span, LiveFieldKind::Calc);
                                }
                            }
                        }
                        recur_expand(live_registry, shader_registry, 0, &mut false, live_type, draw_shader_def, span);
                    }
                    DrawShaderQuery::Geometry => {
                        if live_type == geometry_fields.live_type_check() {
                            let mut fields = Vec::new();
                            geometry_fields.geometry_fields(&mut fields);
                            for field in fields {
                                draw_shader_def.add_geometry(field.id, field.ty, span);
                            }
                        }
                        else {
                            eprintln!("lf.get_type() != geometry_fields.live_type_check()");
                        }
                    }
                }
            });
            // ok lets print an error
            match result {
                Err(e) => {
                    // ok so. lets get the source for this file id
                    let file = &cx.live_registry.borrow().live_files[e.span.file_id().to_index()];
                    //println!("{}", file.source);
                    println!("Error {}", e.to_live_file_error(&file.file_name, &file.source, file.line_offset));
                }
                Ok(draw_shader_def) => {
                    // OK! SO the shader parsed
                    let draw_shader_id = cx.draw_shaders.len();
                    
                    let const_table = DrawShaderConstTable::default();
                    //let mut const_table = cx.shader_registry.compute_const_table(&draw_shader_def, NONE)
                    
                    let mut mapping = CxDrawShaderMapping::from_draw_shader_def(
                        draw_shader_def,
                        const_table,
                        DRAW_SHADER_INPUT_PACKING
                    );
                    mapping.update_live_uniforms(&cx.live_registry.borrow());
                    
                    let live_registry_rc = cx.live_registry.clone();
                    let live_registry = live_registry_rc.borrow();
                    let class_node = live_registry.ptr_to_node(draw_shader_ptr.0);
                    
                    let shader_type_name = match &class_node.value {
                        LiveValue::Class {live_type, ..} => {
                            // lets get the type name
                            let lti = live_registry.live_type_infos.get(live_type).unwrap();
                            lti.type_name
                        }
                        _ => LiveId(0)
                    };
                    cx.draw_shader_fingerprints.push(DrawShaderFingerprint {
                        draw_shader_id,
                        fingerprint
                    });
                    cx.draw_shaders.push(CxDrawShader {
                        field: class_node.id,
                        type_name: shader_type_name,
                        platform: None,
                        mapping: mapping
                    });
                    // ok so. maybe we should fill the live_uniforms buffer?
                    
                    cx.draw_shader_ptr_to_id.insert(draw_shader_ptr, draw_shader_id);
                    cx.draw_shader_compile_set.insert(draw_shader_ptr);
                    // now we simply queue it somewhere somehow to compile.
                    self.draw_shader = Some(DrawShader {
                        draw_shader_id,
                        draw_shader_ptr
                    });
                    
                    // self.geometry_id = geometry_fields.get_geometry_id();
                    //println!("{:?}", self.geometry_id);
                    // also we should allocate it a Shader object
                }
            }
        }
    }
    
    pub fn update_vars_in_place(&mut self, cx: &mut Cx) {
        if let Some(draw_shader) = self.draw_shader {
            // ok now what.
            // we could iterate our uniform and instance props
            // call get_write_ref and write into it
            if let Some(inst) = self.area.valid_instance(cx) {
                let sh = &cx.draw_shaders[draw_shader.draw_shader_id];
                let cxview = &mut cx.views[inst.view_id];
                let draw_call = cxview.draw_items[inst.draw_item_id].draw_call.as_mut().unwrap();
                
                let repeat = inst.instance_count;
                let stride = sh.mapping.instances.total_slots;
                let instances = &mut draw_call.instances.as_mut().unwrap()[inst.instance_offset..];
                let inst_slice = self.as_slice();
                
                // lets iterate the /*
                for input in &sh.mapping.live_instances.inputs {
                    for j in 0..repeat {
                        for i in 0..input.slots {
                            instances[input.offset + i + j * stride] = inst_slice[input.offset + i]
                        }
                    }
                }
                for input in &sh.mapping.user_uniforms.inputs {
                    for i in 0..input.slots {
                        draw_call.user_uniforms[input.offset + i] = self.user_uniforms[input.offset + i]
                    }
                }
                // DONE!
                
                cx.passes[cxview.pass_id].paint_dirty = true;
                draw_call.instance_dirty = true;
                draw_call.uniforms_dirty = true;
            }
        }
    }
    
    pub fn init_slicer(
        &mut self,
        cx: &mut Cx,
    ) {
        if let Some(draw_shader) = self.draw_shader {
            let sh = &cx.draw_shaders[draw_shader.draw_shader_id];
            self.var_instance_start = self.var_instances.len() - sh.mapping.var_instances.total_slots;
            self.var_instance_slots = sh.mapping.instances.total_slots;
        }
    }
    
    pub fn before_apply(&mut self, cx: &mut Cx, apply_from: ApplyFrom, index: usize, _nodes: &[LiveNode], geometry_fields: &dyn GeometryFields) {
        if self.draw_shader.is_some() {
            
            return
        }
        let draw_shader_ptr = if let Some(file_id) = apply_from.file_id() {
            DrawShaderPtr(LivePtr::from_index(file_id, index))
        }
        else {
            
            return
        };
        self.init_shader(cx, draw_shader_ptr, geometry_fields)
    }
    
    pub fn apply_value(&mut self, cx: &mut Cx, apply_from: ApplyFrom, index: usize, nodes: &[LiveNode]) -> usize {
        fn apply_slots(cx: &mut Cx, slots: usize, output: &mut [f32], offset: usize, apply_from: ApplyFrom, index: usize, nodes: &[LiveNode]) -> usize {
            match slots {
                1 => {
                    let mut v: f32 = 0.0;
                    let index = v.apply(cx, apply_from, index, nodes);
                    output[offset + 0] = v;
                    return index;
                }
                2 => {
                    let mut v: Vec2 = Vec2::default();
                    let index = v.apply(cx, apply_from, index, nodes);
                    output[offset + 0] = v.x;
                    output[offset + 1] = v.y;
                    return index;
                }
                3 => {
                    let mut v: Vec3 = Vec3::default();
                    let index = v.apply(cx, apply_from, index, nodes);
                    output[offset + 0] = v.x;
                    output[offset + 1] = v.y;
                    output[offset + 2] = v.z;
                    return index;
                }
                4 => {
                    let mut v: Vec4 = Vec4::default();
                    let index = v.apply(cx, apply_from, index, nodes);
                    output[offset + 0] = v.x;
                    output[offset + 1] = v.y;
                    output[offset + 2] = v.z;
                    output[offset + 3] = v.w;
                    return index;
                }
                _ => {
                    return nodes.skip_node(index)
                }
            }
        }
        
        if let Some(draw_shader) = self.draw_shader {
            let id = nodes[index].id;
            let sh = &cx.draw_shaders[draw_shader.draw_shader_id];
            for input in &sh.mapping.user_uniforms.inputs {
                let offset = input.offset;
                let slots = input.slots;
                if input.id == id {
                    return apply_slots(cx, slots, &mut self.user_uniforms, offset, apply_from, index, nodes);
                }
            }
            for input in &sh.mapping.var_instances.inputs {
                let offset = (self.var_instances.len() - sh.mapping.var_instances.total_slots) + input.offset;
                let slots = input.slots;
                if input.id == id {
                    return apply_slots(cx, slots, &mut self.var_instances, offset, apply_from, index, nodes);
                }
            }
        }
        else{
            panic!("no shader applying {}", nodes[index].id);
        }
        let unknown_shader_props = match nodes[index].id {
            id!(debug) => false,
            _ => true
        };
        if unknown_shader_props && nodes[index].value.is_value_type() {
            cx.apply_error_no_matching_field(live_error_origin!(), apply_from, index, nodes);
        }
        nodes.skip_node(index)
    }
    
    pub fn after_apply(&mut self, cx: &mut Cx, apply_from: ApplyFrom, _index: usize, _nodes: &[LiveNode], geometry_fields: &dyn GeometryFields) {
        if apply_from.is_from_doc() {
            self.init_slicer(cx);
        }
        self.geometry_id = geometry_fields.get_geometry_id();
        self.update_vars_in_place(cx);
    }
    
}

impl DrawShaderInputs {
    pub fn new(packing_method: DrawShaderInputPacking) -> Self {
        Self {
            inputs: Vec::new(),
            packing_method,
            total_slots: 0
        }
    }
    
    pub fn push(&mut self, id: LiveId, ty: ShaderTy, value_ptr: Option<ValuePtr>) {
        let slots = ty.slots();
        match self.packing_method {
            DrawShaderInputPacking::Attribute => {
                self.inputs.push(DrawShaderInput {
                    id,
                    offset: self.total_slots,
                    slots,
                    ty,
                    value_ptr
                });
                self.total_slots += slots;
            }
            DrawShaderInputPacking::UniformsGLSL => {
                self.inputs.push(DrawShaderInput {
                    id,
                    offset: self.total_slots,
                    slots,
                    ty,
                    value_ptr
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
                    value_ptr
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
                    value_ptr
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
            live_uniforms.push(LiveId(0), ty.clone(), Some(*value_node_ptr));
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
                    let node = live_registry.ptr_to_node(input.value_ptr.unwrap().0);
                    if let LiveValue::Float(float) = node.value {
                        let o = input.offset;
                        self.live_uniforms_buf[o] = float as f32;
                        
                    }
                },
                2 => { // float
                    let node = live_registry.ptr_to_node(input.value_ptr.unwrap().0);
                    if let LiveValue::Vec2(value) = node.value {
                        let o = input.offset;
                        self.live_uniforms_buf[o + 0] = value.x;
                        self.live_uniforms_buf[o + 1] = value.y;
                    }
                },
                3 => { // float
                    let node = live_registry.ptr_to_node(input.value_ptr.unwrap().0);
                    if let LiveValue::Vec3(value) = node.value {
                        let o = input.offset;
                        self.live_uniforms_buf[o + 0] = value.x;
                        self.live_uniforms_buf[o + 1] = value.y;
                        self.live_uniforms_buf[o + 2] = value.z;
                    }
                },
                4 => { // color
                    let node = live_registry.ptr_to_node(input.value_ptr.unwrap().0);
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

#[derive(Clone)]
pub struct CxDrawShader {
    pub field: LiveId,
    pub type_name: LiveId,
    pub platform: Option<CxPlatformShader>,
    pub mapping: CxDrawShaderMapping
}