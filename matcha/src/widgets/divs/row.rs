use nalgebra as na;
use std::{cell::Cell, sync::Arc};

use crate::{
    context::SharedContext,
    events::UiEvent,
    renderer::Renderer,
    types::size::{Size, StdSize},
    ui::{Dom, DomComPareResult, Widget},
    vertex::uv_vertex::UvVertex,
};

pub struct RowDescriptor<R> {
    pub label: Option<String>,
    pub vec: Vec<Box<dyn Dom<R>>>,
}

impl<R> Default for RowDescriptor<R> {
    fn default() -> Self {
        Self {
            label: None,
            vec: Vec::new(),
        }
    }
}

pub struct Row<R: 'static> {
    label: Option<String>,
    children: Vec<Box<dyn Dom<R>>>,
}

impl<R> Row<R> {
    pub fn new(disc: RowDescriptor<R>) -> Box<Self> {
        Box::new(Self {
            label: disc.label,
            children: disc.vec,
        })
    }

    pub fn push(&mut self, child: Box<dyn Dom<R>>) {
        self.children.push(child);
    }
}

impl<R: Send + 'static> Dom<R> for Row<R> {
    fn build_widget_tree(&self) -> Box<dyn Widget<R>> {
        Box::new(RowRenderNode {
            label: self.label.clone(),
            redraw: true,
            children: self
                .children
                .iter()
                .map(|child| Child {
                    item: child.build_widget_tree(),
                    position: None,
                    size: None,
                })
                .collect(),
            cache_self_size: Cell::new(None),
            mouse_hovering_index: None,
        })
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

pub struct RowRenderNode<T: 'static> {
    label: Option<String>,
    redraw: bool,
    children: Vec<Child<T>>,
    cache_self_size: Cell<Option<[f32; 2]>>,
    mouse_hovering_index: Option<usize>,
}

struct Child<T> {
    item: Box<dyn Widget<T>>,
    // cache
    position: Option<[f32; 2]>,
    size: Option<[f32; 2]>,
}

impl<T> Widget<T> for RowRenderNode<T> {
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
