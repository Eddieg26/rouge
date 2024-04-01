use rouge_asset::{
    actions::{AssetLoaded, LoadAsset},
    loader::AssetLoader,
    plugin::{AssetGameExt, AssetPlugin},
    storage::Assets,
    Asset, AssetId, DefaultSettings,
};
use rouge_ecs::{
    meta::{Access, AccessMeta, AccessType},
    observer::{
        builtin::{AddComponent, CreateEntity, DeleteEntity, RemoveComponent},
        Actions, IntoObserver, Observers,
    },
    query::Query,
    Component, Entity, IntoSystem, ResourceId, World,
};
use rouge_game::game::{Game, PostInit, PostUpdate, Start, Update};
use rouge_graphics::{
    core::{
        draw::{Draw, DrawCalls, Render},
        ty::color::Color,
    },
    renderer::graph::{
        nodes::{
            render::{Attachment, RenderGroup, RenderPass, Subpass},
            GraphNode,
        },
        resources::TextureDesc,
        RenderGraph,
    },
    wgpu,
};

#[derive(Debug)]
pub struct Player {
    health: u32,
}

impl Player {
    pub fn new(health: u32) -> Self {
        Self { health }
    }

    pub fn health(&self) -> u32 {
        self.health
    }
}

impl Component for Player {}

fn start(mut actions: Actions) {
    println!("Start");
    actions.add(CreateEntity::new().with(Player::new(100)));
    actions.add(CreateEntity::new());
}

fn update() {
    println!("Update");
}

fn test() {
    println!("Test");
}

fn world_system(_: &World) {
    println!("World System");
}

fn post_update(mut actions: Actions) {
    println!("Post Update");
    // actions.add(DeleteEntity::new(Entity::new(0, 0)));
}

fn player_added(entities: &[Entity], q: Query<&Player>) {
    println!("Player Added");
    for player in q.entities(entities) {
        println!("Player{:?}", player);
    }
}

fn player_removed(entities: &[Entity]) {
    println!("Player Removed");
    for entity in entities {
        println!("Off Player{:?}", entity);
    }
}

fn entities_deleted(entities: &[Entity]) {
    println!("Entities Deleted");
    for entity in entities {
        println!("Deleted Entity{:?}", entity);
    }
}

pub struct TextFile {
    contents: String,
}

impl TextFile {
    pub fn new(contents: String) -> Self {
        Self { contents }
    }

    pub fn contents(&self) -> &str {
        &self.contents
    }
}

impl Asset for TextFile {}

impl AssetLoader for TextFile {
    type Asset = TextFile;

    type Settings = DefaultSettings;

    type Arg = ();

    fn load(
        context: &mut rouge_asset::loader::LoadContext<Self::Settings>,
        data: &[u8],
    ) -> Result<Self::Asset, rouge_asset::error::AssetError> {
        println!("Loading TextFile: {:?}", context.path());
        Ok(Self::new(String::from_utf8_lossy(data).to_string()))
    }

    fn extensions() -> &'static [&'static str] {
        &["txt"]
    }
}

fn load_text_file(mut actions: Actions) {
    actions.add(LoadAsset::<TextFile>::process("test.txt"))
    // actions.add(LoadAsset::<TextFile>::new("test.txt"));
}

// fn main() {
//     let mut game = Game::new();
//     game.add_plugin(AssetPlugin::default())
//         .register_asset::<TextFile>()
//         .add_asset_loader::<TextFile>();

//     game.register::<Player>();
//     game.add_system(PostInit, load_text_file);
//     game.add_system(Update, update.after(start));
//     game.add_system(Update, test.before(world_system));
//     game.add_system(PostUpdate, post_update);

//     let add_player_systems = Observers::<AddComponent<Player>>::new().add_system(player_added);
//     let remove_player_systems =
//         Observers::<RemoveComponent<Player>>::new().add_system(player_removed);
//     let delete_entity_systems = Observers::<DeleteEntity>::new().add_system(entities_deleted);
//     game.add_observers(add_player_systems);
//     game.add_observers(remove_player_systems);
//     game.add_observers(delete_entity_systems);

//     let asset_loaded = Observers::<AssetLoaded<TextFile>>::new().add_system(
//         |ids: &[AssetId], assets: &Assets<TextFile>| {
//             for id in ids {
//                 println!("Asset Loaded: {:?}", assets.get(id).unwrap().contents());
//             }
//         },
//     );

//     game.add_observers(asset_loaded);

//     game.run();
// }

pub struct TestNode {
    name: String,
    access: Vec<AccessMeta>,
}

impl TestNode {
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            access: vec![],
        }
    }

    pub fn add_read(mut self, ty: AccessType) -> Self {
        self.access.push(AccessMeta::new(ty, Access::Read));
        self
    }

    pub fn add_write(mut self, ty: AccessType) -> Self {
        self.access.push(AccessMeta::new(ty, Access::Write));
        self
    }

    pub fn add_meta(mut self, ty: AccessType, access: Access) -> Self {
        self.access.push(AccessMeta::new(ty, access));
        self
    }
}

impl GraphNode for TestNode {
    fn execute(&self, _: rouge_graphics::renderer::graph::context::RenderContext) {
        println!("Executing TestNode: {:?}", &self.name);
    }

    fn prepare(&mut self, _: rouge_graphics::renderer::graph::context::RenderContext) {}

    fn phase(&self) -> rouge_graphics::renderer::graph::nodes::RenderPhase {
        rouge_graphics::renderer::graph::nodes::RenderPhase::Process
    }

    fn access(&self) -> Vec<rouge_ecs::meta::AccessMeta> {
        self.access.clone()
    }
}

pub struct Render3d {
    position: glam::Vec3,
    rotation: glam::Quat,
    scale: glam::Vec3,
    clear: Option<Color>,
    depth: u32,
}

impl Render for Render3d {
    fn clear(&self) -> Option<Color> {
        self.clear
    }

    fn depth(&self) -> u32 {
        self.depth
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}

pub struct Draw3d;

impl Draw for Draw3d {
    type Partition = Vec<Self>;

    type Render = Render3d;
}

pub struct Draw3dGroup;

impl RenderGroup<Draw3d> for Draw3dGroup {
    type Arg = ();

    fn render<'a>(
        &self,
        pass: &mut wgpu::RenderPass,
        arg: rouge_ecs::ArgItem<'a, Self::Arg>,
        render: &<Draw3d as Draw>::Render,
        calls: &DrawCalls<Draw3d>,
    ) {
    
    }
}

fn main() {
    let mut graph = RenderGraph::new();

    let texture = graph.create_texture("texture", TextureDesc::default());
    let read_node = TestNode::new("Read Node").add_read(AccessType::id(texture));
    let write_node = TestNode::new("Write Node").add_read(AccessType::World);

    graph.add_node("write_node", write_node);
    graph.add_node("read_node", read_node);
    graph.add_node(
        "forward",
        RenderPass::new()
            .with_color(
                Attachment::Surface,
                None,
                wgpu::StoreOp::Store,
                Some(Color::black()),
            )
            .with_subpass(Subpass::new()),
    );

    graph
        .node_mut::<RenderPass>("forward")
        .unwrap()
        .add_group(0, Draw3dGroup);

    graph.build_hierarchy();

    for (row, ids) in graph.hierarchy().iter().enumerate() {
        println!("Row: {}", row);
        for id in ids {
            let node = graph.node::<TestNode>(*id).unwrap();
            println!("Node: {:?}", &node.name);
        }
    }
}
