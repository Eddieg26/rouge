use macro_utils::{get_crate_path, Token};
use proc_macro::TokenStream;
use proc_macro2::Span;
use quote::ToTokens;
use std::{borrow::Cow, collections::HashMap};
use syn::{
    parse::{Parse, ParseStream},
    punctuated::Punctuated,
    token::Comma,
    Data, DataStruct, DeriveInput, Error, Expr, Fields, Ident, LitInt, Meta, MetaList,
    MetaNameValue, Result,
};

/*
   [uniform(0)]
   [texture(0, dimension=2d, visibility="fragment")]
   [sampler(0, sampler_type="uint", visibility="vertex, fragment")]
   [storage_texture(0, dimension=2d, access="read_write", format="rgba8unorm", visibility="all"))]
   [storage(0, access="read_write", visibility="all")]
*/

const DATA_ATTRIBUTE: Token = Token("data");
const UNIFORM_ATTRIBUTE: Token = Token("uniform");
const TEXTURE_ATTRIBUTE: Token = Token("texture");
const SAMPLER_ATTRIBUTE: Token = Token("sampler");
const VISIBILITY: Token = Token("visibility");
const VISIBILITY_ALL: Token = Token("all");
const VISIBILITY_VERTEX: Token = Token("vertex");
const VISIBILITY_FRAGMENT: Token = Token("fragment");
const VISIBILITY_COMPUTE: Token = Token("compute");
const TEXTURE_DIMENSION: Token = Token("dimension");
const MULTISAMPLED: Token = Token("multisampled");
const SAMPLE_TYPE: Token = Token("sample_type");
const SAMPLER_TYPE: Token = Token("sampler_type");
const BIND_GROUP_LAYOUT_BUILDER: Token = Token("bind_group_layout_builder");
const BIND_GROUP_BUILDER: Token = Token("bind_group_builder");

