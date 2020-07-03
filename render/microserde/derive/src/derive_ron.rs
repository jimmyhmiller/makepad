 
use proc_macro::{TokenStream};
use crate::macro_lib::*;

pub fn derive_ser_ron_impl(input: TokenStream) -> TokenStream {

    let mut parser = TokenParser::new(input);
    let mut tb = TokenBuilder::new();
    
    parser.eat_ident("pub");
    if parser.eat_ident("struct"){
        if let Some(name) = parser.eat_any_ident(){
            
            let generic = parser.eat_generic();
            let types = parser.eat_all_types();
            let where_clause = parser.eat_where_clause(Some("SerRon"));

            tb.add("impl").stream(generic.clone());
            tb.add("SerRon for").ident(&name).stream(generic).stream(where_clause);
            tb.add("{ fn ser_ron ( & self , d : usize , s : & mut makepad_microserde :: SerRonState ) {");
            
            if let Some(types) = types{
                tb.add("s . out . push (").chr('[').add(") ;");
                for i in 0..types.len(){
                     tb.add("self .").unsuf_usize(i).add(". ser_json ( d , s ) ;");
                     if i != types.len() - 1{
                         tb.add("s . out . push (").chr(',').add(") ;");
                     }
                }
                tb.add("s . out . push (").chr(']').add(") ;");
            }
            else if let Some(fields) = parser.eat_all_struct_fields(){ 
                tb.add("s . st_pre ( ) ;");
                // named struct
                for (field,ty) in fields{
                    if ty.into_iter().next().unwrap().to_string() == "Option"{
                        tb.add("if let Some ( t ) = ").add("& self .").ident(&field).add("{");
                        tb.add("s . field ( d + 1 ,").string(&field).add(") ;");
                        tb.add("t . ser_json ( d + 1 , s ) ; s . conl ( ) ; } ;");
                    }
                    else{
                        tb.add("s . field ( d + 1 ,").string(&field).add(" ) ;");
                        tb.add("self .").ident(&field).add(". ser_json ( d + 1 , s ) ; s . conl ( ) ;");
                    }
                }
                tb.add("s . st_post ( d ) ;");
            }
            else{
                return parser.unexpected()
            }
            tb.add("} } ;");
            return tb.end();
        }
    }
    else if parser.eat_ident("enum"){
        if let Some(name) = parser.eat_any_ident(){
            let generic = parser.eat_generic();
            let where_clause = parser.eat_where_clause(Some("SerRon"));

            tb.add("impl").stream(generic.clone());
            tb.add("SerRon for").ident(&name).stream(generic).stream(where_clause);
            tb.add("{ fn ser_ron ( & self , d : usize , s : & mut makepad_microserde :: SerRonState ) {");
            tb.add("s . out . push (").chr('{').add(") ;");
            tb.add("match self {");
            
            if !parser.open_brace(){
                return parser.unexpected()
            }

            while !parser.eat_eot(){
                // parse ident
                if let Some(variant) = parser.eat_any_ident(){
                    if let Some(types) = parser.eat_all_types(){
                        
                        tb.add("Self ::").ident(&variant).add("(");
                        for i in 0..types.len(){
                            tb.ident(&format!("n{}", i)).add(",");
                        }
                        tb.add(") => {");
                        tb.add("s . label (").string(&variant).add(") ;");
                        tb.add("s . out . push (").chr(':').add(") ;");
                        tb.add("s . out . push (").chr('[').add(") ;");
                        
                        for i in 0..types.len(){
                            tb.ident(&format!("n{}", i)).add(". ser_ron ( d , s ) ;");
                            if i != types.len() - 1{
                                tb.add("s . out . push (").chr(',').add(") ;");
                            }
                        }
                        tb.add("s . out . push (").chr(']').add(") ;");
                        tb.add("}");
                    }
                    else if let Some(fields) = parser.eat_all_struct_fields(){ // named variant
                        tb.add("Self ::").ident(&variant).add("{");
                        for (field, _ty) in fields.iter(){
                            tb.ident(field).add(",");
                        }
                        tb.add("} => {");
                        
                        tb.add("s . label (").string(&variant).add(") ;");
                        tb.add("s . out . push (").chr(':').add(") ;");
                        tb.add("s . st_pre ( ) ;");
                        
                        for (field, ty) in fields{
                            if ty.into_iter().next().unwrap().to_string() == "Option"{
                                tb.add("if let Some ( t ) = ").ident(&field).add("{");
                                tb.add("s . field ( d + 1 ,").string(&field).add(") ;");
                                tb.add("t . ser_json ( d + 1 , s ) ; s . conl ( ) ; } ;");
                            }
                            else{
                                tb.add("s . field ( d + 1 ,").string(&field).add(" ) ;");
                                tb.ident(&field).add(". ser_json ( d + 1 , s ) ; s . conl ( ) ;");
                            }
                        }
                        tb.add("s . st_post ( d ) ; }");
                    }
                    else if parser.is_punct(',') || parser.is_eot(){ // bare variant
                        tb.add("Self ::").ident(&variant).add("=> {");
                        tb.add("s . label (").string(&variant).add(") ;");
                        tb.add("s . out . push_str (").string(":[]").add(") ; }");
                    }
                    else{
                        return parser.unexpected();
                    }
                    parser.eat_punct(',');
                }
                else{
                    return parser.unexpected()
                }
            }
            tb.add("}");
            tb.add("s . out . push (").chr('}').add(") ;");
            tb.add("} } ;");
            return tb.end();
        }
    }
    return parser.unexpected()
}

