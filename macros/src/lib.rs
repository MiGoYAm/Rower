use proc_macro::Span;
use proc_macro::TokenStream;
use quote::quote;
use syn::parse::Parse;
use syn::parse::ParseStream;
use syn::punctuated::Punctuated;
use syn::{parse_macro_input, ItemStruct};

struct Args {
    vars: Vec<syn::Expr>,
}

impl Parse for Args {
    fn parse(input: ParseStream) -> syn::parse::Result<Self> {
        let vars = Punctuated::<syn::Expr, syn::Token![,]>::parse_terminated(input)?;

        Ok(Args {
            vars: vars.into_iter().collect(),
        })
    }
}

impl Args {
    pub fn get_direction(&self) -> syn::Result<syn::Expr> {
        self.vars
            .first()
            .ok_or_else(|| syn::Error::new(Span::call_site().into(), "No direction was provided"))
            .cloned()
    }

    pub fn get_state(&self) -> syn::Result<syn::Expr> {
        self.vars
            .get(1)
            .ok_or_else(|| syn::Error::new(Span::call_site().into(), "No state was provided"))
            .cloned()
    }

    pub fn get_id(&self) -> syn::Result<syn::Expr> {
        self.vars
            .get(2)
            .ok_or_else(|| syn::Error::new(Span::call_site().into(), "No id was provided"))
            .cloned()
    }
}

#[proc_macro_attribute]
pub fn packet_const(attr: TokenStream, item: TokenStream) -> TokenStream {
    let args = parse_macro_input!(attr as Args);
    let item = parse_macro_input!(item as ItemStruct);

    let direction = args.get_direction().unwrap();
    let state = args.get_state().unwrap();
    let id = args.get_id().unwrap();

    let indent = item.ident.clone();
    let (impl_generics, ty_generics, where_clause) = item.generics.split_for_impl();

    quote! {
        #item

        impl #impl_generics IdPacket for #indent #ty_generics #where_clause {
            fn id(direction: Direction, state: State, _: ProtocolVersion) -> Option<u8> {
                match (direction, state) {
                    (#direction, #state) => Some(#id),
                    _ => None,
                }
            }
        }
    }
    .into()
}
