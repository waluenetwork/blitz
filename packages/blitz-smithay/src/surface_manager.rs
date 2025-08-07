
use std::collections::HashMap;
use tracing::debug;

use crate::{ObjectId, SurfaceError, BlitzTexture};
use crate::coordinate_mapper::{Transform, Rectangle};

pub struct WaylandSurfaceManager {
    surfaces: HashMap<ObjectId, WaylandSurface>,
    
    damage_tracker: SurfaceDamageTracker,
    
    layer_manager: LayerManager,
    
    subsurface_tree: SubsurfaceTree,
}

impl WaylandSurfaceManager {
    pub fn new() -> Self {
        debug!("DEBUG: Initializing WaylandSurfaceManager");
        
        Self {
            surfaces: HashMap::new(),
            damage_tracker: SurfaceDamageTracker::new(),
            layer_manager: LayerManager::new(),
            subsurface_tree: SubsurfaceTree::new(),
        }
    }
    
    pub fn commit_surface(&mut self, surface_id: ObjectId) -> Result<(), SurfaceError> {
        debug!("DEBUG: Committing surface {:?}", surface_id);
        
        let _surface = self.surfaces.get_mut(&surface_id)
            .ok_or(SurfaceError::SurfaceNotFound)?;
        
        debug!("DEBUG: Surface has new buffer (simplified)");
        
        self.damage_tracker.commit_surface_damage(surface_id);
        
        debug!("DEBUG: Surface commit successful");
        Ok(())
    }
    
    pub fn track_damage(&mut self, surface_id: ObjectId, damage: &[Rectangle<i32, smithay::utils::Logical>]) {
        debug!("DEBUG: Tracking damage for surface {:?} - {} rectangles", surface_id, damage.len());
        
        self.damage_tracker.track_surface_damage(surface_id, damage.to_vec());
    }
    
    pub fn add_surface(&mut self, surface_id: ObjectId) {
        debug!("DEBUG: Adding new surface {:?}", surface_id);
        
        let surface = WaylandSurface::new(surface_id);
        self.surfaces.insert(surface_id, surface);
    }
    
    pub fn remove_surface(&mut self, surface_id: ObjectId) {
        debug!("DEBUG: Removing surface {:?}", surface_id);
        
        self.surfaces.remove(&surface_id);
        self.damage_tracker.remove_surface(surface_id);
        self.subsurface_tree.remove_surface(surface_id);
    }
    
    pub fn surfaces(&self) -> impl Iterator<Item = &WaylandSurface> {
        self.surfaces.values()
    }
    
    fn find_surface_by_id(&self, surface_id: ObjectId) -> Option<&WaylandSurface> {
        self.surfaces.get(&surface_id)
    }
    
    pub fn render_all_surfaces(&self) -> Vec<RenderElement> {
        debug!("Rendering all surfaces in Z-order");
        
        let mut render_elements = Vec::new();
        
        let mut sorted_surfaces: Vec<_> = self.surfaces.values().collect();
        sorted_surfaces.sort_by_key(|surface| self.calculate_z_index(surface));
        
        for surface in sorted_surfaces {
            if surface.state == SurfaceState::Mapped {
                if let Some(element) = self.create_render_element(surface) {
                    render_elements.push(element);
                    debug!("Added surface {:?} to render queue with z_index={}", 
                           surface.id(), element.z_index);
                }
            }
        }
        
        debug!("Created {} render elements for multi-surface rendering", render_elements.len());
        render_elements
    }
    
    fn create_render_element(&self, surface: &WaylandSurface) -> Option<RenderElement> {
        debug!("Creating render element for surface {:?}", surface.id());
        
        if let Some(texture_source) = surface.texture_source() {
            if let TextureSource::Simple(simple_texture) = texture_source {
                if simple_texture.texture.wgpu_texture().is_some() {
                    let element = RenderElement {
                        surface_id: surface.id(),
                        texture_source: texture_source.clone(),
                        transform: surface.transform,
                        scale: surface.scale,
                        z_index: self.calculate_z_index(surface),
                        damage_regions: surface.get_damage_regions(),
                        opacity: surface.opacity,
                        blend_mode: surface.blend_mode,
                    };
                    
                    debug!("Created render element with z_index={} opacity={}", 
                           element.z_index, element.opacity);
                    return Some(element);
                }
            }
        }
        
        debug!("Surface {:?} has no valid texture source, skipping", surface.id());
        None
    }
    