pub fn derive_de_ron_impl(input: TokenStream) -> TokenStream {
    let mut parser = TokenParser::new(input);
    let mut tb = TokenBuilder::new();
    
    parser.eat_ident("pub");
    if parser.eat_ident("struct"){
        if let Some(name) = parser.eat_any_ident(){
            let generic = parser.eat_generic();
            let types = parser.eat_all_types();
            let where_clause = parser.eat_where_clause(Some("DeJson"));

            tb.add("impl").stream(generic.clone());
            tb.add("DeJson for").ident(&name).stream(generic).stream(where_clause);
            tb.add("{ fn de_json ( s : &  mut makepad_microserde :: DeJsonState , i : & mut std :: str :: Chars )");
            tb.add("-> std :: result :: Result < Self , DeJsonErr > { ");

            if let Some(types) = types{
                tb.add("s . block_open ( i ) ? ;");
                tb.add("let r = Self");
                tb.add("(");
                for _ in 0..types.len(){
                     tb.add("{ let r = DeJson :: de_json ( s , i ) ? ; s . eat_comma_block ( i ) ? ; r } ,");
                }
                tb.add(") ;");
                tb.add("s . block_close ( i ) ? ;");
                tb.add("std :: result :: Result :: Ok ( r ) ;");
            }
            else if let Some(fields) = parser.eat_all_struct_fields(){ 
                tb.add("s . curly_open ( i ) ? ;");
                for (field,_ty) in &fields{
                    tb.add("let mut").ident(&format!("_{}",field)).add("= None ;");
                }
                tb.add("while let Some ( _ ) = s . next_str ( ) {");
                tb.add("match s . strbuf . as_ref ( ) {");
                for (field,_ty) in &fields{
                    tb.string(&field).add("=> { s . next_colon ( i ) ? ;");
                    tb.ident(&format!("_{}",field)).add("= Some ( DeJson :: de_json ( s , i ) ? ) ; } ,");
                }
                tb.add("_ => return std :: result :: Result :: Err ( s . err_exp ( & s . strbuf ) )");
                tb.add("} ; s . eat_comma_curly ( i ) ? ;");
                tb.add("} ; s . curly_close ( i ) ? ;");
                
                tb.add("std :: result :: Result :: Ok ( Self {");
                for (field,ty) in fields{
                    tb.ident(&field).add(":");
                    if ty.into_iter().next().unwrap().to_string() == "Option"{
                        tb.add("if let Some ( t ) =").ident(&format!("_{}",field));
                        tb.add("{ t } else { None } ,");
                    }
                    else{
                        tb.add("if let Some ( t ) =").ident(&format!("_{}",field));
                        tb.add("{ t } else { return Err ( s . err_nf (");
                        tb.string(&field).add(") ) } ,");
                    }
                }
                tb.add("} )");
            }
            else{
                return parser.unexpected()
            }
            tb.add("} } ;"); 
            return tb.end();
        }
    }
    else if parser.eat_ident("enum"){
        
        if let Some(name) = parser.eat_any_ident(){
            let generic = parser.eat_generic();
            let where_clause = parser.eat_where_clause(Some("DeJson"));

            tb.add("impl").stream(generic.clone());
            tb.add("DeJson for").ident(&name).stream(generic).stream(where_clause);
            tb.add("{ fn de_json ( s : & mut makepad_microserde :: DeJsonState , i : & mut std :: str :: Chars )");
            tb.add("-> std :: result :: Result < Self , DeJsonErr > { ");
            tb.add("s . curly_open ( i ) ? ;");
            tb.add("let _ = s . string ( i ) ? ;");
            tb.add("s . colon ( i ) ? ;");
            tb.add("let r = std :: result :: Result :: Ok ( match s . strbuf . as_ref ( ) {");
            
            if !parser.open_brace(){
                return parser.unexpected()
            }
            while !parser.eat_eot(){
                // parse ident
                if let Some(variant) = parser.eat_any_ident(){
                    tb.string(&variant).add("=> {");
                    if let Some(types) = parser.eat_all_types(){
                        
                        tb.add("s . block_open ( i ) ? ;");
                        tb.add("let r = Self ::").ident(&variant).add("(");
                        for _ in 0..types.len(){
                            tb.add("{ let r = DeJson :: de_json ( s , i ) ? ; s . eat_comma_block ( i ) ? ; r } ,");
                        }
                        tb.add(") ;");
                        tb.add("s . block_close ( i ) ? ; r");
                    }
                    else if let Some(fields) = parser.eat_all_struct_fields(){ // named variant
                        tb.add("s . curly_open ( i ) ? ;");
                        for (field,_ty) in &fields{
                            tb.add("let mut").ident(&format!("_{}",field)).add("= None ;");
                        }
                        tb.add("while let Some ( _ ) = s . next_str ( ) {");
                        tb.add("match s . strbuf . as_ref ( ) {");
                        for (field,_ty) in &fields{
                            tb.string(&field).add("=> { s . next_colon ( i ) ? ;");
                            tb.ident(&format!("_{}",field)).add("= Some ( DeJson :: de_json ( s , i ) ? ) ; } ,");
                        }
                        tb.add("_ => return std :: result :: Result :: Err ( s . err_exp ( & s . strbuf ) )");
                        tb.add("} s . eat_comma_curly ( i ) ? ;");
                        tb.add("} s . curly_close ( i ) ? ;");
                        
                        tb.add("Self ::").ident(&variant).add("{");
                        for (field,ty) in fields{
                            tb.ident(&field).add(":");
                            if ty.into_iter().next().unwrap().to_string() == "Option"{
                                tb.add("if let Some ( t ) =").ident(&format!("_{}",field));
                                tb.add("{ t } else { None } ,");
                            }
                            else{
                                tb.add("if let Some ( t ) =").ident(&format!("_{}",field));
                                tb.add("{ t } else { return Err ( s . err_nf (");
                                tb.string(&field).add(") ) } ,");
                            }
                        }
                        tb.add("}");
                    }
                    else if parser.is_punct(',') || parser.is_eot(){ // bare variant
                        tb.add("s . block_open ( i ) ? ; s . block_close ( i ) ? ; Self ::").ident(&variant);
                    }
                    else{
                        return parser.unexpected();
                    }
                    
                    tb.add("}");
                    parser.eat_punct(',');
                }
                else{
                    return parser.unexpected()
                }
            } 
            tb.add("_ => return std :: result :: Result :: Err ( s . err_exp ( & s . strbuf ) )");
            tb.add("} ) ; s . curly_close ( i ) ? ; r } }");
            return tb.end();
        }
    }
    return parser.unexpected()
}