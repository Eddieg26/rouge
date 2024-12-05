use ecs::system::schedule::Phase;

pub struct PreRender;
impl Phase for PreRender {}
pub struct Render;
impl Phase for Render {}
pub struct PostRender;
impl Phase for PostRender {}
pub struct Present;
impl Phase for Present {}
pub struct PostExtract;
impl Phase for PostExtract {}
