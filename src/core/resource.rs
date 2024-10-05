use super::{
    internal::blob::{BlobCell, Ptr},
    Type,
};
use hashbrown::HashMap;
use std::{hash::Hash, thread::ThreadId};

pub trait Resource: 'static {}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ResourceId(Type);
impl ResourceId {
    pub fn of<R: Resource>() -> Self {
        Self(Type::of::<R>())
    }
}
impl std::ops::Deref for ResourceId {
    type Target = Type;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl Into<Type> for &ResourceId {
    fn into(self) -> Type {
        self.0
    }
}

pub struct ResourceMeta {
    name: &'static str,
    owner: Option<ThreadId>,
    is_send_sync: bool,
}

impl ResourceMeta {
    pub fn new<R: Resource>(is_send_sync: bool) -> Self {
        let name = std::any::type_name::<R>();
        let owner = std::thread::current().id();

        Self {
            name,
            owner: Some(owner),
            is_send_sync,
        }
    }

    pub fn name(&self) -> &'static str {
        self.name
    }

    pub fn owner(&self) -> Option<ThreadId> {
        self.owner.clone()
    }

    pub fn has_access(&self, thread: ThreadId) -> bool {
        self.is_send_sync || self.owner == Some(thread)
    }
}

pub(crate) struct ResourceInfo {
    data: Option<BlobCell>,
    meta: ResourceMeta,
}

impl ResourceInfo {
    pub fn new<R: Resource>(resource: R) -> Self {
        let data = Some(BlobCell::new(resource));
        let meta = ResourceMeta::new::<R>(true);

        Self { data, meta }
    }

    pub fn new_non_send_sync<R: Resource>(resource: R) -> Self {
        let data = Some(BlobCell::new(resource));
        let meta = ResourceMeta::new::<R>(false);

        Self { data, meta }
    }

    pub fn as_ref<R: Resource>(&self) -> &R {
        let value = self
            .data
            .as_ref()
            .expect(&format!("Resource {} no longer exists", self.meta.name));
        value.value::<R>()
    }

    pub fn as_mut<R: Resource>(&mut self) -> &mut R {
        let value = self
            .data
            .as_mut()
            .expect(&format!("Resource {} no longer exists", self.meta.name));
        value.value_mut::<R>()
    }

    pub fn take<R: Resource>(&mut self) -> R {
        let data = self
            .data
            .take()
            .expect(&format!("Resource {} no longer exists", self.meta.name));
        data.into()
    }
}

impl Drop for ResourceInfo {
    fn drop(&mut self) {
        let id = std::thread::current().id();
        if !self.meta.has_access(id) && !std::thread::panicking() {
            let name = self.meta.name();
            let owner = self.meta.owner();
            panic!("Dopping a non-send resource {} that is owned by thread {:?} from thread {:?} is not allowed.", name, owner, id);
        }
    }
}

pub struct Resources<const SEND: bool> {
    resources: HashMap<ResourceId, ResourceInfo>,
}

impl<const SEND: bool> Resources<SEND> {
    pub fn new() -> Self {
        Self {
            resources: HashMap::new(),
        }
    }

    pub fn add<R: Resource>(&mut self, resource: R) {
        let id = ResourceId::of::<R>();
        let info = if SEND {
            ResourceInfo::new(resource)
        } else {
            ResourceInfo::new_non_send_sync(resource)
        };

        self.resources.insert(id, info);
    }

    pub fn get<R: Resource>(&self) -> &R {
        self.try_get().expect(&format!(
            "Resource {} not found",
            std::any::type_name::<R>()
        ))
    }

    pub fn get_mut<R: Resource>(&mut self) -> &mut R {
        self.try_get_mut().expect(&format!(
            "Resource {} not found",
            std::any::type_name::<R>()
        ))
    }

    pub fn try_get<R: Resource>(&self) -> Option<&R> {
        let id = ResourceId::of::<R>();
        self.resources.get(&id).map(|info| info.as_ref())
    }

    pub fn try_get_mut<R: Resource>(&mut self) -> Option<&mut R> {
        let id = ResourceId::of::<R>();
        self.resources.get_mut(&id).map(|info| info.as_mut())
    }

    pub fn remove<R: Resource>(&mut self) -> Option<R> {
        let id = ResourceId::of::<R>();
        self.resources.get_mut(&id).map(|info| info.take())
    }

    pub fn contains<R: Resource>(&self) -> bool {
        let id = ResourceId::of::<R>();
        self.resources.contains_key(&id)
    }
}

pub struct Res<'a, R: Resource> {
    ptr: Ptr<'a, R>,
}

impl<'a, R: Resource> Res<'a, R> {
    pub fn new(ptr: Ptr<'a, R>) -> Self {
        Self { ptr }
    }
}

impl<'a, R: Resource> std::ops::Deref for Res<'a, R> {
    type Target = R;

    fn deref(&self) -> &Self::Target {
        &self.ptr
    }
}

pub struct ResMut<'a, R: Resource> {
    ptr: Ptr<'a, R>,
}

impl<'a, R: Resource> std::ops::Deref for ResMut<'a, R> {
    type Target = R;

    fn deref(&self) -> &Self::Target {
        &self.ptr
    }
}

impl<'a, R: Resource> std::ops::DerefMut for ResMut<'a, R> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.ptr
    }
}
