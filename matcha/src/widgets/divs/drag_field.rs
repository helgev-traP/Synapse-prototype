use std::sync::Arc;

use crate::{
    context::SharedContext,
    device::mouse::MouseButton,
    events::UiEvent,
    types::size::Size,
    ui::{Dom, DomComPareResult, Widget},
};

use nalgebra as na;

pub struct DragFieldDescriptor<R> {
    pub label: Option<String>,
    pub size: [Size; 2],

    // item
    pub item: Box<dyn Dom<R>>,
}

pub struct DragField<T> {
    label: Option<String>,
    size: [Size; 2],

    item: Box<dyn Dom<T>>,
}

impl<T> DragField<T> {
    pub fn new(disc: DragFieldDescriptor<T>) -> Box<Self> {
        Box::new(Self {
            label: disc.label,
            size: disc.size,
            item: disc.item,
        })
    }
}

impl<T: Send + 'static> Dom<T> for DragField<T> {
    fn build_widget_tree(&self) -> Box<dyn Widget<T>> {
        Box::new(DragFieldNode {
            label: self.label.clone(),
            size: self.size,

            item_position: [0.0, 0.0],
            drag_delta: None,
            item: self.item.build_widget_tree(),
        })
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

pub struct DragFieldNode<T> {
    label: Option<String>,
    size: [Size; 2],

    item_position: [f32; 2],
    drag_delta: Option<[f32; 2]>,
    item: Box<dyn Widget<T>>,
}

impl<T: Send + 'static> Widget<T> for DragFieldNode<T> {
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
        parent_size: [crate::types::size::StdSize; 2],
        context: &SharedContext,
    ) -> crate::events::UiEventResult<T> {
        todo!()
    }

    fn size(&self) -> [Size; 2] {
        todo!()
    }

    fn px_size(&self, parent_size: [crate::types::size::StdSize; 2], context: &SharedContext) -> [f32; 2] {
        todo!()
    }

    fn default_size(&self) -> [f32; 2] {
        todo!()
    }

    fn render(
        &mut self,
        // ui environment
        parent_size: [crate::types::size::StdSize; 2],
        // context
        context: &SharedContext,
        renderer: &crate::renderer::Renderer,
        frame: u64,
    ) -> Vec<(
        Arc<wgpu::Texture>,
        Arc<Vec<crate::vertex::uv_vertex::UvVertex>>,
        Arc<Vec<u16>>,
        nalgebra::Matrix4<f32>,
    )> {
        todo!()
    }
}
