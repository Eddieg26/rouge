use super::{Attribute, Node, NodeOutput};

pub struct SampleTexture2D {
    name: String,
    inputs: [Attribute; 2],
    outputs: [NodeOutput; 5],
}

impl SampleTexture2D {
    pub const TEXTURE_SLOT: usize = 0;
    pub const UV_SLOT: usize = 1;

    pub fn new(name: &str) -> SampleTexture2D {
        SampleTexture2D {
            name: name.to_string(),
            inputs: [Attribute::Texture2D, Attribute::Vec2],
            outputs: [
                NodeOutput::new(&format!("{}", &name), Attribute::Vec4),
                NodeOutput::new(&format!("{}_r", &name), Attribute::Float),
                NodeOutput::new(&format!("{}_g", &name), Attribute::Float),
                NodeOutput::new(&format!("{}_b", &name), Attribute::Float),
                NodeOutput::new(&format!("{}_a", &name), Attribute::Float),
            ],
        }
    }
}

impl Node for SampleTexture2D {
    fn name(&self) -> &str {
        &self.name
    }

    fn input(&self, index: usize) -> Option<&Attribute> {
        self.inputs.get(index)
    }

    fn output(&self, index: usize) -> Option<&NodeOutput> {
        self.outputs.get(index)
    }

    fn run(&self, inputs: &[super::NodeInput]) -> String {
        let texture = inputs.get(0).expect("Texture2D input not found");
        let uv = inputs.get(1).expect("UV input not found");

        format!(
            r#"
                let {rgba} = textureSample({texture}, {sampler}, {uv});
                let {r} = {rgba}.r;
                let {g} = {rgba}.g;
                let {b} = {rgba}.b;
                let {a} = {rgba}.a;
            "#,
            rgba = self.outputs[0].name(),
            r = self.outputs[1].name(),
            g = self.outputs[2].name(),
            b = self.outputs[3].name(),
            a = self.outputs[4].name(),
            texture = texture.name(),
            sampler = format!("{}_sampler", texture.name()),
            uv = uv.name(),
        )
    }
}
