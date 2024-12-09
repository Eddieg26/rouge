use crate::{
    camera::Camera,
    materials::{
        standard::Standard,
        unlit::{UnlitColor, UnlitTexture},
        Mesh3d,
    },
};
use asset::{embed_asset, io::embedded::EmbeddedFs, plugin::AssetExt, AssetRef, Uuid};
use game::{GameBuilder, Plugin, Plugins};
use graphics::{
    plugin::RenderPlugin,
    resource::{plugin::MaterialAppExt, ShaderSource},
};

pub const MESH3D_SHADER: Uuid = Uuid::from_u128(293524138301106227452958946405827563583);
pub const UNLIT_COLOR_SHADER: Uuid = Uuid::from_u128(33531744476557521861330493527589569272);
pub const UNLIT_TEXTURE_SHADER: Uuid = Uuid::from_u128(283280117650599531559073816480036973034);

pub struct Render3dPlugin;

impl Plugin for Render3dPlugin {
    fn name(&self) -> &'static str {
        "Render3d"
    }

    fn start(&mut self, game: &mut GameBuilder) {
        let assets = EmbeddedFs::new("assets");
        let shader_id = AssetRef::<ShaderSource>::from(MESH3D_SHADER);
        embed_asset!(assets, shader_id, "assets/mesh3d.wgsl", ());

        let shader_id = AssetRef::<ShaderSource>::from(UNLIT_COLOR_SHADER);
        embed_asset!(assets, shader_id, "assets/unlit-color.wgsl", ());

        let shader_id = AssetRef::<ShaderSource>::from(UNLIT_TEXTURE_SHADER);
        embed_asset!(assets, shader_id, "assets/unlit-texture.wgsl", ());

        game.register::<Camera>()
            .add_material::<Standard<Mesh3d>>()
            .add_material::<UnlitColor<Mesh3d>>()
            .add_material::<UnlitTexture<Mesh3d>>()
            .embed_assets("graphics3d", assets);
    }

    fn dependencies(&self) -> Plugins {
        let mut plugins = Plugins::new();
        plugins.add(RenderPlugin);

        plugins
    }
}
