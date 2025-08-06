//! 
//!

use std::sync::{Arc, Mutex};
use tracing::debug;

pub mod error;
pub mod format_converter;
pub mod coordinate_mapper;
pub mod resource_manager;
pub mod surface_manager;

pub use error::*;
pub use format_converter::*;
pub use coordinate_mapper::*;
pub use resource_manager::*;
pub use surface_manager::*;

/// 
pub struct BlitzSmithayRenderer {
    format_converter: FormatConverter,
    
    coordinate_mapper: CoordinateMapper,
    
    resource_manager: Arc<Mutex<ResourceManager>>,
    
    surface_manager: WaylandSurfaceManager,
}

impl BlitzSmithayRenderer {
    pub fn new() -> Result<Self, BlitzSmithayError> {
        debug!("DEBUG: Creating new BlitzSmithayRenderer skeleton");
        
        let format_converter = FormatConverter::new()?;
        debug!("DEBUG: Format converter initialized with supported formats: {:?}", 
               format_converter.supported_formats());
        
        let coordinate_mapper = CoordinateMapper::new();
        debug!("DEBUG: Coordinate mapper initialized");
        
        let resource_manager = Arc::new(Mutex::new(ResourceManager::new()));
        debug!("DEBUG: Resource manager initialized");
        
        let surface_manager = WaylandSurfaceManager::new();
        debug!("DEBUG: Surface manager initialized");
        
        debug!("DEBUG: BlitzSmithayRenderer skeleton created successfully");
        
        Ok(Self {
            format_converter,
            coordinate_mapper,
            resource_manager,
            surface_manager,
        })
    }
    
    pub fn create_texture(&mut self, width: u32, height: u32, format: String) -> Result<Arc<BlitzTexture>, BlitzSmithayError> {
        debug!("DEBUG: Creating texture - format={} size={}x{}", format, width, height);
        
        let texture = {
            let mut manager = self.resource_manager
                .lock()
                .map_err(|_| BlitzSmithayError::ResourceManagerLocked)?;
            manager.create_texture(width, height, format)?
        };
        
        debug!("DEBUG: Texture creation successful - texture_id={:?}", texture.id());
        Ok(texture)
    }
}

/// 
#[derive(Debug, Clone)]
pub struct BlitzTexture {
    id: TextureId,
    
    width: u32,
    height: u32,
    
    format: String,
}

impl BlitzTexture {
    pub fn new(width: u32, height: u32, format: String) -> Self {
        let id = TextureId::new();
        
        debug!("DEBUG: Created BlitzTexture id={:?} format={} size={}x{}", id, format, width, height);
        
        Self {
            id,
            width,
            height,
            format,
        }
    }
    
    pub fn id(&self) -> TextureId {
        self.id
    }
    
    pub fn dimensions(&self) -> (u32, u32) {
        (self.width, self.height)
    }
    
    pub fn format(&self) -> &str {
        &self.format
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TextureId(u64);

impl TextureId {
    fn new() -> Self {
        use std::sync::atomic::{AtomicU64, Ordering};
        static COUNTER: AtomicU64 = AtomicU64::new(1);
        Self(COUNTER.fetch_add(1, Ordering::Relaxed))
    }
}

/// 
pub struct BlitzFramebuffer {
    target_texture: BlitzTexture,
}

impl BlitzFramebuffer {
    pub fn new(target_texture: BlitzTexture) -> Self {
        let (width, height) = target_texture.dimensions();
        debug!("DEBUG: Created BlitzFramebuffer size={}x{}", width, height);
        
        Self {
            target_texture,
        }
    }
    
    pub fn dimensions(&self) -> (u32, u32) {
        self.target_texture.dimensions()
    }
}

/// 
pub struct BlitzFrame {
    current_surface_id: Option<ObjectId>,
}

impl BlitzFrame {
    pub fn new() -> Self {
        debug!("DEBUG: Created BlitzFrame");
        
        Self {
            current_surface_id: None,
        }
    }
    
    pub fn set_current_surface(&mut self, surface_id: ObjectId) {
        debug!("DEBUG: Setting current surface to {:?}", surface_id);
        self.current_surface_id = Some(surface_id);
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ObjectId(u64);

impl ObjectId {
    pub fn new() -> Self {
        use std::sync::atomic::{AtomicU64, Ordering};
        static COUNTER: AtomicU64 = AtomicU64::new(1);
        Self(COUNTER.fetch_add(1, Ordering::Relaxed))
    }
}

impl BlitzSmithayRenderer {
    pub fn render_frame(&mut self, _framebuffer: BlitzFramebuffer) -> Result<BlitzFrame, BlitzSmithayError> {
        debug!("DEBUG: Starting frame render");
        
        if let Ok(mut manager) = self.resource_manager.try_lock() {
            manager.maybe_cleanup();
        }
        
        let frame = BlitzFrame::new();
        
        debug!("DEBUG: Frame render setup complete");
        Ok(frame)
    }
}
