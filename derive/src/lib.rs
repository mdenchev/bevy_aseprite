use bevy_aseprite_reader::Aseprite;
use heck::ToShoutySnekCase;
use proc_macro::TokenStream;
use proc_macro_error::abort;
use proc_macro_error::proc_macro_error;
use quote::{format_ident, quote};
use syn::{parse::Parse, parse_macro_input, Ident, LitStr, Token, Visibility};

extern crate proc_macro;

struct AsepriteDeclaration {
    vis: Visibility,
    name: Ident,
    path: LitStr,
}

impl Parse for AsepriteDeclaration {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let vis: Visibility = input.parse()?;
        let name: Ident = input.parse()?;
        input.parse::<Token!(,)>()?;
        let path: LitStr = input.parse()?;

        Ok(AsepriteDeclaration { vis, name, path })
    }
}

#[proc_macro]
#[proc_macro_error]
pub fn aseprite(input: TokenStream) -> TokenStream {
    let AsepriteDeclaration { vis, name, path } = parse_macro_input!(input as AsepriteDeclaration);

    let aseprite = match Aseprite::from_path(format!("assets/{}", path.value())) {
        Ok(aseprite) => aseprite,
        Err(err) => {
            abort!(path, "Could not load file."; note = err);
        }
    };

    let tags = aseprite.tags();
    let tag_names = tags
        .all()
        .map(|tag| format_ident!("{}", tag.name.TO_SHOUTY_SNEK_CASE()));
    let tag_values = tags.all().map(|tag| &tag.name);

    let slices = aseprite.slices();

    let slice_names = slices
        .get_all()
        .map(|slice| format_ident!("{}", slice.name.TO_SHOUTY_SNEK_CASE()));
    let slice_values = slices.get_all().map(|slice| &slice.name);

    let expanded = quote! {
        #[allow(non_snake_case)]
        #vis mod #name {
            pub const PATH: &'static str = #path;

            pub mod tags {
                #( pub const #tag_names: &'static str = #tag_values; )*
            }

            pub mod slices {
                #( pub const #slice_names: &'static str = #slice_values; )*
            }
        }
    };

    TokenStream::from(expanded)
}
