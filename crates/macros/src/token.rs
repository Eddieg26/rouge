use core::fmt::{self, Display};
use quote::ToTokens;
use syn::{Ident, Path};

#[derive(Copy, Clone, PartialEq, Eq)]
pub struct Token(pub &'static str);

impl PartialEq<Token> for Ident {
    fn eq(&self, word: &Token) -> bool {
        self == word.0
    }
}

impl<'a> PartialEq<Token> for &'a Ident {
    fn eq(&self, word: &Token) -> bool {
        *self == word.0
    }
}

impl PartialEq<Token> for Path {
    fn eq(&self, word: &Token) -> bool {
        self.is_ident(word.0)
    }
}

impl<'a> PartialEq<Token> for &'a Path {
    fn eq(&self, word: &Token) -> bool {
        self.is_ident(word.0)
    }
}

impl Display for Token {
    fn fmt(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str(self.0)
    }
}

impl ToTokens for Token {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        tokens.extend(self.0.parse::<proc_macro2::TokenStream>().unwrap());
    }
}
