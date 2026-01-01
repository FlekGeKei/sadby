extern crate proc_macro;

use proc_macro::TokenStream;
use proc_macro2::Span;
use proc_macro2::TokenStream as TokenStream2;
use quote::ToTokens;
use quote::quote;
use syn::DeriveInput;
use syn::Error;
use syn::Expr;
use syn::ExprLit;
use syn::Ident;
use syn::Lit;
use syn::spanned::Spanned;

#[proc_macro_derive(Sadaby)]
pub fn sadb_derive(input: TokenStream) -> TokenStream {
    match sadb_macro(input.into()) {
        Ok(o) => o,
        Err(e) => e.to_compile_error(),
    }
    .into()
}

fn sadb_macro(input: proc_macro2::TokenStream) -> syn::Result<proc_macro2::TokenStream> {
    let ast: DeriveInput = syn::parse2(input)?;

    let ident = &ast.ident;
    let (impl_generics, type_generics, where_clause) = ast.generics.split_for_impl();

    let (complete_tokens_to, complete_tokens_from): (TokenStream2, TokenStream2) = match &ast.data {
        syn::Data::Enum(e) => {
            let Some(repr) = ast.attrs.iter().find(|a| {
                if a.style == syn::AttrStyle::Outer
                    && let syn::Meta::List(l) = &a.meta
                {
                    l.path.is_ident("repr")
                } else {
                    false
                }
            }) else {
                return Err(Error::new(ast.span(), "Expected #[repr(u8)]"));
            };

            repr.parse_nested_meta(|meta| {
                if meta.path.is_ident("u8") {
                    return Ok(());
                };

                Err(meta.error("Expected u8"))
            })
            .unwrap();

            let mut tokens_to = Vec::<TokenStream2>::new();
            let mut tokens_from = Vec::<TokenStream2>::new();

            let mut contains_some = false;
            let mut expression = 0u8;

            for variant in e.variants.iter() {
                let v_ident = &variant.ident;

                match &variant.discriminant {
                    Some((
                        _,
                        Expr::Lit(ExprLit {
                            attrs: _,
                            lit: Lit::Byte(b),
                        }),
                    )) => expression = b.value(),
                    Some((
                        _,
                        Expr::Lit(ExprLit {
                            attrs: _,
                            lit: Lit::Char(ch),
                        }),
                    )) => expression = ch.value() as u8,
                    Some((
                        _,
                        Expr::Lit(ExprLit {
                            attrs: _,
                            lit: Lit::Int(i),
                        }),
                    )) => expression = i.base10_digits().parse::<u8>().unwrap(),

                    Some((_, expr)) => {
                        return Err(Error::new(expr.span(), "Expected int, byte or char"));
                    }
                    None => {}
                }

                match &variant.fields {
                    syn::Fields::Named(syn::FieldsNamed {
                        brace_token: _,
                        named,
                    }) => {
                        contains_some = true;

                        let mut local_from = Vec::<TokenStream2>::new();
                        let mut field_names = Vec::<Ident>::new();
                        let mut first: TokenStream2 = Default::default();

                        for field in named.iter() {
                            handle_field(
                                field,
                                &mut local_from,
                                &mut first,
                                &mut field_names,
                                None,
                            )?;
                        }

                        let field = quote! { { #(#field_names, )* } };

                        modify_sadb_tokens_enum(
                            named,
                            v_ident,
                            expression,
                            &mut tokens_from,
                            &mut tokens_to,
                            &mut local_from,
                            first,
                            field,
                            &mut field_names,
                        )?;
                    }
                    syn::Fields::Unnamed(syn::FieldsUnnamed {
                        paren_token: _,
                        unnamed,
                    }) => {
                        contains_some = true;

                        let mut local_from = Vec::<TokenStream2>::new();

                        let mut last_name = 'a';
                        let mut field_names = Vec::<Ident>::new();
                        let mut first: TokenStream2 = Default::default();

                        for field in unnamed.iter() {
                            handle_field(
                                field,
                                &mut local_from,
                                &mut first,
                                &mut field_names,
                                Some(&mut last_name),
                            )?;

                            if last_name == '{' {
                                return Err(Error::new(
                                    field.span(),
                                    format!(
                                        r#"Why the fuck you have so many unnamed fields in {}???
                                           Advice: Use freaking named fields or better a struct"#,
                                        variant.ident
                                    ),
                                ));
                            }
                        }

                        let field = quote! { ( #(#field_names, )* ) };

                        modify_sadb_tokens_enum(
                            unnamed,
                            v_ident,
                            expression,
                            &mut tokens_from,
                            &mut tokens_to,
                            &mut local_from,
                            first,
                            field,
                            &mut field_names,
                        )?;
                    }
                    syn::Fields::Unit => {
                        tokens_from.push(quote! {
                            #expression => Ok(Self::#v_ident),
                        });
                        tokens_to.push(quote! {
                            Self::#v_ident => buf.push(#expression),
                        });
                    }
                }
                expression += 1;
            }

            (
                if contains_some {
                    quote! {
                        let mut buf = vec![unsafe { *<*const _>::from(self).cast::<u8>() }];

                        match self {
                            #(#tokens_to)*
                        }

                        buf
                    }
                } else {
                    quote! {
                        vec![unsafe { *<*const _>::from(self).cast::<u8>() }]
                    }
                },
                quote! {
                    match input[0] {
                        #(#tokens_from)*
                        _ => Err(SerDeBytesError::UnexpectedToken),
                    }
                },
            )
        }
        syn::Data::Struct(s) => {
            let mut tokens_to = Vec::<TokenStream2>::new();
            let mut tokens_from = Vec::<TokenStream2>::new();

            match &s.fields {
                syn::Fields::Named(syn::FieldsNamed {
                    brace_token: _,
                    named,
                }) => {
                    let mut local_from = Vec::<TokenStream2>::new();
                    let mut field_names = Vec::<Ident>::new();
                    let mut first: TokenStream2 = Default::default();

                    for field in named.iter() {
                        handle_field(field, &mut local_from, &mut first, &mut field_names, None)?;
                    }

                    let field = quote! { { #(#field_names, )* } };

                    modify_sadb_tokens_struct(
                        &named,
                        &mut tokens_from,
                        &mut tokens_to,
                        &mut local_from,
                        first,
                        field,
                        &mut field_names,
                    )?;
                }
                syn::Fields::Unnamed(syn::FieldsUnnamed {
                    paren_token: _,
                    unnamed,
                }) => {
                    let mut local_from = Vec::<TokenStream2>::new();
                    let mut last_name = 'a';
                    let mut field_names = Vec::<Ident>::new();
                    let mut first: TokenStream2 = Default::default();

                    for field in unnamed.iter() {
                        handle_field(
                            field,
                            &mut local_from,
                            &mut first,
                            &mut field_names,
                            Some(&mut last_name),
                        )?;

                        if last_name == '{' {
                            return Err(Error::new(
                                field.span(),
                                format!(
                                    r#"Why the fuck you have so many unnamed fields in {}???
                                       Advice: Use freaking named fields or better a struct"#,
                                    ident
                                ),
                            ));
                        }
                    }

                    let field = quote! { ( #(#field_names, )* ) };

                    modify_sadb_tokens_struct(
                        &unnamed,
                        &mut tokens_from,
                        &mut tokens_to,
                        &mut local_from,
                        first,
                        field,
                        &mut field_names,
                    )?;
                }
                syn::Fields::Unit => {
                    return Err(Error::new(ast.span(), "Unit structs are unsupported"));
                }
            }

            (
                quote! {
                    let mut buf = Vec::<u8>::new();

                    #(#tokens_to)*

                    buf
                },
                quote! { #(#tokens_from)* },
            )
        }
        _ => return Err(Error::new(ast.span(), "Expected Enum or Struct")),
    };

    Ok(quote! {
        impl #impl_generics Sadaby for #ident #type_generics #where_clause {
            fn se_bytes(&self) -> Vec<u8> {
                #complete_tokens_to
            }
            fn de_bytes(input: &[u8]) -> Result<Self, SadabyError> {
                #complete_tokens_from
            }
        }
    })
}

fn handle_field(
    field: &syn::Field,
    local_from: &mut Vec<TokenStream2>,
    first: &mut TokenStream2,
    field_names: &mut Vec<Ident>,
    last_name: Option<&mut char>,
) -> syn::Result<()> {
    let ty = match &field.ty {
        syn::Type::Path(syn::TypePath { qself: None, path }) => {
            let path_iter = path.segments.clone().into_iter();
            let path_last_segm = path_iter.clone().last();
            if let Some(path_segm) = path_last_segm
                && let syn::PathArguments::AngleBracketed(syn::AngleBracketedGenericArguments {
                    colon2_token: _,
                    lt_token: _,
                    args,
                    gt_token: _,
                }) = &path_segm.arguments
            {
                let inner_type = match args.first().unwrap() {
                    syn::GenericArgument::Type(syn::Type::Path(path)) => path.to_token_stream(),
                    syn::GenericArgument::Type(syn::Type::Tuple(tuple)) => tuple.to_token_stream(),
                    syn::GenericArgument::Type(syn::Type::Slice(slice)) => slice.to_token_stream(),
                    syn::GenericArgument::Type(syn::Type::Array(array)) => array.to_token_stream(),
                    _ => {
                        return Err(Error::new(
                            args.span(),
                            "Expected Type, Tuple, Slice or Array",
                        ));
                    }
                };

                let p_ident = path_segm.ident;
                let b_path_len = path_iter.len();
                let mut b_path_vec = Vec::<TokenStream2>::new();
                for (i, p) in path_iter.enumerate() {
                    if i == b_path_len - 1 {
                        break;
                    }
                    b_path_vec.push(p.to_token_stream());
                }
                quote! { #(#b_path_vec :: )* #p_ident :: < #inner_type > }
            } else {
                path.to_token_stream()
            }
        }
        syn::Type::Array(t) => quote! { <#t> },
        _ => {
            return Err(Error::new(
                field.ty.span(),
                format!(
                    "WTF are you trying to use in {}",
                    field.ty.clone().into_token_stream()
                ),
            ));
        }
    };

    let name: Ident;
    match last_name {
        Some(c) => {
            let n = *c;
            *c = ((*c as u8) + 1) as char;

            name = Ident::new(n.encode_utf8(&mut [0u8; 1]), Span::call_site());

            if field_names.is_empty() {
                *first = quote! {
                    ( #ty::de_bytes(&input[1..])? )
                };
            }
        }
        None => {
            name = field.ident.clone().unwrap();

            if field_names.is_empty() {
                *first = quote! {
                    { #name : #ty::de_bytes(&input[1..])? }
                }
            }
        }
    };

    // TODO: make it smarter. Make it able to hardcode current and next values for types with
    //       always stable* size (like [u8; 2], f32, u64 etc.)
    local_from.push(quote! {
        let #name = #ty::de_bytes(&input[current..=next])?;
    });
    local_from.push(quote! {
        current = next + 1;
        next = current + input[current] as usize;
        current += 1;
        //dbg!(&current, &next);
    });

    field_names.push(name);

    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn modify_sadb_tokens_enum<T: Spanned>(
    un_named_field: &T,
    v_ident: &syn::Ident,

    expression: u8,

    tokens_from: &mut Vec<TokenStream2>,
    tokens_to: &mut Vec<TokenStream2>,

    local_from: &mut Vec<TokenStream2>,
    first: TokenStream2,

    field: TokenStream2,
    field_names: &mut Vec<Ident>,
) -> syn::Result<()> {
    if field_names.is_empty() {
        return Err(Error::new(
            un_named_field.span(),
            format!("Put something in '{}'", v_ident),
        ));
    }

    if field_names.len() == 1 {
        tokens_to.push(quote! {
            Self::#v_ident #field => {
                #(
                    buf.append(&mut #field_names.se_bytes());
                )*
            }
        });
        tokens_from.push(quote! {
            #expression => Ok( Self::#v_ident #first ),
        });
    } else {
        let _ = local_from.pop();
        tokens_to.push(quote! {
            Self::#v_ident #field => {
                #(
                    let mut #field_names = #field_names.se_bytes();
                    buf.push(#field_names.len() as u8);
                    buf.append(&mut #field_names);
                )*
            }
        });
        tokens_from.push(quote! {
            #expression => {
                let mut current = 2usize;
                let mut next = 1 + input[1] as usize;

                #(#local_from)*

                Ok(Self::#v_ident #field )
            }
        });
    };

    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn modify_sadb_tokens_struct<T: Spanned>(
    un_named_field: &T,

    tokens_from: &mut Vec<TokenStream2>,
    tokens_to: &mut Vec<TokenStream2>,

    local_from: &mut Vec<TokenStream2>,
    first: TokenStream2,

    field: TokenStream2,
    field_names: &mut Vec<Ident>,
) -> syn::Result<()> {
    if field_names.is_empty() {
        return Err(Error::new(
            un_named_field.span(),
            "Put something in the struct",
        ));
    }

    if field_names.len() == 1 {
        tokens_to.push(quote! {
            #(
                buf.append(&mut self.#field_names.se_bytes());
            )*
        });
        tokens_from.push(quote! {
            Ok( Self #first )
        });
    } else {
        let _ = local_from.pop();
        tokens_to.push(quote! {
            #(
                let mut #field_names = self.#field_names.se_bytes();
                buf.push(#field_names.len() as u8);
                buf.append(&mut #field_names);
            )*
        });
        tokens_from.push(quote! {
            let mut current = 1usize;
            let mut next = input[0] as usize;

            #(#local_from)*

            Ok(Self #field )
        });
    };

    Ok(())
}
