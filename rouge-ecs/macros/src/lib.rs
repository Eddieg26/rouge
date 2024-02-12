extern crate proc_macro;
use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, DeriveInput};

#[proc_macro_derive(Component)]
pub fn derive_component(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let name = input.ident;
    let (impl_generics, ty_generics, where_clause) = input.generics.split_for_impl();
    let output = quote! {
        impl #impl_generics Component for #name #ty_generics #where_clause {}
    };
    output.into()
}

#[proc_macro_derive(Resource)]
pub fn derive_resource(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let name = input.ident;
    let (impl_generics, ty_generics, where_clause) = input.generics.split_for_impl();
    let output = quote! {
        impl #impl_generics rouge_ecs::resource::Resource for #name #ty_generics #where_clause {}
    };
    output.into()
}

#[proc_macro_derive(LocalResource)]
pub fn derive_local_resource(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let name = input.ident;
    let (impl_generics, ty_generics, where_clause) = input.generics.split_for_impl();
    let output = quote! {
        impl #impl_generics LocalResource for #name #ty_generics #where_clause {}
    };
    output.into()
}

#[proc_macro_derive(SchedulePhase)]
pub fn derive_schedule_phase(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let name = input.ident;
    let (impl_generics, ty_generics, where_clause) = input.generics.split_for_impl();
    let output = quote! {
        impl #impl_generics SchedulePhase for #name #ty_generics #where_clause {
            const PHASE: &'static str = stringify!(#name);
        }
    };
    output.into()
}
