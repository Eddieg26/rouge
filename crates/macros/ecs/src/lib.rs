use proc_macro::TokenStream;
use quote::quote;
use syn::{DeriveInput, parse_macro_input};

#[proc_macro_derive(Component)]
pub fn derive_component(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let name = &input.ident;

    let (impl_generics, type_generics, where_clause) = &input.generics.split_for_impl();

    TokenStream::from(quote! {
        impl #impl_generics ecs::Component for #name #type_generics #where_clause { }
    })
}

#[proc_macro_derive(Resource)]
pub fn derive_resource(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let name = &input.ident;

    let (impl_generics, type_generics, where_clause) = &input.generics.split_for_impl();

    TokenStream::from(quote! {
        impl #impl_generics ecs::Resource for #name #type_generics #where_clause { }
    })
}
