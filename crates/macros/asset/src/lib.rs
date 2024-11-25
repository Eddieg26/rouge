use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, DeriveInput};

#[proc_macro_derive(Asset)]
pub fn derive_asset(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let name = &input.ident;

    let (impl_generics, type_generics, where_clause) = &input.generics.split_for_impl();

    TokenStream::from(quote! {
        impl #impl_generics asset::Asset for #name #type_generics #where_clause { }
    })
}
