use self::attribute::{Attribute, BufferProperty, PropertyBlock};
use crate::{
    ecs::resource::ResourceId,
    graphics::resources::{
        material::{BlendMode, ShaderModel},
        TextureId,
    },
};
use itertools::Itertools;
use std::{
    collections::HashMap,
    hash::{Hash, Hasher},
};

pub mod attribute;
pub mod nodes;

pub type NodeId = ResourceId;

#[derive(Clone, Debug, PartialEq, Eq, Ord)]
pub struct ShaderInput {
    name: String,
    attribute: Attribute,
}

impl ShaderInput {
    pub fn new(name: &str, attribute: Attribute) -> ShaderInput {
        ShaderInput {
            name: name.to_string(),
            attribute,
        }
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn attribute(&self) -> &Attribute {
        &self.attribute
    }

    pub fn definition(&self, prefix: &str) -> String {
        self.attribute.definition(&self.name, prefix)
    }
}

impl PartialOrd for ShaderInput {
    fn partial_cmp(&self, other: &ShaderInput) -> Option<std::cmp::Ordering> {
        if self.attribute == other.attribute {
            Some(self.name.cmp(&other.name))
        } else {
            Some(self.attribute.size().cmp(&other.attribute.size()))
        }
    }
}

#[derive(Clone, Debug)]
pub struct ShaderOutput {
    name: String,
    location: usize,
}

impl ShaderOutput {
    pub fn new(name: &str, location: usize) -> ShaderOutput {
        ShaderOutput {
            name: name.to_string(),
            location,
        }
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn location(&self) -> usize {
        self.location
    }

    pub fn definition(&self) -> String {
        format!(
            "@location({}) var {} : vec4<f32>;\n",
            self.location, self.name
        )
    }
}

pub struct NodeInput {
    name: String,
    attribute: Attribute,
    index: usize,
}

impl NodeInput {
    pub fn new(name: &str, attribute: Attribute, index: usize) -> NodeInput {
        NodeInput {
            name: name.to_string(),
            attribute,
            index,
        }
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn attribute(&self) -> &Attribute {
        &self.attribute
    }

    pub fn index(&self) -> usize {
        self.index
    }

    pub fn cast(&self, attribute: &Attribute) -> String {
        self.attribute.cast(&self.name, attribute)
    }
}

pub struct NodeOutput {
    name: String,
    attribute: Attribute,
}

impl NodeOutput {
    pub fn new(name: &str, attribute: Attribute) -> NodeOutput {
        NodeOutput {
            name: name.to_string(),
            attribute,
        }
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn attribute(&self) -> &Attribute {
        &self.attribute
    }

    pub fn cast(&self, attribute: &Attribute) -> String {
        self.attribute.cast(&self.name, attribute)
    }
}

pub trait Node: 'static {
    fn name(&self) -> &str;
    fn input(&self, index: usize) -> Option<&Attribute>;
    fn output(&self, index: usize) -> Option<&NodeOutput>;
    fn run(&self, inputs: &[NodeInput]) -> String;
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Slot {
    node: NodeId,
    output: usize,
}

impl Slot {
    pub fn new(node: impl Into<NodeId>, output: usize) -> Slot {
        Slot {
            node: node.into(),
            output,
        }
    }

    pub fn node(&self) -> NodeId {
        self.node
    }

