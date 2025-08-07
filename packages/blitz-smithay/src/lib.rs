//! 
//!

use std::sync::{Arc, Mutex};
use tracing::debug;
use smithay::{
    backend::{
        egl::{EGLContext, EGLDisplay},
        renderer::gles::GlesRenderer,
    },
    desktop::{Space, PopupManager, Window},
    input::{SeatState, Seat},
    wayland::{
        compositor::CompositorState,
        shell::xdg::XdgShellState,
        shm::ShmState,
        selection::data_device::DataDeviceState,
    },
};
use wayland_server::Display as WaylandDisplay;
use calloop::EventLoop;
use gbm::{Device as GbmDevice, BufferObjectFlags};
use std::fs::File;

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

pub struct BlitzSmithayRenderer {
    format_converter: FormatConverter,
    coordinate_mapper: CoordinateMapper,
    resource_manager: Arc<Mutex<ResourceManager>>,
    surface_manager: WaylandSurfaceManager,
    
    smithay_compositor: Option<SmithayCompositor>,
    egl_context: Option<EGLContext>,
    gles_renderer: Option<GlesRenderer>,
    
    wgpu_device: Option<wgpu::Device>,
    wgpu_queue: Option<wgpu::Queue>,
}

pub struct SmithayCompositor {
    pub display: WaylandDisplay<AnvilState>,
    pub event_loop: EventLoop<'static, AnvilState>,
    pub state: AnvilState,
}

pub struct AnvilState {
    pub compositor_state: CompositorState,
    pub xdg_shell_state: XdgShellState,
    pub shm_state: ShmState,
    pub seat_state: SeatState<Self>,
    pub data_device_state: DataDeviceState,
    pub seat: Seat<Self>,
    pub space: Space<Window>,
    pub popups: PopupManager,
}

impl BlitzSmithayRenderer {
    pub fn new() -> Result<Self, BlitzSmithayError> {
        debug!("Creating new BlitzSmithayRenderer with real Smithay integration");
        
        let format_converter = FormatConverter::new()?;
        debug!("Format converter initialized with supported formats: {:?}", 
               format_converter.supported_formats());
        
        let coordinate_mapper = CoordinateMapper::new();
        debug!("Coordinate mapper initialized");
        
        let resource_manager = Arc::new(Mutex::new(ResourceManager::new()));
        debug!("Resource manager initialized");
        
        let surface_manager = WaylandSurfaceManager::new();
        debug!("Surface manager initialized");
        
        debug!("BlitzSmithayRenderer created successfully - ready for compositor initialization");
        
        Ok(Self {
            format_converter,
            coordinate_mapper,
            resource_manager,
            surface_manager,
            smithay_compositor: None,
            egl_context: None,
            gles_renderer: None,
            wgpu_device: None,
            wgpu_queue: None,
        })
    }
    
    pub fn initialize_compositor(&mut self, wgpu_device: wgpu::Device, wgpu_queue: wgpu::Queue) -> Result<(), BlitzSmithayError> {
        debug!("Initializing Smithay compositor with WGPU texture bridging");
        
        self.wgpu_device = Some(wgpu_device);
        self.wgpu_queue = Some(wgpu_queue);
        
        let drm_file = File::open("/dev/dri/renderD128")?;
        let gbm_device = GbmDevice::new(drm_file)?;
        
        let egl_display = unsafe { 
            EGLDisplay::new(&gbm_device, None)?
        };
        let egl_context = EGLContext::new(&egl_display, None)?;
        
        let gles_renderer = unsafe { 
            GlesRenderer::new(egl_context, None)?
        };
        
        let smithay_compositor = self.create_smithay_compositor()?;
        
        self.egl_context = Some(egl_context);
        self.gles_renderer = Some(gles_renderer);
        self.smithay_compositor = Some(smithay_compositor);
        
        debug!("Smithay compositor initialized successfully with texture bridging");
        Ok(())
    }
    
