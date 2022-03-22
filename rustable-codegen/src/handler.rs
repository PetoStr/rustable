use proc_macro::TokenStream;
use quote::{format_ident, quote};
use syn::parse_macro_input;

struct Args {
    event: syn::LitStr,
    subject: syn::LitStr,
    object: Option<syn::LitStr>,
}

impl Args {
    fn new(args: syn::AttributeArgs) -> syn::Result<Self> {
        let mut event = None;
        let mut subject = None;
        let mut object = None;

        for arg in args {
            match arg {
                syn::NestedMeta::Meta(syn::Meta::NameValue(nv)) => {
                    if nv.path.is_ident("event") {
                        match nv.lit {
                            syn::Lit::Str(val) => event = Some(val),
                            _ => {
                                return Err(syn::Error::new_spanned(
                                    nv.lit,
                                    "Expects string literal for attribute event.",
                                ))
                            }
                        }
                    } else if nv.path.is_ident("subject") {
                        match nv.lit {
                            syn::Lit::Str(val) => subject = Some(val),
                            _ => {
                                return Err(syn::Error::new_spanned(
                                    nv.lit,
                                    "Expects string literal for attribute subject.",
                                ))
                            }
                        }
                    } else if nv.path.is_ident("object") {
                        match nv.lit {
                            syn::Lit::Str(val) => object = Some(val),
                            _ => {
                                return Err(syn::Error::new_spanned(
                                    nv.lit,
                                    "Expects string literal for attribute object.",
                                ))
                            }
                        }
                    } else {
                        return Err(syn::Error::new_spanned(nv.path, "Unknown attribute key."));
                    }
                }
                arg => return Err(syn::Error::new_spanned(arg, "Unknown attribute.")),
            }
        }

        Ok(Self {
            event: event.expect("Missing mandatory attribute event"),
            subject: subject.expect("Missing mandatory attribute subject"),
            object,
        })
    }
}

pub(crate) fn new(args: TokenStream, input: TokenStream) -> TokenStream {
    let args = Args::new(parse_macro_input!(args as syn::AttributeArgs));

    let args = match args {
        Ok(args) => args,
        Err(err) => return extend_error(input, err),
    };

    let ast = syn::parse::<syn::ItemFn>(input.clone());

    let mut ast = match ast {
        Ok(ast) => ast,
        Err(err) => return extend_error(input, err),
    };

    let struct_name = ast.sig.ident.clone();

    let Args {
        event,
        subject,
        object,
    } = args;

    let handler = format_ident!("fn_{}", ast.sig.ident);
    ast.sig.ident = handler.clone();

    let subject = if subject.value() == "*" {
        quote!(::rustable::medusa::Space::All)
    } else {
        quote!(::rustable::medusa::Space::ByName(#subject))
    };

    let object = match object {
        Some(object) => {
            if object.value() == "*" {
                quote!(Some(::rustable::medusa::Space::All))
            } else {
                quote!(Some(::rustable::medusa::Space::ByName(#object)))
            }
        }
        None => quote!(None),
    };

    let stream = quote! {
        #ast

        #[allow(non_camel_case_types, missing_docs)]
        pub struct #struct_name;

        impl ::rustable::medusa::handler::CustomHandler for #struct_name {
            fn define(self) -> ::rustable::medusa::handler::CustomHandlerDef {
                ::rustable::medusa::handler::CustomHandlerDef {
                    event: #event,
                    subject: #subject,
                    object: #object,
                    handler: ::rustable::force_boxed!(#handler),
                }
            }
        }
    };

    TokenStream::from(stream)
}

fn extend_error(mut input: TokenStream, err: syn::Error) -> TokenStream {
    input.extend(TokenStream::from(err.to_compile_error()));
    input
}
