use rouge_asset::{
    actions::{AssetLoaded, LoadAsset},
    loader::AssetLoader,
    plugin::{AssetGameExt, AssetPlugin},
    storage::Assets,
    Asset, AssetId, DefaultSettings,
};
use rouge_ecs::{
    observer::{
        builtin::{AddComponent, CreateEntity, DeleteEntity, RemoveComponent},
        Actions, IntoObserver, Observers,
    },
    query::Query,
    Component, Entity, IntoSystem, World,
};
use rouge_game::game::{Game, PostInit, PostUpdate, Start, Update};

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

fn main() {
    let mut game = Game::new();
    game.add_plugin(AssetPlugin::default())
        .register_asset::<TextFile>()
        .add_asset_loader::<TextFile>();

    game.register::<Player>();
    game.add_system(PostInit, load_text_file);
    game.add_system(Update, update.after(start));
    game.add_system(Update, test.before(world_system));
    game.add_system(PostUpdate, post_update);

    // let add_player_systems = Observers::<AddComponent<Player>>::new().add_system(player_added);
    // let remove_player_systems =
    //     Observers::<RemoveComponent<Player>>::new().add_system(player_removed);
    // let delete_entity_systems = Observers::<DeleteEntity>::new().add_system(entities_deleted);
    // game.add_observers(add_player_systems);
    // game.add_observers(remove_player_systems);
    // game.add_observers(delete_entity_systems);

    let asset_loaded = Observers::<AssetLoaded<TextFile>>::new().add_system(
        |ids: &[AssetId], assets: &Assets<TextFile>| {
            for id in ids {
                println!("Asset Loaded: {:?}", assets.get(id).unwrap().contents());
            }
        },
    );

    game.add_observers(asset_loaded);

    game.run();
}
