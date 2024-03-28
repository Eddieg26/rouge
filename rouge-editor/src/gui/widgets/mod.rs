use super::{
    attributes::{Attributes, Properties},
    context::Context,
    core::ElementId,
    elements::Element,
    event::{BaseListeners, Event, EventListeners, EventType},
};
use std::collections::HashMap;

pub trait Widget {
    type Props;

    fn render(id: &ElementId, ctx: &mut Context, props: Self::Props) -> View;
}

pub trait IntoView {
    fn into_view(self, id: &ElementId, attributes: Attributes) -> View;
}

impl<E: Element> IntoView for E {
    fn into_view(self, id: &ElementId, attributes: Attributes) -> View {
        View::new(id, self, attributes)
    }
}

pub struct View {
    id: ElementId,
    attributes: Attributes,
    properties: Properties,
    element: Box<dyn Element>,
    children: Vec<View>,
    event_listeners: HashMap<EventType, Box<dyn BaseListeners>>,
}

impl View {
    pub fn new<E: Element>(id: &ElementId, element: E, attributes: Attributes) -> Self {
        Self {
            id: id.clone(),
            attributes,
            properties: Properties::default(),
            element: Box::new(element),
            children: Vec::new(),
            event_listeners: HashMap::new(),
        }
    }

    pub fn id(&self) -> &ElementId {
        &self.id
    }

    pub fn attributes(&self) -> &Attributes {
        &self.attributes
    }

    pub fn properties(&self) -> &Properties {
        &self.properties
    }

    pub fn element(&self) -> &dyn Element {
        &*self.element
    }

    pub fn element_as<E: Element>(&self) -> Option<&E> {
        self.element.as_any().downcast_ref::<E>()
    }

    pub fn add_child(mut self, child: View) -> Self {
        self.children.push(child);
        self
    }

    pub fn add_element<E: Element>(mut self, element: E, attributes: Attributes) -> Self {
        let id = self.id.child(&self.children.len().to_string());
        self.children.push(View::new(&id, element, attributes));
        self
    }

    pub fn add_widget<W: Widget>(mut self, ctx: &mut Context, props: W::Props) -> Self {
        let id = self.id.child(&self.children.len().to_string());
        self.children.push(W::render(&id, ctx, props));
        self
    }

    pub fn on<E: Event>(mut self, listener: impl Fn(&E) + Send + Sync + 'static) -> Self {
        let event_type = EventType::new::<E>();
        let listeners = self
            .event_listeners
            .entry(event_type)
            .or_insert_with(|| Box::new(EventListeners::<E>::new()));
        listeners
            .as_any_mut()
            .downcast_mut::<EventListeners<E>>()
            .unwrap()
            .add_listener(listener);
        self
    }
}

// pub struct OuterWidget;

// impl Widget for OuterWidget {
//     type Props = ();

//     fn render(id: &ElementId, ctx: &mut Context, props: Self::Props) -> View {
//         Quad::new()
//             .into_view(id, Attributes::new((100u32, 100u32).into()))
//             .add_widget::<InnerWidget>(ctx, true)
//             .on::<Click>(|event| println!("Clicked"))
//     }
// }

// pub struct InnerWidget;

// impl Widget for InnerWidget {
//     type Props = bool;

//     fn render(id: &ElementId, ctx: &mut Context, props: Self::Props) -> View {
//         Checkbox::new(props).into_view(id, Attributes::new((100u32, 100u32).into()))
//     }
// }
