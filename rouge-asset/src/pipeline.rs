use rouge_ecs::{macros::Resource, resource::Resource, ArgItem, SystemArg};

use crate::{
    metadata::{AssetMetadata, LoadSettings},
    Asset, AssetId, LoadContext,
};

pub trait AssetPipeline: Send + Sync + 'static {
    type Asset: Asset;
    type Settings: LoadSettings;
    type Arg: SystemArg;
    type PostProcessor: AssetPostProcessor<
        Asset = Self::Asset,
        Arg = Self::Arg,
        Settings = Self::Settings,
    >;

    fn load(ctx: &mut LoadContext<Self::Settings>, data: &[u8]) -> Option<Self::Asset>;
    fn unload<'a>(
        asset: Self::Asset,
        metadata: AssetMetadata<Self::Settings>,
        arg: &ArgItem<'a, Self::Arg>,
    );
    fn extensions() -> &'static [&'static str];
}

pub trait BaseAssetCacher: Send + Sync + 'static {
    type Asset: Asset;

    fn read(data: &[u8]) -> Option<Self::Asset>;
    fn write(asset: &Self::Asset) -> Vec<u8>;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Resource)]
pub struct AssetCacher<A: Asset> {
    reader: fn(&[u8]) -> Option<A>,
    writer: fn(&A) -> Vec<u8>,
}

impl<A: Asset> AssetCacher<A> {
    pub fn new<C: BaseAssetCacher<Asset = A>>() -> Self {
        Self {
            reader: C::read,
            writer: C::write,
        }
    }

    pub fn read(&self, data: &[u8]) -> Option<A> {
        (self.reader)(data)
    }

    pub fn write(&self, asset: &A) -> Vec<u8> {
        (self.writer)(asset)
    }
}

pub trait AssetPostProcessor: Send + Sync + 'static {
    type Asset: Asset;
    type Arg: SystemArg;
    type Settings: LoadSettings;

    fn process<'a>(
        _: AssetId,
        _: &mut Self::Asset,
        _: &'a Self::Settings,
        _: ArgItem<'a, Self::Arg>,
    ) {
    }
}

impl AssetPostProcessor for () {
    type Asset = ();
    type Arg = ();
    type Settings = ();
}