    fn calculate_z_index(&self, surface: &WaylandSurface) -> i32 {
        match surface.state {
            SurfaceState::Mapped => 100,
            SurfaceState::Minimized => 0,
            SurfaceState::Unmapped => -1,
        }
    }
}

#[derive(Debug)]
pub struct WaylandSurface {
    id: ObjectId,
    texture_source: Option<TextureSource>,
    state: SurfaceState,
    transform: Transform,
    scale: f64,
    damage_regions: Vec<Rectangle<i32, i32>>,
    opacity: f32,
    blend_mode: BlendMode,
    buffer_age: u32,
}

impl WaylandSurface {
    fn new(id: ObjectId) -> Self {
        debug!("Creating new WaylandSurface {:?}", id);
        
        Self {
            id,
            texture_source: None,
            state: SurfaceState::Unmapped,
            transform: Transform::Normal,
            scale: 1.0,
            damage_regions: Vec::new(),
            opacity: 1.0,
            blend_mode: BlendMode::Normal,
            buffer_age: 0,
        }
    }
    
    pub fn id(&self) -> ObjectId {
        self.id
    }
    
    pub fn texture_source(&self) -> Option<&TextureSource> {
        self.texture_source.as_ref()
    }
    
    pub fn set_texture(&mut self, texture: BlitzTexture) {
        debug!("Setting texture for surface {:?}", self.id);
        self.texture_source = Some(TextureSource::Simple(SimpleTexture {
            texture,
        }));
        self.buffer_age += 1;
    }
    
    pub fn get_damage_regions(&self) -> Vec<Rectangle<i32, smithay::utils::Logical>> {
        self.damage_regions.clone()
    }
    
    pub fn add_damage_region(&mut self, region: Rectangle<i32, smithay::utils::Logical>) {
        debug!("Adding damage region {:?} to surface {:?}", region, self.id);
        self.damage_regions.push(region);
    }
    
    pub fn clear_damage_regions(&mut self) {
        self.damage_regions.clear();
    }
    
    pub fn set_opacity(&mut self, opacity: f32) {
        self.opacity = opacity.clamp(0.0, 1.0);
    }
    
    pub fn set_blend_mode(&mut self, blend_mode: BlendMode) {
        self.blend_mode = blend_mode;
    }
    
    pub fn map_surface(&mut self) {
        debug!("Mapping surface {:?}", self.id);
        self.state = SurfaceState::Mapped;
    }
    
    pub fn unmap_surface(&mut self) {
        debug!("Unmapping surface {:?}", self.id);
        self.state = SurfaceState::Unmapped;
        self.clear_damage_regions();
    }
}

#[derive(Debug, Clone)]
pub enum TextureSource {
    Simple(SimpleTexture),
}