fn generate_create_bind_group(input: DeriveInput) -> Result<TokenStream> {
    let graphics = get_crate_path("graphics");
    let ecs = get_crate_path("ecs");

    let name = &input.ident;
    let mut data_attribute = None;
    let mut uniforms = HashMap::new();
    let mut layout = proc_macro2::TokenStream::new();
    let mut entries = Vec::<proc_macro2::TokenStream>::new();
    let mut type_defs = Vec::new();

    for attribute in &input.attrs {
        if let Some(ident) = attribute.path().get_ident() {
            if ident == DATA_ATTRIBUTE {
                data_attribute = attribute
                    .parse_args_with(|input: ParseStream| input.parse::<Ident>())
                    .ok();
            } else if ident == UNIFORM_ATTRIBUTE {
                let binding_info = match attribute.parse_args_with(AttributeInfo::parse) {
                    Ok(binding_info) => binding_info,
                    Err(error) => return Err(error),
                };

                let binding_attribute =
                    match BindingAttribute::parse_info(BindingType::Uniform, binding_info) {
                        Ok(attribute) => attribute,
                        Err(error) => return Err(error),
                    };

                if let BindingAttribute::Uniform { binding, ty } = binding_attribute {
                    let Some(ty) = ty else {
                        return Err(Error::new_spanned(attribute, "Expected type"));
                    };

                    entries.push(quote::quote! {
                        let mut uniform_buffer = #graphics::encase::UniformBuffer::new(Vec::<u8>::new());
                        uniform_buffer.write(&<Self as #graphics::resource::IntoBufferData<#ty>>::into_buffer_data(self)).unwrap();

                        uniform_buffers.push(Buffer::with_data(device, uniform_buffer.as_ref().as_slice(), BufferUsages::UNIFORM, None));
                        #BIND_GROUP_BUILDER.add_buffer(#binding, uniform_buffers.last().unwrap().inner(), 0, None);
                    });
                } else {
                    return Err(Error::new_spanned(attribute, "Invalid attribute"));
                }
            }
        }
    }

    let fields = match &input.data {
        Data::Struct(DataStruct {
            fields: Fields::Named(fields),
            ..
        }) => &fields.named,
        Data::Enum(_) => return Err(Error::new_spanned(input, "Enums not supported")),
        Data::Union(_) => return Err(Error::new_spanned(input, "Unions not supported")),
        _ => return Err(Error::new_spanned(input, "Unsupported data type")),
    };

    for field in fields {
        let field_name = match &field.ident {
            Some(ident) => ident,
            None => return Err(Error::new_spanned(field, "Unnamed fields not supported")),
        };

        for attribute in &field.attrs {
            let Some(ident) = attribute.path().get_ident() else {
                continue;
            };

            let binding_type = if ident == UNIFORM_ATTRIBUTE {
                BindingType::Uniform
            } else if ident == TEXTURE_ATTRIBUTE {
                BindingType::Texture
            } else if ident == SAMPLER_ATTRIBUTE {
                BindingType::Sampler
            } else {
                continue;
            };

            let binding_info = match attribute.parse_args_with(AttributeInfo::parse) {
                Ok(binding_info) => binding_info,
                Err(error) => return Err(error),
            };

            let binding_attribute = match BindingAttribute::parse_info(binding_type, binding_info) {
                Ok(binding_attribute) => binding_attribute,
                Err(error) => return Err(error),
            };

            match binding_attribute {
                BindingAttribute::Uniform { binding, .. } => {
                    uniforms.entry(binding).or_insert(Vec::new()).push(field);
                }
                BindingAttribute::Texture {
                    binding,
                    dimension,
                    ty,
                    multisampled,
                    visibility,
                } => {
                    entries.push(quote::quote! {
                        let id: Id<RenderTexture> = match self.#field_name {
                            Some(id) => id.into(),
                            None => fallbacks.dimension_id(#dimension),
                        };
                        match textures.get(&id) {
                            Some(texture) => #BIND_GROUP_BUILDER.add_texture(#binding, texture.view()),
                            None => return Err(#graphics::resource::CreateBindGroupError::MissingTexture{id}),
                        }
                    });

                    layout.extend(quote::quote! {
                        #BIND_GROUP_LAYOUT_BUILDER = #BIND_GROUP_LAYOUT_BUILDER.with_texture(#binding,  #ty, #dimension, #visibility, #multisampled);
                    });
                }
                BindingAttribute::Sampler {
                    binding,
                    ty,
                    visibility,
                } => {
                    entries.push(quote::quote! {
                        let id: Id<Sampler> = match self.#field_name {
                            Some(id) => id.into(),
                            None => fallbacks.sampler,
                        };
                        match samplers.get(&id) {
                            Some(sampler) =>  #BIND_GROUP_BUILDER.add_sampler(#binding, *sampler),
                            None => return Err(#graphics::resource::CreateBindGroupError::MissingSampler{id}),
                        }
                    });

                    layout.extend(quote::quote! {
                        #BIND_GROUP_LAYOUT_BUILDER  = #BIND_GROUP_LAYOUT_BUILDER.with_sampler(#binding, #ty, #visibility);
                    });
                }
            }
        }
    }

    let mut uniform_buffer_def = quote::quote! {};
    for (binding, fields) in uniforms {
        let field = match fields.len() {
            0 => return Err(Error::new_spanned(binding, "Empty fields not supported")),
            1 => {
                let field = fields[0].ident.as_ref().unwrap();
                let uniform_buffer_name =
                    Ident::new(&format!("uniform_buffer_{}", binding), Span::call_site());

                let uniform_buffer = quote::quote! {
                    let mut #uniform_buffer_name = #graphics::encase::UniformBuffer::new(Vec::<u8>::new());
                    #uniform_buffer_name.write(#field).unwrap();

                    uniform_buffers.push(Buffer::with_data(device, #uniform_buffer_name.as_ref().as_slice(), BufferUsages::UNIFORM, None));
                };

                entries.push(uniform_buffer);

                Cow::Borrowed(&fields[0].ty)
            }
            _ => {
                let uniform_struct_name = Ident::new(
                    &format!("{}UniformBufferBinding{}", name, binding),
                    Span::call_site(),
                );
                let uniform_buffer_name =
                    Ident::new(&format!("uniform_buffer_{}", binding), Span::call_site());

                let uniform_values = fields.iter().map(|field| {
                    let field_name = &field.ident;
                    quote::quote! { #field_name: self.#field_name }
                });

                let uniform_buffer = quote::quote! {
                    let value = #uniform_struct_name {
                        #(#uniform_values),*
                    };
                    let mut #uniform_buffer_name = #graphics::encase::UniformBuffer::new(Vec::<u8>::new());
                    #uniform_buffer_name.write(&value).unwrap();

                    uniform_buffers.push(Buffer::with_data(device, #uniform_buffer_name.as_ref().as_slice(), BufferUsages::UNIFORM, None));
                    #BIND_GROUP_BUILDER.add_buffer(#binding, uniform_buffers.last().unwrap().inner(), 0, None);
                };

                let fields = fields.iter().map(|field| {
                    let field_name = &field.ident;
                    let field_ty = &field.ty;

                    quote::quote! { #field_name: #field_ty }
                });

                let def = quote::quote! {
                    #[derive(#graphics::encase::ShaderType)]
                    struct #uniform_struct_name {
                        #(#fields),*
                    }
                };

                type_defs.push(def);

                entries.push(uniform_buffer);

                let ty = syn::Type::Verbatim(quote::quote! { #uniform_struct_name }.into());

                Cow::Owned(ty)
            }
        };

        let field = field.as_ref();

        let uniform_layout = quote::quote! {
            #BIND_GROUP_LAYOUT_BUILDER = #BIND_GROUP_LAYOUT_BUILDER.with_uniform_buffer(#binding, ShaderStages::all(), false, Some(<#field as #graphics::encase::ShaderType>::min_size()), None);
        };

        layout.extend(uniform_layout);
        uniform_buffer_def = quote::quote! {let mut uniform_buffers = Vec::new();}
    }

    let (impl_generics, ty_generics, where_clause) = input.generics.split_for_impl();
    let type_name = name.to_string();

    let (bind_group_data, get_bind_group_data) = match data_attribute {
        Some(data_attribute) => (
            quote::quote! { #data_attribute },
            quote::quote! { self.into_bind_group_data() },
        ),
        None => (quote::quote! { () }, quote::quote! { () }),
    };

    Ok(TokenStream::from(quote::quote! {
        #(#type_defs)*

        impl #impl_generics #graphics::resource::CreateBindGroup for #name #ty_generics #where_clause {
            type Data = #bind_group_data;

            type Arg = #ecs::system::StaticArg<'static,(
                #ecs::system::unlifetime::ReadRes<#graphics::RenderAssets<#graphics::resource::RenderTexture>>,
                #ecs::system::unlifetime::ReadRes<#graphics::RenderAssets<#graphics::resource::Sampler>>,
                #ecs::system::unlifetime::ReadRes<#graphics::resource::Fallbacks>
            )>;

            fn label() -> Option< &'static str> {
                Some(#type_name)
            }

            fn bind_group(
                &self,
                device: &RenderDevice,
                layout: &BindGroupLayout,
                arg: &#ecs::system::ArgItem<Self::Arg>,
            ) -> Result<BindGroup<Self::Data>, #graphics::resource::CreateBindGroupError> {
                use #graphics::{wgpu::BufferUsages, resource::{Buffer, BindGroupEntries, TextureDimension, Sampler, RenderTexture, IntoBufferData}};

                let (textures, samplers, fallbacks) = arg.inner();

                #uniform_buffer_def
                let mut #BIND_GROUP_BUILDER = BindGroupEntries::new();

                #(#entries)*

                Ok(BindGroup::create(device, layout, #BIND_GROUP_BUILDER.entries(), #get_bind_group_data))
            }

            fn bind_group_layout(device: &RenderDevice) -> BindGroupLayout {
                use #graphics::{wgpu::{TextureSampleType, SamplerBindingType, ShaderStages}, resource::{BindGroupLayoutBuilder, TextureDimension}};

                let mut #BIND_GROUP_LAYOUT_BUILDER = BindGroupLayoutBuilder::new();

                #layout

                #BIND_GROUP_LAYOUT_BUILDER.build(device)
            }

        }
    }))
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
enum TextureDimension {
    D1,
    #[default]
    D2,
    D2Array,
    D3,
    Cube,
    CubeArray,
}

impl ToTokens for TextureDimension {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        tokens.extend(match self {
            Self::D1 => quote::quote! { TextureDimension::D1 },
            Self::D2 => quote::quote! { TextureDimension::D2 },
            Self::D2Array => quote::quote! { TextureDimension::D2Array },
            Self::D3 => quote::quote! { TextureDimension::D3 },
            Self::Cube => quote::quote! { TextureDimension::Cube },
            Self::CubeArray => quote::quote! { TextureDimension::CubeArray },
        });
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TextureSampleType {
    Float { filterable: bool },
    Depth,
    Int,
    UInt,
}

impl ToTokens for TextureSampleType {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        tokens.extend(match self {
            Self::Float { filterable } => {
                quote::quote! { TextureSampleType::Float { filterable: #filterable } }
            }
            Self::Depth => quote::quote! { TextureSampleType::Depth },
            Self::Int => quote::quote! { TextureSampleType::Int },
            Self::UInt => quote::quote! { TextureSampleType::UInt },
        });
    }
}

impl Default for TextureSampleType {
    fn default() -> Self {
        Self::Float { filterable: true }
    }
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
enum SamplerBindingType {
    #[default]
    Filtering,
    NonFiltering,
    Comparison,
}

impl ToTokens for SamplerBindingType {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        tokens.extend(match self {
            Self::Filtering => quote::quote! { SamplerBindingType::Filtering },
            Self::NonFiltering => quote::quote! { SamplerBindingType::NonFiltering },
            Self::Comparison => quote::quote! { SamplerBindingType::Comparison },
        });
    }
}

bitflags::bitflags! {
    #[derive(Debug,  Clone, Copy, PartialEq, Eq)]
    struct ShaderStages: u32 {
        const VERTEX = 0b00000001;
        const FRAGMENT = 0b00000010;
        const COMPUTE = 0b00000100;
        const ALL = Self::VERTEX.bits() | Self::FRAGMENT.bits() | Self::COMPUTE.bits();
    }
}

impl ToTokens for ShaderStages {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        let mut temp = Vec::new();

        if self.contains(ShaderStages::VERTEX) {
            temp.push(quote::quote! { ShaderStages::VERTEX });
        }

        if self.contains(ShaderStages::FRAGMENT) {
            temp.push(quote::quote! { ShaderStages::FRAGMENT });
        }

        if self.contains(ShaderStages::COMPUTE) {
            temp.push(quote::quote! { ShaderStages::COMPUTE });
        }

        let temp = temp
            .into_iter()
            .fold(quote::quote! {ShaderStages::empty()}, |tokens, token| {
                quote::quote! { #tokens | #token }
            });

        tokens.extend(temp);
    }
}

impl Default for ShaderStages {
    fn default() -> Self {
        Self::ALL
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum BindingType {
    Uniform,
    Texture,
    Sampler,
}

enum BindingAttribute {
    Uniform {
        binding: u32,
        ty: Option<Ident>,
    },
    Sampler {
        binding: u32,
        ty: SamplerBindingType,
        visibility: ShaderStages,
    },
    Texture {
        binding: u32,
        dimension: TextureDimension,
        ty: TextureSampleType,
        multisampled: bool,
        visibility: ShaderStages,
    },
}

impl BindingAttribute {
    fn parse_info(ty: BindingType, info: AttributeInfo) -> Result<Self> {
        match ty {
            BindingType::Uniform => Ok(Self::Uniform {
                binding: info.binding,
                ty: info
                    .meta
                    .iter()
                    .next()
                    .and_then(|meta| meta.path().get_ident().cloned()),
            }),
            BindingType::Sampler => {
                let mut ty = SamplerBindingType::default();
                let mut visibility = ShaderStages::default();

                for meta in info.meta {
                    match meta {
                        Meta::NameValue(name) if name.path == SAMPLER_TYPE => {
                            ty = parse_sampler_binding_type(&name)?;
                        }
                        Meta::List(meta) if meta.path == VISIBILITY => {
                            visibility = parse_visibility(&meta)?;
                        }
                        _ => return Err(Error::new_spanned(meta, "Invalid attribute")),
                    }
                }

                Ok(Self::Sampler {
                    binding: info.binding,
                    ty,
                    visibility,
                })
            }
            BindingType::Texture => {
                let mut dimension = TextureDimension::default();
                let mut ty = TextureSampleType::default();
                let mut visibility = ShaderStages::default();
                let mut multisampled = false;

                for meta in info.meta {
                    match meta {
                        Meta::NameValue(name) if name.path == TEXTURE_DIMENSION => {
                            dimension = parse_texture_dimension(&name)?;
                        }
                        Meta::NameValue(name) if name.path == MULTISAMPLED => {
                            multisampled = parse_syn_bool(&name)?;
                        }
                        Meta::NameValue(name) if name.path == SAMPLE_TYPE => {
                            ty = parse_texture_sample_type(&name)?;
                        }
                        Meta::List(meta) if meta.path == VISIBILITY => {
                            visibility = parse_visibility(&meta)?;
                        }
                        _ => return Err(Error::new_spanned(meta, "Invalid attribute")),
                    }
                }

                Ok(Self::Texture {
                    binding: info.binding,
                    dimension,
                    ty,
                    multisampled,
                    visibility,
                })
            }
        }
    }
}
struct AttributeInfo {
    binding: u32,
    meta: Punctuated<Meta, Comma>,
}

impl std::fmt::Debug for AttributeInfo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AttributeInfo")
            .field("binding", &self.binding)
            .field(
                "meta",
                &self
                    .meta
                    .iter()
                    .map(|m| m.to_token_stream())
                    .collect::<Vec<_>>(),
            )
            .finish()
    }
}

impl Parse for AttributeInfo {
    fn parse(input: syn::parse::ParseStream) -> Result<Self> {
        let binding = input.parse::<LitInt>()?.base10_parse()?;

        match input.parse::<Comma>() {
            Ok(_) => {
                let meta = input.parse_terminated(Meta::parse, Comma)?;

                Ok(Self { binding, meta })
            }
            Err(_) => Ok(Self {
                binding,
                meta: Punctuated::new(),
            }),
        }
    }
}

fn parse_visibility(meta: &MetaList) -> Result<ShaderStages> {
    let mut paths = vec![];
    let _ = meta.parse_nested_meta(|meta| Ok(paths.push(meta.path)));

    if paths.is_empty() {
        return Err(Error::new_spanned(meta, "Expected shader stage"));
    }

    let mut stages = ShaderStages::empty();

    for path in paths {
        let ident = match path.get_ident() {
            Some(ident) => ident,
            None => return Err(Error::new_spanned(path, "Expected identifier")),
        };

        if ident == VISIBILITY_ALL {
            stages |= ShaderStages::ALL;
        } else if ident == VISIBILITY_VERTEX {
            stages |= ShaderStages::VERTEX;
        } else if ident == VISIBILITY_FRAGMENT {
            stages |= ShaderStages::FRAGMENT;
        } else if ident == VISIBILITY_COMPUTE {
            stages |= ShaderStages::COMPUTE;
        } else {
            return Err(Error::new_spanned(ident, "Invalid shader stage"));
        }
    }

    Ok(stages)
}

fn parse_texture_dimension(meta: &MetaNameValue) -> Result<TextureDimension> {
    let dimension = parse_syn_str(meta)?;

    match dimension.as_str() {
        "d1" => Ok(TextureDimension::D1),
        "d2" => Ok(TextureDimension::D2),
        "d2_array" => Ok(TextureDimension::D2Array),
        "d3" => Ok(TextureDimension::D3),
        "cube" => Ok(TextureDimension::Cube),
        "cube_array" => Ok(TextureDimension::CubeArray),
        _ => Err(Error::new_spanned(&meta.path, "Invalid texture dimension")),
    }
}

fn parse_texture_sample_type(meta: &MetaNameValue) -> Result<TextureSampleType> {
    let ty = parse_syn_str(meta)?;

    match ty.as_str() {
        "float" => Ok(TextureSampleType::Float { filterable: true }),
        "depth" => Ok(TextureSampleType::Depth),
        "int" => Ok(TextureSampleType::Int),
        "uint" => Ok(TextureSampleType::UInt),
        _ => Err(Error::new_spanned(
            &meta.path,
            "Invalid texture sample type",
        )),
    }
}

fn parse_sampler_binding_type(meta: &MetaNameValue) -> Result<SamplerBindingType> {
    let ty = parse_syn_str(meta)?;

    match ty.as_str() {
        "filtering" => Ok(SamplerBindingType::Filtering),
        "non_filtering" => Ok(SamplerBindingType::NonFiltering),
        "comparison" => Ok(SamplerBindingType::Comparison),
        _ => Err(Error::new_spanned(
            &meta.path,
            "Invalid sampler binding type",
        )),
    }
}

fn parse_syn_str(meta: &MetaNameValue) -> Result<String> {
    match &meta.value {
        Expr::Lit(syn::ExprLit {
            lit: syn::Lit::Str(lit),
            ..
        }) => Ok(lit.value()),
        _ => return Err(Error::new_spanned(&meta.path, "Expected string")),
    }
}

fn parse_syn_bool(meta: &MetaNameValue) -> Result<bool> {
    match &meta.value {
        Expr::Lit(syn::ExprLit {
            lit: syn::Lit::Bool(lit),
            ..
        }) => Ok(lit.value),
        _ => return Err(Error::new_spanned(&meta.path, "Expected boolean")),
    }
}

#[proc_macro_derive(CreateBindGroup, attributes(uniform, texture, sampler, data))]
pub fn derive_generate_create_bind_group(input: proc_macro::TokenStream) -> TokenStream {
    let input = syn::parse_macro_input!(input as syn::DeriveInput);

    match generate_create_bind_group(input) {
        Ok(token_stream) => token_stream,
        Err(error) => error.to_compile_error().into(),
    }
}
