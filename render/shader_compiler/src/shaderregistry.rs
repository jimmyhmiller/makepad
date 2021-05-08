#![allow(unused_variables)]
use makepad_live_parser::LiveRegistry;
use makepad_live_parser::Id;
use makepad_live_parser::LiveError;
use makepad_live_parser::LiveValue;
use makepad_live_parser::Span;
//use makepad_live_parser::id_pack;
//use makepad_live_parser::IdPack;
use makepad_live_parser::FullNodePtr;
use crate::shaderast::ShaderAst;
use crate::shaderast::StructDecl;
use crate::shaderparser::ShaderParser;
use std::collections::HashMap;

#[derive(Default, Debug)]
pub struct ShaderRegistry {
    pub live_registry: LiveRegistry,
    pub structs: HashMap<FullNodePtr, StructDecl>
}

impl ShaderRegistry {
    
    
    // lets compile the thing
    pub fn compile_draw_shader(&self, crate_id: Id, module_id: Id, ids: &[Id]) -> Result<(), LiveError> {
        // lets find the FullPointer
        
        if let Some(full_ptr) = self.live_registry.find_full_node_ptr(crate_id, module_id, ids) {
            let shast = ShaderAst::default();
            // we have a pointer to the thing to instance.
            let doc = &self.live_registry.expanded[full_ptr.file_id.to_index()];
            let class_node = &doc.nodes[full_ptr.local_ptr.level][full_ptr.local_ptr.index];
            
            match class_node.value {
                LiveValue::Class {node_start, node_count, ..} => {
                    let mut type_deps = Vec::new();
                    for i in 0..node_count as usize {
                        let prop = &doc.nodes[full_ptr.local_ptr.level + 1][i + node_start as usize];
                        match &prop.value {
                            LiveValue::VarDef {token_start, token_count, scope_start, scope_count} => {
                                //let parser = ShaderParser::new(&doc.tokens[*token_start as usize..(token_start + token_count)as usize]);
                                //let decl = parser.expect_decl()?;
                                
                            },
                            LiveValue::Fn {token_start, token_count, scope_start, scope_count} => {
                                // lets parse this thing
                                let mut parser = ShaderParser::new(
                                    &doc.tokens[*token_start as usize..(token_start + token_count)as usize],
                                    &doc.scopes[*scope_start as usize..(*scope_start + *scope_count as  u32) as usize],
                                    &mut type_deps
                                );
                                let decl = parser.expect_fn_decl(None)?;
                                /*
                                match prop.id_pack {
                                    id_pack!(vertex) => {
                                    },
                                    id_pack!(pixel) => {
                                        //let parser = ShaderParser::new(&doc.tokens[*token_start as usize..(token_start + token_count )as usize]);
                                        //let decl = parser.expect_decl()?;
                                    }
                                    _ => ()
                                }*/
                                // lets resolve the structs and recur-analyse them
                                
                            }
                            _ => ()
                        }
                    }
                    println!("HERE {}", type_deps.len());
                }
                _ => {
                    return Err(LiveError {
                        span: Span::default(),
                        message: format!("Compile shader could not find {} {} {} ", crate_id, module_id, ids[0])
                    })
                }
            }
            // we have collected all the custom struct types as a set of pointers.
            // now we need to
            
            // we need to run over the thing
            
            // alright we collected all the decls and entry points
            // now we go and run our analyser
            // this will in turn on-demand parse other structs and functions as needed
            
            // and do all the declarations
            
            // ok
            
            // then we need to find vertex/pixel
            
            // parse them
            
            // then analyse vertex first, recur expand parse decls
            
            // then analyse pixel, recur expand parse
        }
        return Ok(())
    }
    
    
}