    pub fn output(&self) -> usize {
        self.output
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct EdgeId(u64);

impl EdgeId {
    pub fn new(source: &Slot, target: &Slot) -> EdgeId {
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        source.hash(&mut hasher);
        target.hash(&mut hasher);

        EdgeId(hasher.finish())
    }
}

impl std::ops::Deref for EdgeId {
    type Target = u64;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl std::ops::DerefMut for EdgeId {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

pub struct Edge {
    id: EdgeId,
    source: Slot,
    target: Slot,
}

impl Edge {
    pub fn new(source: Slot, target: Slot) -> Edge {
        Edge {
            id: EdgeId::new(&source, &target),
            source,
            target,
        }
    }

    pub fn id(&self) -> &EdgeId {
        &self.id
    }

    pub fn source(&self) -> &Slot {
        &self.source
    }

    pub fn target(&self) -> &Slot {
        &self.target
    }
}

pub struct ShaderConfig {
    model: ShaderModel,
    blend_mode: BlendMode,
}

impl Default for ShaderConfig {
    fn default() -> Self {
        Self {
            model: ShaderModel::Unlit,
            blend_mode: BlendMode::Opaque,
        }
    }
}

impl ShaderConfig {
    pub fn new(model: ShaderModel, blend_mode: BlendMode) -> ShaderConfig {
        ShaderConfig { model, blend_mode }
    }

    pub fn model(&self) -> &ShaderModel {
        &self.model
    }

    pub fn blend_mode(&self) -> &BlendMode {
        &self.blend_mode
    }

    pub fn set_model(&mut self, model: ShaderModel) {
        self.model = model;
    }

    pub fn set_blend_mode(&mut self, blend_mode: BlendMode) {
        self.blend_mode = blend_mode;
    }
}

pub struct ShaderConstants;

impl ShaderConstants {
    pub const GLOBAL_BIND_GROUP: usize = 0;
    pub const OBJECT_BIND_GROUP: usize = 1;
    pub const MATERIAL_BIND_GROUP: usize = 2;

    pub const GLOBAL_BINDING: usize = 0;
    pub const MATERIAL_BINDING: usize = 0;
    pub const OBJECT_BINDING: usize = 0;
    pub const LIGHTS_BINDING: usize = 1;
}

pub struct ShaderGraph {
    config: ShaderConfig,
    nodes: Vec<Box<dyn Node>>,
    edges: HashMap<EdgeId, Edge>,
    inputs: Vec<ShaderInput>,
    outputs: Vec<ShaderOutput>,
}

impl Default for ShaderGraph {
    fn default() -> Self {
        Self::new(ShaderConfig::default())
    }
}

impl ShaderGraph {
    pub fn new(config: ShaderConfig) -> ShaderGraph {
        ShaderGraph {
            config,
            nodes: Vec::new(),
            edges: HashMap::new(),
            inputs: Vec::new(),
            outputs: Vec::new(),
        }
    }

    pub fn config(&self) -> &ShaderConfig {
        &self.config
    }

    pub fn config_mut(&mut self) -> &mut ShaderConfig {
        &mut self.config
    }

    pub fn property_block(&self) -> PropertyBlock {
        let mut block = PropertyBlock::new();
        let mut inputs = self.inputs.iter().collect_vec();
        inputs.sort();

        for input in &inputs {
            if input.attribute.is_texture() {
                let dimension = match input.attribute {
                    Attribute::Texture2D => wgpu::TextureViewDimension::D2,
                    Attribute::Texture3D => wgpu::TextureViewDimension::D3,
                    Attribute::CubeMap => wgpu::TextureViewDimension::Cube,
                    Attribute::Texture2DArray => wgpu::TextureViewDimension::D2Array,
                    _ => panic!("Invalid texture attribute"),
                };
                block.set_texture(input.name(), &TextureId::zero(), dimension);
            } else {
                block.set_input(BufferProperty::new(input.name(), input.attribute().into()));
            }
        }

        block
    }

    pub fn add_input(&mut self, name: &str, attribute: Attribute) {
        if self.inputs.iter().any(|i| i.name() == name) {
            panic!("Input with name {} already exists", name);
        }

        self.inputs.push(ShaderInput::new(name, attribute));
        self.nodes
            .push(Box::new(ShaderInputNode::new(name, attribute)));
    }

    pub fn add_output(&mut self, name: &str, location: usize) {
        if self.outputs.iter().any(|i| i.name() == name) {
            panic!("Output with name {} already exists", name);
        }

        self.outputs.push(ShaderOutput::new(name, location));
        self.nodes.push(Box::new(ShaderOutputNode::new(name)));
    }

    pub fn add_node<T: Node>(&mut self, node: T) -> NodeId {
        if self.nodes.iter().any(|n| n.name() == node.name()) {
            panic!("Node with name {} already exists", node.name());
        }

        let id = node.name().into();
        self.nodes.push(Box::new(node));

        id
    }

    pub fn add_dyn_node(&mut self, node: Box<dyn Node>) -> NodeId {
        if self.nodes.iter().any(|n| n.name() == node.name()) {
            panic!("Node with name {} already exists", node.name());
        }

        let id = node.name().into();
        self.nodes.push(node);

        id
    }

    pub fn add_edge(&mut self, source: Slot, target: Slot) -> &Edge {
        let edge = Edge::new(source, target);
        let id = *edge.id();
        self.edges.insert(id, edge);

        self.edges.get(&id).unwrap()
    }

    pub fn remove_edge(&mut self, edge: &Edge) {
        self.edges.remove(edge.id());
    }

    pub fn remove_node(&mut self, node: NodeId) {
        self.nodes.retain(|n| NodeId::from(n.name()) != node);
    }

    pub fn remove_input(&mut self, name: &str) {
        self.inputs.retain(|i| i.name() != name);
        self.remove_node(name.into());
    }

    pub fn remove_output(&mut self, name: &str) {
        self.outputs.retain(|o| o.name() != name);
        self.remove_node(name.into());
    }

    pub fn generate(&self) -> String {
        let mut nodes = self.nodes.iter().collect_vec();
        let mut sorted = vec![];

        while !nodes.is_empty() {
            let removed = nodes
                .iter()
                .filter_map(|node| {
                    let target_edges = self
                        .edges
                        .values()
                        .filter(|e| e.target().node() == NodeId::from(node.name()))
                        .collect_vec();

                    if target_edges.is_empty() {
                        Some(node.name())
                    } else {
                        if target_edges.iter().any(|edge| {
                            nodes
                                .iter()
                                .any(|n| NodeId::from(n.name()) == edge.source().node())
                        }) {
                            None
                        } else {
                            Some(node.name())
                        }
                    }
                })
                .collect_vec();

            if removed.is_empty() {
                panic!("Cyclic dependency detected");
            }

            for name in &removed {
                let index = nodes
                    .iter()
                    .position(|n| n.name() == *name)
                    .expect("Node not found");
                sorted.push(nodes.remove(index));
            }
        }

        let texture_inputs = self
            .inputs
            .iter()
            .filter(|i| i.attribute().is_texture())
            .collect_vec();

        let shader_inputs = self
            .inputs
            .iter()
            .filter(|i| !i.attribute().is_texture())
            .map(|i| i.definition(""))
            .collect_vec()
            .join("\n");

        let shader_outputs = self
            .outputs
            .iter()
            .map(|o| o.definition())
            .collect_vec()
            .join("\n");

        let has_shader_inputs = !shader_inputs.is_empty();
        let start_binding = if has_shader_inputs { 1 } else { 0 };

        let texture_bindings = texture_inputs
            .iter()
            .enumerate()
            .map(|(index, input)| {
                let prefix = format!(
                    "@group({}) @binding({})",
                    ShaderConstants::MATERIAL_BIND_GROUP,
                    start_binding + index
                );
                input.attribute.definition(input.name(), &prefix)
            })
            .collect_vec()
            .join("\n");

        let sampler_bindings = texture_inputs
            .iter()
            .enumerate()
            .map(|(index, input)| {
                let binding = start_binding + texture_inputs.len() + index;
                format!(
                    "@group({}) @binding({}) var {}_sampler : sampler;\n",
                    ShaderConstants::MATERIAL_BIND_GROUP,
                    binding,
                    input.name()
                )
            })
            .collect_vec()
            .join("\n");

        let shader_input_block = if has_shader_inputs {
            format!(
                r#"
                    struct ShaderInputs {{
                        {}
                    }}

                    @group({}) @binding({})
                    var<uniform> shader_inputs : ShaderInputs;
                "#,
                shader_inputs,
                ShaderConstants::MATERIAL_BIND_GROUP,
                ShaderConstants::MATERIAL_BINDING
            )
        } else {
            String::new()
        };

        let vertex_shader_definition = format!(
            r#"
                struct Camera {{
                    view : mat4x4<f32>;
                    projection : mat4x4<f32>;
                }}

                struct Globals {{
                    camera: Camera;
                }}

                struct Object {{
                    model: mat4x4<f32>;
                    normal: mat3x3<f32>;
                }}

                struct VertexInput {{
                    @location(0) position: vec4<f32>;
                    @location(1) normal: vec3<f32>;
                    @location(2) uv: vec2<f32>;
                }}

                struct VertexOutput {{
                    @builtin(position) position: vec4<f32>;
                    @location(0) normal: vec3<f32>;
                    @location(1) uv: vec2<f32>;
                }}

                @group({GLOBAL_GROUP}) @binding({GLOBAL_BINDING})
                var<uniform> globals : Globals;

                @group({OBJECT_GROUP}) @binding({OBJECT_BINDING})
                var<uniform> object : Object;

                @vertex
                fn vs_main(input: VertexInput) -> VertexOutput {{
                    var output: VertexOutput;

                    output.position = globals.camera.view * globals.camera.proj * object.model * input.position;
                    output.normal = input.normal;
                    output.uv = input.uv;

                    return output;
                }}
            "#,
            GLOBAL_GROUP = ShaderConstants::GLOBAL_BIND_GROUP,
            GLOBAL_BINDING = ShaderConstants::GLOBAL_BINDING,
            OBJECT_GROUP = ShaderConstants::OBJECT_BIND_GROUP,
            OBJECT_BINDING = ShaderConstants::OBJECT_BINDING
        );

        let lights_definition = if matches!(self.config.model, ShaderModel::Unlit) {
            String::new()
        } else {
            format!(
                r#"
                    struct Light {{
                        position : vec3<f32>;
                        direction : vec3<f32>;
                        color : vec3<f32>;
                        intensity : f32;
                        range : f32;
                        kind : u32;
                        angle : f32;
                    }}
    
                    const MAX_LIGHTS = {MAX_LIGHTS};
    
                    struct LightArray {{
                        lights: array<Light, MAX_LIGHTS>;
                    }}
    
                    @group({LIGHTS_GROUP}) @binding({LIGHTS_BINDING})
                    var<uniform> lights : Lights;
                "#,
                MAX_LIGHTS = 16,
                LIGHTS_GROUP = ShaderConstants::GLOBAL_BIND_GROUP,
                LIGHTS_BINDING = ShaderConstants::LIGHTS_BINDING
            )
        };

        let mut shader_block = String::new();
        shader_block.push_str(&vertex_shader_definition);
        shader_block.push_str(&lights_definition);
        shader_block.push_str(&shader_input_block);
        shader_block.push_str(&texture_bindings);
        shader_block.push_str(&sampler_bindings);
        shader_block.push_str(&shader_outputs);

        let mut fs_inner_block = String::new();
        for node in &sorted {
            let mut inputs = Vec::new();
            for edge in self.edges.values() {
                if edge.target().node() == NodeId::from(node.name()) {
                    let source = edge.source();
                    let slot = source.output();
                    let source_node = self
                        .nodes
                        .iter()
                        .find(|n| NodeId::from(n.name()) == source.node())
                        .expect("Source node not found");
                    let source = source_node
                        .output(source.output())
                        .expect("Source output not found");

                    let input = NodeInput::new(source.name(), source.attribute, slot);
                    inputs.push(input);
                }
            }

            inputs.sort_by(|a, b| a.index().cmp(&b.index()));

            fs_inner_block.push_str(&node.run(&inputs));
        }

        let fs_entry_block = format!(
            r#"
                    fn fs_main(input: VertexOutput) {{
                        {}
                    }}
                "#,
            fs_inner_block
        );

        shader_block.push_str(&fs_entry_block);

        shader_block
    }
}

pub struct ShaderInputNode {
    name: String,
    attribute: Attribute,
    output: NodeOutput,
}

impl ShaderInputNode {
    pub fn new(name: &str, attribute: Attribute) -> ShaderInputNode {
        ShaderInputNode {
            name: name.to_string(),
            attribute,
            output: NodeOutput::new(name, attribute),
        }
    }
}

impl Node for ShaderInputNode {
    fn name(&self) -> &str {
        &self.name
    }

    fn output(&self, index: usize) -> Option<&NodeOutput> {
        if index == 0 {
            Some(&self.output)
        } else {
            None
        }
    }

    fn run(&self, _inputs: &[NodeInput]) -> String {
        match self.attribute {
            Attribute::Texture2D
            | Attribute::Texture3D
            | Attribute::CubeMap
            | Attribute::Texture2DArray => String::new(),
            _ => format!("var {} = shader_inputs.{};\n", self.name, self.name),
        }
    }

    fn input(&self, _index: usize) -> Option<&Attribute> {
        None
    }
}

impl From<ShaderInput> for ShaderInputNode {
    fn from(input: ShaderInput) -> ShaderInputNode {
        ShaderInputNode::new(input.name(), *input.attribute())
    }
}

impl From<ShaderInputNode> for Box<dyn Node> {
    fn from(node: ShaderInputNode) -> Box<dyn Node> {
        Box::new(node)
    }
}

pub struct ShaderOutputNode {
    name: String,
    input: NodeInput,
}

impl ShaderOutputNode {
    pub fn new(name: &str) -> ShaderOutputNode {
        ShaderOutputNode {
            name: name.to_string(),
            input: NodeInput::new(name, Attribute::Color, 0),
        }
    }
}

impl Node for ShaderOutputNode {
    fn name(&self) -> &str {
        &self.name
    }

    fn output(&self, _index: usize) -> Option<&NodeOutput> {
        None
    }

    fn run(&self, inputs: &[NodeInput]) -> String {
        let input = inputs[0].cast(&self.input.attribute);
        format!("{} = {};\n", self.name, input)
    }

    fn input(&self, index: usize) -> Option<&Attribute> {
        if index == 0 {
            Some(&self.input.attribute)
        } else {
            None
        }
    }
}

impl From<ShaderOutput> for ShaderOutputNode {
    fn from(output: ShaderOutput) -> ShaderOutputNode {
        ShaderOutputNode::new(output.name())
    }
}

impl From<ShaderOutputNode> for Box<dyn Node> {
    fn from(node: ShaderOutputNode) -> Box<dyn Node> {
        Box::new(node)
    }
}
