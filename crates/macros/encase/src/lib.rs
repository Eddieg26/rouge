use encase_derive_impl::{implement, syn};

fn encase_path() -> syn::Path {
    syn::parse_str("graphics::encase").unwrap()
}

implement!(encase_path());
