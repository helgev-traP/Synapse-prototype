use std::{default, sync::Arc};

use crate::{
    context::SharedContext,
    events::UiEvent,
    renderer::Renderer,
    types::size::{Size, StdSize},
    ui::{Dom, DomComPareResult, Widget},
    vertex::{
        colored_vertex::ColoredVertex, uv_vertex::UvVertex, vertex_generator::RectangleDescriptor,
    },
};

// todo: organize modules and public uses.

// style
pub mod style;
use style::{border, BoxSizing, Style, Visibility};

// layout
pub mod layout;
use layout::{Layout, LayoutNode};
use vello::skrifa::color;
use wgpu::naga::back;

#[derive(Default)]
pub struct ContainerDescriptor<T: 'static> {
    pub label: Option<String>,
    // style of the container itself
    pub style: Style,
    // layout of the child elements
    pub layout: Layout<T>,
}
pub struct Container<T: 'static> {
    label: Option<String>,
    style: Style,
    layout: Layout<T>,
}

impl<T> Container<T> {
    pub fn new(disc: ContainerDescriptor<T>) -> Box<Self> {
        Box::new(Self {
            label: disc.label,
            style: disc.style,
            layout: disc.layout,
        })
    }
}

impl<T: Send + 'static> Dom<T> for Container<T> {
    fn build_widget_tree(&self) -> Box<dyn Widget<T>> {
        Box::new(ContainerNode {
            label: self.label.clone(),
            style: self.style.clone(),
            layout: self.layout.build(),
            scene: vello::Scene::new(),
            texture: None,
            vertices: None,
            indices: None,
        })
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

pub struct ContainerNode<T> {
    // entity info
    label: Option<String>,
    style: Style,
    layout: LayoutNode<T>,

    // vello scene
    scene: vello::Scene,

    // texture, vertices, indices
    texture: Option<Arc<wgpu::Texture>>,
    vertices: Option<Arc<Vec<UvVertex>>>,
    indices: Option<Arc<Vec<u16>>>,
}

impl<T: Send + 'static> Widget<T> for ContainerNode<T> {
    fn label(&self) -> Option<&str> {
        todo!()
    }

    fn update_widget_tree(&mut self, dom: &dyn Dom<T>) -> Result<(), ()> {
        todo!()
    }

    fn compare(&self, dom: &dyn Dom<T>) -> DomComPareResult {
        todo!()
    }

    fn widget_event(
        &mut self,
        event: &UiEvent,
        parent_size: [StdSize; 2],
        context: &SharedContext,
    ) -> crate::events::UiEventResult<T> {
        todo!()
    }

    fn size(&self) -> [Size; 2] {
        todo!()
    }

    fn px_size(&self, parent_size: [StdSize; 2], context: &SharedContext) -> [f32; 2] {
        todo!()
    }

    fn default_size(&self) -> [f32; 2] {
        todo!()
    }

    fn render(
        &mut self,
        // ui environment
        parent_size: [StdSize; 2],
        // context
        context: &SharedContext,
        renderer: &Renderer,
        frame: u64,
    ) -> Vec<(
        Arc<wgpu::Texture>,
        Arc<Vec<UvVertex>>,
        Arc<Vec<u16>>,
        nalgebra::Matrix4<f32>,
    )> {
        todo!()
    }
}
