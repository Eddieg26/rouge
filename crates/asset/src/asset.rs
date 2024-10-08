use ulid::Ulid;

pub trait Asset: 'static {}

pub trait Settings: serde::Serialize + for<'a>serde::Deserialize<'a> + 'static {}

pub struct AssetId(Ulid);
