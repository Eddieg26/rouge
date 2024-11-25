use proc_macro2::{Span, TokenStream};
use syn::{Ident, Path};

pub mod token;

pub use token::*;

pub fn get_crate_path(name: &str) -> Path {
    match name.parse::<TokenStream>() {
        Ok(path) => match syn::parse2::<Path>(path.into()) {
            Ok(path) => path,
            Err(_) => Path::from(Ident::new(name, Span::call_site())),
        },
        _ => Path::from(Ident::new(name, Span::call_site())),
    }
}
