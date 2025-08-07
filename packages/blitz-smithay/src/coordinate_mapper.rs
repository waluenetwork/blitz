
use std::collections::HashMap;
use kurbo::Affine;
use tracing::debug;

use crate::ObjectId;

#[derive(Debug, Clone, Copy)]
pub struct Point<T, U = ()> {
    pub x: T,
    pub y: T,
    _phantom: std::marker::PhantomData<U>,
}

impl<T, U> Point<T, U> {
    pub fn new(x: T, y: T) -> Self {
        Self { x, y, _phantom: std::marker::PhantomData }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Size<T, U = ()> {
    pub w: T,
    pub h: T,
    _phantom: std::marker::PhantomData<U>,
}

impl<T, U> Size<T, U> {
    pub fn new(w: T, h: T) -> Self {
        Self { w, h, _phantom: std::marker::PhantomData }
    }
    
    pub fn from(tuple: (T, T)) -> Self {
        Self { w: tuple.0, h: tuple.1, _phantom: std::marker::PhantomData }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Rectangle<T, U = ()> 
where 
    T: Clone,
    U: Clone,
{
    pub loc: Point<T, U>,
    pub size: Size<T, U>,
    _phantom: std::marker::PhantomData<U>,
}

impl<T: Clone, U: Clone> Rectangle<T, U> {
    pub fn from_loc_and_size(loc: Point<T, U>, size: Size<T, U>) -> Self {
        Self { loc, size, _phantom: std::marker::PhantomData }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Transform {
    Normal,
    _90,
    _180,
    _270,
    Flipped,
    Flipped90,
    Flipped180,
    Flipped270,
}

#[derive(Debug, Clone, Copy)]
pub struct Logical;

#[derive(Debug, Clone, Copy)]
pub struct Physical;

pub struct CoordinateMapper {
    wayland_to_blitz: Affine,
    
    blitz_to_wayland: Affine,
    
    surface_transforms: HashMap<ObjectId, Transform>,
    
    output_transforms: HashMap<OutputId, Transform>,
    
    scale_factors: HashMap<OutputId, f64>,
    
    viewport_height: f64,
}

impl CoordinateMapper {
    pub fn new() -> Self {
        debug!("DEBUG: Initializing CoordinateMapper");
        
        let wayland_to_blitz = Affine::IDENTITY;
        let blitz_to_wayland = Affine::IDENTITY;
        
        Self {
            wayland_to_blitz,
            blitz_to_wayland,
            surface_transforms: HashMap::new(),
            output_transforms: HashMap::new(),
            scale_factors: HashMap::new(),
            viewport_height: 1080.0, // Default height
        }
    }
    
    pub fn set_viewport(&mut self, width: f64, height: f64) {
        debug!("DEBUG: Setting viewport dimensions {}x{}", width, height);
        
        self.viewport_height = height;
        
        self.update_transforms();
    }
    
    pub fn map_wayland_to_blitz(&self, point: Point<f64, Logical>, 
                               surface_id: Option<ObjectId>) -> Point<f64, Logical> {
        debug!("DEBUG: Mapping Wayland point {:?} to Blitz coordinates", point);
        
        let transformed_point = if let Some(id) = surface_id {
            self.apply_surface_transform(id, point)
        } else {
            point
        };
        
        let mapped = Point::new(
            transformed_point.x,
            self.viewport_height - transformed_point.y
        );
        
        debug!("DEBUG: Mapped point {:?} -> {:?}", point, mapped);
        mapped
    }
    
    pub fn map_wayland_rect_to_blitz(&self, rect: Rectangle<f64, Logical>,
                                    surface_id: Option<ObjectId>) -> Rectangle<f64, Logical> {
        debug!("DEBUG: Mapping Wayland rectangle {:?} to Blitz coordinates", rect);
        
        let mapped_loc = self.map_wayland_to_blitz(rect.loc, surface_id);
        
        let mapped_size = if let Some(id) = surface_id {
            self.apply_surface_transform_to_size(id, rect.size)
        } else {
            rect.size
        };
        
        let mapped_rect = Rectangle::from_loc_and_size(mapped_loc, mapped_size);
        debug!("DEBUG: Mapped rectangle {:?} -> {:?}", rect, mapped_rect);
        
        mapped_rect
    }
    
    pub fn set_surface_transform(&mut self, surface_id: ObjectId, transform: Transform) {
        debug!("DEBUG: Setting surface transform for {:?}: {:?}", surface_id, transform);
        self.surface_transforms.insert(surface_id, transform);
    }
    
    pub fn set_output_transform(&mut self, output_id: OutputId, transform: Transform) {
        debug!("DEBUG: Setting output transform for {:?}: {:?}", output_id, transform);
        self.output_transforms.insert(output_id, transform);
    }
    
    pub fn set_output_scale(&mut self, output_id: OutputId, scale: f64) {
        debug!("DEBUG: Setting output scale for {:?}: {}", output_id, scale);
        self.scale_factors.insert(output_id, scale);
    }
    
    pub fn create_transform_matrix(&self, transform: Transform, 
                                  size: Size<i32, Logical>) -> Affine {
        debug!("DEBUG: Creating transform matrix for {:?} with size {:?}", transform, size);
        
        let matrix = match transform {
            Transform::Normal => {
                debug!("DEBUG: Normal transform (identity)");
                Affine::IDENTITY
            }
            Transform::_90 => {
                debug!("DEBUG: 90° rotation transform");
                Affine::rotate(std::f64::consts::PI / 2.0)
                    .then_translate((size.h as f64, 0.0).into())
            }
            Transform::_180 => {
                debug!("DEBUG: 180° rotation transform");
                Affine::rotate(std::f64::consts::PI)
                    .then_translate((size.w as f64, size.h as f64).into())
            }
            Transform::_270 => {
                debug!("DEBUG: 270° rotation transform");
                Affine::rotate(3.0 * std::f64::consts::PI / 2.0)
                    .then_translate((0.0, size.w as f64).into())
            }
            Transform::Flipped => {
                debug!("DEBUG: Horizontal flip transform");
                Affine::scale_non_uniform(-1.0, 1.0)
                    .then_translate((size.w as f64, 0.0).into())
            }
            Transform::Flipped90 => {
                debug!("DEBUG: Horizontal flip + 90° rotation transform");
                Affine::scale_non_uniform(-1.0, 1.0)
                    .then_rotate(std::f64::consts::PI / 2.0)
                    .then_translate((size.h as f64, size.w as f64).into())
            }
            Transform::Flipped180 => {
                debug!("DEBUG: Horizontal flip + 180° rotation transform");
                Affine::scale_non_uniform(-1.0, 1.0)
                    .then_rotate(std::f64::consts::PI)
                    .then_translate((0.0, size.h as f64).into())
            }
            Transform::Flipped270 => {
                debug!("DEBUG: Horizontal flip + 270° rotation transform");
                Affine::scale_non_uniform(-1.0, 1.0)
                    .then_rotate(3.0 * std::f64::consts::PI / 2.0)
                    .then_translate((0.0, 0.0).into())
            }
        };
        
        debug!("DEBUG: Created transform matrix: {:?}", matrix);
        matrix
    }
    
    fn apply_surface_transform(&self, surface_id: ObjectId, 
                              point: Point<f64, Logical>) -> Point<f64, Logical> {
        if let Some(transform) = self.surface_transforms.get(&surface_id) {
            debug!("DEBUG: Applying surface transform {:?} to point {:?}", transform, point);
            
            match transform {
                Transform::Normal => point,
                Transform::_90 => Point::new(-point.y, point.x),
                Transform::_180 => Point::new(-point.x, -point.y),
                Transform::_270 => Point::new(point.y, -point.x),
                Transform::Flipped => Point::new(-point.x, point.y),
                Transform::Flipped90 => Point::new(-point.y, -point.x),
                Transform::Flipped180 => Point::new(point.x, -point.y),
                Transform::Flipped270 => Point::new(point.y, point.x),
            }
        } else {
            point
        }
    }
    
    fn apply_surface_transform_to_size(&self, surface_id: ObjectId,
                                      size: Size<f64, Logical>) -> Size<f64, Logical> {
        if let Some(transform) = self.surface_transforms.get(&surface_id) {
            debug!("DEBUG: Applying surface transform {:?} to size {:?}", transform, size);
            
            match transform {
                Transform::Normal | Transform::_180 | Transform::Flipped | Transform::Flipped180 => size,
                Transform::_90 | Transform::_270 | Transform::Flipped90 | Transform::Flipped270 => {
                    Size::from((size.h, size.w))
                }
            }
        } else {
            size
        }
    }
    
    fn update_transforms(&mut self) {
        debug!("DEBUG: Updating coordinate transforms for viewport height {}", self.viewport_height);
        
        self.wayland_to_blitz = Affine::scale_non_uniform(1.0, -1.0)
            .then_translate((0.0, self.viewport_height).into());
        
        self.blitz_to_wayland = self.wayland_to_blitz.inverse();
        
        debug!("DEBUG: Updated transforms - W2B: {:?}, B2W: {:?}", 
               self.wayland_to_blitz, self.blitz_to_wayland);
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct OutputId(u64);

impl OutputId {
    pub fn new() -> Self {
        use std::sync::atomic::{AtomicU64, Ordering};
        static COUNTER: AtomicU64 = AtomicU64::new(1);
        Self(COUNTER.fetch_add(1, Ordering::Relaxed))
    }
}