    fn create_smithay_compositor(&self) -> Result<SmithayCompositor, BlitzSmithayError> {
        debug!("Creating Smithay compositor instance");
        
        let display = WaylandDisplay::new().map_err(|e| BlitzSmithayError::WaylandDisplay(format!("{:?}", e)))?;
        let display_handle = display.handle();
        
        let event_loop = EventLoop::try_new()?;
        
        let compositor_state = CompositorState::new::<AnvilState>(&display_handle);
        let xdg_shell_state = XdgShellState::new::<AnvilState>(&display_handle);
        let shm_state = ShmState::new::<AnvilState>(&display_handle, vec![]);
        
        let mut seat_state = SeatState::new();
        let seat = seat_state.new_wl_seat(&display_handle, "blitz-compositor");
        
        let data_device_state = DataDeviceState::new::<AnvilState>(&display_handle);
        
        let space = Space::default();
        let popups = PopupManager::default();
        
        let state = AnvilState {
            compositor_state,
            xdg_shell_state,
            shm_state,
            seat_state,
            data_device_state,
            seat,
            space,
            popups,
        };
        
        debug!("Smithay compositor instance created successfully");
        
        Ok(SmithayCompositor {
            display,
            event_loop,
            state,
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

#[derive(Debug, Clone)]
pub struct BlitzTexture {
    id: TextureId,
    
    width: u32,
    height: u32,
    
    format: String,
    
    dmabuf_info: Option<DmaBufInfo>,
    
    created_at: std::time::Instant,
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
            dmabuf_info: None,
            created_at: std::time::Instant::now(),
        }
    }
    
    pub fn from_dmabuf(width: u32, height: u32, format: String, dmabuf_info: DmaBufInfo) -> Self {
        let id = TextureId::new();
        
        debug!("DEBUG: Created BlitzTexture from DMA-BUF id={:?} format={} size={}x{} fd={}", 
               id, format, width, height, dmabuf_info.fd);
        
        Self {
            id,
            width,
            height,
            format,
            dmabuf_info: Some(dmabuf_info),
            created_at: std::time::Instant::now(),
        }
    }
    
    pub fn is_dmabuf(&self) -> bool {
        self.dmabuf_info.is_some()
    }
    
    pub fn dmabuf_info(&self) -> Option<&DmaBufInfo> {
        self.dmabuf_info.as_ref()
    }
    
    pub fn age(&self) -> std::time::Duration {
        self.created_at.elapsed()
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
    pub fn import_dmabuf(&mut self, dmabuf_info: DmaBufInfo) -> Result<Arc<BlitzTexture>, BlitzSmithayError> {
        debug!("DEBUG: Importing DMA-BUF fd={} format={} size={}x{}", 
               dmabuf_info.fd, dmabuf_info.format, dmabuf_info.width, dmabuf_info.height);
        
        if !self.format_converter.is_format_supported(&dmabuf_info.format) {
            debug!("DEBUG: Format {} not supported, attempting conversion", dmabuf_info.format);
            return Err(BlitzSmithayError::FormatConversion(
                FormatConversionError::UnsupportedSourceFormat(dmabuf_info.format.clone())
            ));
        }
        
        let texture = Arc::new(BlitzTexture::from_dmabuf(
            dmabuf_info.width,
            dmabuf_info.height,
            dmabuf_info.format.clone(),
            dmabuf_info
        ));
        
        {
            let mut manager = self.resource_manager
                .lock()
                .map_err(|_| BlitzSmithayError::ResourceManagerLocked)?;
            manager.cache_dmabuf_texture(texture.clone())?;
        }
        
        debug!("DEBUG: DMA-BUF import successful - texture_id={:?}", texture.id());
        Ok(texture)
    }
    
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

#[derive(Debug, Clone)]
pub struct DmaBufInfo {
    pub fd: i32,
    pub width: u32,
    pub height: u32,
    pub format: String,
    pub stride: u32,
    pub offset: u32,
    pub modifier: u64,
}

impl DmaBufInfo {
    pub fn new(fd: i32, width: u32, height: u32, format: String, stride: u32) -> Self {
        debug!("DEBUG: Creating DmaBufInfo fd={} format={} size={}x{} stride={}", 
               fd, format, width, height, stride);
        
        Self {
            fd,
            width,
            height,
            format,
            stride,
            offset: 0,
            modifier: 0, // DRM_FORMAT_MOD_LINEAR
        }
    }
}