#[derive(Debug, Clone)]
pub struct SimpleTexture {
    pub texture: BlitzTexture,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SurfaceState {
    Unmapped,
    Mapped,
    Minimized,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BlendMode {
    Normal,
    Multiply,
    Screen,
    Overlay,
    SoftLight,
    HardLight,
}

pub struct SurfaceDamageTracker {
    surface_damages: HashMap<ObjectId, Vec<Rectangle<i32, smithay::utils::Logical>>>,
    accumulated_damage: Vec<Rectangle<i32, smithay::utils::Logical>>,
}

impl SurfaceDamageTracker {
    fn new() -> Self {
        debug!("DEBUG: Initializing SurfaceDamageTracker");
        
        Self {
            surface_damages: HashMap::new(),
            accumulated_damage: Vec::new(),
        }
    }
    
    fn track_surface_damage(&mut self, surface_id: ObjectId, 
                           damage: Vec<Rectangle<i32, smithay::utils::Logical>>) {
        debug!("Tracking {} damage rectangles for surface {:?}", 
               damage.len(), surface_id);
        
        self.surface_damages.insert(surface_id, damage.clone());
        
        self.accumulated_damage.extend(damage);
        
        debug!("Total accumulated damage regions: {}", self.accumulated_damage.len());
    }
    
    fn commit_surface_damage(&mut self, surface_id: ObjectId) {
        debug!("Committing damage for surface {:?}", surface_id);
        
        if let Some(damage) = self.surface_damages.remove(&surface_id) {
            debug!("Committed {} damage regions for surface {:?}", damage.len(), surface_id);
        }
    }
    
    pub fn get_accumulated_damage(&self) -> &[Rectangle<i32, smithay::utils::Logical>] {
        &self.accumulated_damage
    }
    
    pub fn clear_accumulated_damage(&mut self) {
        debug!("Clearing accumulated damage ({} regions)", self.accumulated_damage.len());
        self.accumulated_damage.clear();
    }
    
    fn remove_surface(&mut self, surface_id: ObjectId) {
        debug!("DEBUG: Removing damage tracking for surface {:?}", surface_id);
        self.surface_damages.remove(&surface_id);
    }
}

pub struct LayerManager {
    background_layer: Vec<LayerSurface>,
    bottom_layer: Vec<LayerSurface>,
    top_layer: Vec<LayerSurface>,
    overlay_layer: Vec<LayerSurface>,
}

impl LayerManager {
    fn new() -> Self {
        debug!("DEBUG: Initializing LayerManager");
        
        Self {
            background_layer: Vec::new(),
            bottom_layer: Vec::new(),
            top_layer: Vec::new(),
            overlay_layer: Vec::new(),
        }
    }
    
    pub fn add_layer_surface(&mut self, surface: LayerSurface, layer: Layer) {
        debug!("DEBUG: Adding surface to layer {:?}", layer);
        
        match layer {
            Layer::Background => self.background_layer.push(surface),
            Layer::Bottom => self.bottom_layer.push(surface),
            Layer::Top => self.top_layer.push(surface),
            Layer::Overlay => self.overlay_layer.push(surface),
        }
    }
    
    pub fn render_layers_in_order(&self) -> Vec<LayerRenderElement> {
        debug!("DEBUG: Phase 3 - Rendering layers in wlr-layer-shell order");
        
        let mut elements = Vec::new();
        
        elements.extend(self.render_layer(&self.background_layer, Layer::Background));
        elements.extend(self.render_layer(&self.bottom_layer, Layer::Bottom));
        elements.extend(self.render_layer(&self.top_layer, Layer::Top));
        elements.extend(self.render_layer(&self.overlay_layer, Layer::Overlay));
        
        debug!("DEBUG: Generated {} layer render elements", elements.len());
        elements
    }
    
    fn render_layer(&self, surfaces: &[LayerSurface], layer: Layer) -> Vec<LayerRenderElement> {
        debug!("DEBUG: Rendering layer {:?} with {} surfaces", layer, surfaces.len());
        
        surfaces.iter().map(|surface| {
            LayerRenderElement {
                surface_id: surface.surface_id,
                layer: surface.layer,
                exclusive_zone: surface.exclusive_zone,
                z_index: self.layer_z_index(layer),
            }
        }).collect()
    }
    
    fn layer_z_index(&self, layer: Layer) -> i32 {
        match layer {
            Layer::Background => -1000,
            Layer::Bottom => -500,
            Layer::Top => 500,
            Layer::Overlay => 1000,
        }
    }
}

#[derive(Debug)]
pub struct LayerSurface {
    pub surface_id: ObjectId,
    pub layer: Layer,
    pub exclusive_zone: ExclusiveZone,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Layer {
    Background,
    Bottom,
    Top,
    Overlay,
}

#[derive(Debug, Clone, Copy)]
pub enum ExclusiveZone {
    Neutral,
    Exclusive(i32),
}

pub struct SubsurfaceTree {
    parent_child_map: HashMap<ObjectId, Vec<ObjectId>>,
    child_parent_map: HashMap<ObjectId, ObjectId>,
}

impl SubsurfaceTree {
    fn new() -> Self {
        debug!("DEBUG: Initializing SubsurfaceTree");
        
        Self {
            parent_child_map: HashMap::new(),
            child_parent_map: HashMap::new(),
        }
    }
    
    pub fn add_subsurface(&mut self, parent: ObjectId, child: ObjectId) {
        debug!("DEBUG: Adding subsurface relationship: {:?} -> {:?}", parent, child);
        
        self.parent_child_map.entry(parent).or_insert_with(Vec::new).push(child);
        self.child_parent_map.insert(child, parent);
    }
    
    pub fn get_children(&self, parent: ObjectId) -> Option<&Vec<ObjectId>> {
        self.parent_child_map.get(&parent)
    }
    
    pub fn remove_surface(&mut self, surface_id: ObjectId) {
        debug!("DEBUG: Removing surface {:?} from subsurface tree", surface_id);
        
        self.parent_child_map.remove(&surface_id);
        
        if let Some(parent) = self.child_parent_map.remove(&surface_id) {
            if let Some(children) = self.parent_child_map.get_mut(&parent) {
                children.retain(|&child| child != surface_id);
            }
        }
    }
    
    pub fn render_subsurface_tree(&self, root_surface: ObjectId) -> Vec<SubsurfaceRenderElement> {
        debug!("DEBUG: Phase 3 - Rendering subsurface tree for root {:?}", root_surface);
        
        let mut elements = Vec::new();
        self.render_subsurface_recursive(root_surface, 0, &mut elements);
        
        debug!("DEBUG: Generated {} subsurface render elements", elements.len());
        elements
    }
    
    fn render_subsurface_recursive(&self, surface_id: ObjectId, depth: i32, 
                                  elements: &mut Vec<SubsurfaceRenderElement>) {
        debug!("DEBUG: Rendering subsurface {:?} at depth {}", surface_id, depth);
        
        elements.push(SubsurfaceRenderElement {
            surface_id,
            depth,
            parent_id: self.child_parent_map.get(&surface_id).copied(),
        });
        
        if let Some(children) = self.parent_child_map.get(&surface_id) {
            for &child in children {
                self.render_subsurface_recursive(child, depth + 1, elements);
            }
        }
    }
}

#[derive(Debug, Clone)]
pub struct RenderElement {
    pub surface_id: ObjectId,
    pub texture_source: TextureSource,
    pub transform: Transform,
    pub scale: f64,
    pub z_index: i32,
    pub damage_regions: Vec<Rectangle<i32, smithay::utils::Logical>>,
    pub opacity: f32,
    pub blend_mode: BlendMode,
}

#[derive(Debug, Clone)]
pub struct LayerRenderElement {
    pub surface_id: ObjectId,
    pub layer: Layer,
    pub exclusive_zone: ExclusiveZone,
    pub z_index: i32,
}

#[derive(Debug, Clone)]
pub struct SubsurfaceRenderElement {
    pub surface_id: ObjectId,
    pub depth: i32,
    pub parent_id: Option<ObjectId>,
}

pub struct MultiSurfaceRenderer {
    surface_elements: Vec<RenderElement>,
    layer_elements: Vec<LayerRenderElement>,
    subsurface_elements: Vec<SubsurfaceRenderElement>,
}

impl MultiSurfaceRenderer {
    pub fn new() -> Self {
        debug!("DEBUG: Phase 3 - Initializing MultiSurfaceRenderer");
        
        Self {
            surface_elements: Vec::new(),
            layer_elements: Vec::new(),
            subsurface_elements: Vec::new(),
        }
    }
    
    pub fn add_surface_element(&mut self, element: RenderElement) {
        debug!("DEBUG: Adding surface element {:?} with z_index={}", element.surface_id, element.z_index);
        self.surface_elements.push(element);
    }
    
    pub fn add_layer_element(&mut self, element: LayerRenderElement) {
        debug!("DEBUG: Adding layer element {:?} for layer {:?}", element.surface_id, element.layer);
        self.layer_elements.push(element);
    }
    
    pub fn add_subsurface_element(&mut self, element: SubsurfaceRenderElement) {
        debug!("DEBUG: Adding subsurface element {:?} at depth {}", element.surface_id, element.depth);
        self.subsurface_elements.push(element);
    }
    
    pub fn render_all_in_order(&mut self) -> Vec<FinalRenderElement> {
        debug!("DEBUG: Phase 3 - Rendering all elements in final Z-order");
        
        let mut final_elements = Vec::new();
        
        for layer_element in &self.layer_elements {
            final_elements.push(FinalRenderElement {
                surface_id: layer_element.surface_id,
                element_type: RenderElementType::Layer(layer_element.layer),
                z_index: layer_element.z_index,
            });
        }
        
        for surface_element in &self.surface_elements {
            final_elements.push(FinalRenderElement {
                surface_id: surface_element.surface_id,
                element_type: RenderElementType::Surface,
                z_index: surface_element.z_index,
            });
        }
        
        for subsurface_element in &self.subsurface_elements {
            final_elements.push(FinalRenderElement {
                surface_id: subsurface_element.surface_id,
                element_type: RenderElementType::Subsurface(subsurface_element.depth),
                z_index: subsurface_element.depth * 10,
            });
        }
        
        final_elements.sort_by_key(|e| e.z_index);
        
        debug!("DEBUG: Final render order contains {} elements", final_elements.len());
        final_elements
    }
}

#[derive(Debug, Clone)]
pub struct FinalRenderElement {
    pub surface_id: ObjectId,
    pub element_type: RenderElementType,
    pub z_index: i32,
}

#[derive(Debug, Clone)]
pub enum RenderElementType {
    Surface,
    Layer(Layer),
    Subsurface(i32),
}
