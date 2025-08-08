use anyrender_vello::wgpu_context::DeviceHandle;
use anyrender_vello::{CustomPaintCtx, CustomPaintSource, TextureHandle};
use blitz_smithay::{BlitzSmithayRenderer, SurfaceCompositor, Size};
use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::{Arc, Mutex};
use std::time::Instant;
use wgpu;
use wgpu::Instance;
use tracing::debug;

#[derive(Clone)]
struct TextureAndHandle {
    texture: wgpu::Texture,
    handle: TextureHandle,
}

#[cfg(feature = "smithay-backend")]
use smithay::{
    delegate_compositor, delegate_data_device, delegate_seat, delegate_shm, delegate_xdg_shell,
    input::{Seat, SeatHandler, SeatState},
    reexports::wayland_server::{Display, protocol::wl_seat},
    utils::Serial,
    wayland::{
        buffer::BufferHandler,
        compositor::{CompositorClientState, CompositorHandler, CompositorState, with_states, SurfaceAttributes, BufferAssignment},
        selection::{
            data_device::{ClientDndGrabHandler, DataDeviceHandler, DataDeviceState, ServerDndGrabHandler},
            SelectionHandler,
        },
        shell::xdg::{PopupSurface, PositionerState, ToplevelSurface, XdgShellHandler, XdgShellState},
        shm::{ShmHandler, ShmState},
    },
};

#[cfg(feature = "smithay-backend")]
use smithay::utils::{Rectangle as SRect, Point as SPoint, Size as SSize};

#[cfg(feature = "smithay-backend")]
use wayland_server::{
    backend::{ClientData, ClientId, DisconnectReason},
    protocol::{
        wl_buffer,
        wl_surface::WlSurface,
    },
    Client, ListeningSocket, Resource,
};

#[cfg(feature = "smithay-backend")]
use wayland_protocols::xdg::shell::server::xdg_toplevel;

#[cfg(feature = "smithay-backend")]
use std::os::unix::io::OwnedFd;

pub struct SmithayPaintSource {
    state: SmithayRendererState,
    start_time: Instant,
    tx: Sender<SmithayMessage>,
    rx: Receiver<SmithayMessage>,
    blitz_renderer: Option<BlitzSmithayRenderer>,
    wayland_state: Option<WaylandCompositorState>,
    surface_compositor: Option<Arc<Mutex<SurfaceCompositor>>>,
}

pub enum SmithayMessage {
    ClientConnected,
    NewToplevelSurface,
    SurfaceCommitted,
    PopupCreated,
}

enum SmithayRendererState {
    Active(Box<ActiveSmithayRenderer>),
    Suspended,
}

struct ActiveSmithayRenderer {
    device: wgpu::Device,
    queue: wgpu::Queue,
    displayed_texture: Option<TextureAndHandle>,
    next_texture: Option<TextureAndHandle>,
    compositor_texture: Option<wgpu::Texture>,
}

#[cfg(feature = "smithay-backend")]
struct WaylandCompositorState {
    display: Display<SmithayApp>,
    listener: ListeningSocket,
    clients: Vec<Client>,
    app_state: SmithayApp,
    socket_name: String,
    start_time: Instant,
}

#[cfg(not(feature = "smithay-backend"))]
struct WaylandCompositorState {
    socket_name: String,
    client_count: u32,
    surface_count: u32,
    start_time: Instant,
}

#[cfg(feature = "smithay-backend")]
struct SmithayApp {
    compositor_state: CompositorState,
    xdg_shell_state: XdgShellState,
    shm_state: ShmState,
    seat_state: SeatState<Self>,
    data_device_state: DataDeviceState,
    seat: Seat<Self>,
    surface_compositor: Option<Arc<Mutex<SurfaceCompositor>>>,
    sender: Sender<SmithayMessage>,
    surface_ids: HashMap<u32, ObjectId>,
}

#[cfg(feature = "smithay-backend")]
#[derive(Default)]
struct ClientState {
    compositor_state: CompositorClientState,
}

#[cfg(feature = "smithay-backend")]
impl ClientData for ClientState {
    fn initialized(&self, _client_id: ClientId) {
        debug!("DEBUG: Smithay client initialized");
    }

    fn disconnected(&self, _client_id: ClientId, _reason: DisconnectReason) {
        debug!("DEBUG: Smithay client disconnected");
    }
}

#[cfg(feature = "smithay-backend")]
impl SmithayApp {
    fn get_or_create_surface_id(&mut self, surface: &WlSurface) -> ObjectId {
        let key = surface.id().protocol_id();
        if let Some(id) = self.surface_ids.get(&key) {
            *id
        } else {
            let id = ObjectId::new();
            self.surface_ids.insert(key, id);
            id
        }
    }
}

#[cfg(feature = "smithay-backend")]
impl BufferHandler for SmithayApp {
    fn buffer_destroyed(&mut self, _buffer: &wl_buffer::WlBuffer) {}
}


#[cfg(feature = "smithay-backend")]
impl CompositorHandler for SmithayApp {
    fn compositor_state(&mut self) -> &mut CompositorState {
        &mut self.compositor_state
    }

    fn client_compositor_state<'a>(&self, client: &'a Client) -> &'a CompositorClientState {
        &client.get_data::<ClientState>().unwrap().compositor_state
    }

    fn commit(&mut self, surface: &WlSurface) {
        debug!("Surface commit received for surface: {:?}", surface.id());
        
        let surface_id = self.get_or_create_surface_id(surface);
        
        let has_buffer = with_states(surface, |states| {
            states.cached_state.get::<SurfaceAttributes>()
                .current()
                .buffer
                .as_ref()
                .and_then(|buffer| match buffer {
                    BufferAssignment::NewBuffer(buffer) => Some(buffer.clone()),
                    _ => None,
                })
        });
        
        if let Some(buffer) = has_buffer {
            debug!("Surface has buffer attached");
            
            if let Ok(buffer_data) = smithay::wayland::shm::with_buffer_contents(&buffer, |data, len, spec| {
                debug!("Buffer data - format: {:?}, width: {}, height: {}, stride: {}", 
                       spec.format, spec.width, spec.height, spec.stride);
                
                let (wgpu_format, format_str) = match spec.format {
                    smithay::reexports::wayland_server::protocol::wl_shm::Format::Argb8888 => (wgpu::TextureFormat::Bgra8Unorm, "ARGB8888"),
                    smithay::reexports::wayland_server::protocol::wl_shm::Format::Xrgb8888 => (wgpu::TextureFormat::Bgra8Unorm, "XRGB8888"),
                    smithay::reexports::wayland_server::protocol::wl_shm::Format::Bgra8888 => (wgpu::TextureFormat::Bgra8Unorm, "BGRA8888"),
                    smithay::reexports::wayland_server::protocol::wl_shm::Format::Rgba8888 => (wgpu::TextureFormat::Rgba8Unorm, "RGBA8888"),
                    _ => (wgpu::TextureFormat::Rgba8Unorm, "RGBA8888"),
                };
                
            if false { // Temporarily disabled until proper surface_compositor integration
                    let compositor = surface_compositor.lock().unwrap();
                    let device = compositor.wgpu_device();
                    let queue = compositor.wgpu_queue();

                    let width = spec.width as u32;
                    let height = spec.height as u32;
                    let src_stride = spec.stride as usize;
                    
                    let aligned_bpr = ((src_stride as u32 + 255) / 256) * 256;
                    let mut padded_data = vec![0u8; aligned_bpr as usize * height as usize];
                    let src = unsafe { std::slice::from_raw_parts(data, len) };
                    for row in 0..height as usize {
                        let src_off = row * src_stride;
                        let dst_off = row * aligned_bpr as usize;
                        let copy_len = std::cmp::min(src_stride, src.len().saturating_sub(src_off));
                        padded_data[dst_off..dst_off + copy_len].copy_from_slice(&src[src_off..src_off + copy_len]);
                    }
                    
                    let wgpu_texture = device.create_texture(&wgpu::TextureDescriptor {
                        label: Some("Wayland Surface Texture"),
                        size: wgpu::Extent3d {
                            width,
                            height,
                            depth_or_array_layers: 1,
                        },
                        mip_level_count: 1,
                        sample_count: 1,
                        dimension: wgpu::TextureDimension::D2,
                        format: wgpu_format,
                        usage: wgpu::TextureUsages::TEXTURE_BINDING
                            | wgpu::TextureUsages::COPY_DST
                            | wgpu::TextureUsages::RENDER_ATTACHMENT,
                        view_formats: &[],
                    });
                    
                    queue.write_texture(
                        wgpu::TexelCopyTextureInfo {
                            texture: &wgpu_texture,
                            mip_level: 0,
                            origin: wgpu::Origin3d::ZERO,
                            aspect: wgpu::TextureAspect::All,
                        },
                        &padded_data,
                        wgpu::TexelCopyBufferLayout {
                            offset: 0,
                            bytes_per_row: Some(aligned_bpr),
                            rows_per_image: Some(height),
                        },
                        wgpu::Extent3d { width, height, depth_or_array_layers: 1 },
                    );
                    
                    let _dmabuf_info = DmaBufInfo::new(0, width, height, format_str.to_string(), spec.stride as u32);
                    let texture = BlitzTexture::from_wgpu_texture(wgpu_texture);
                    Ok::<(BlitzTexture, u32, u32), Box<dyn std::error::Error>>((texture, width, height))
                } else {
                    Err("No SurfaceCompositor available".into())
                }
            }) {
                if let Ok((texture, width, height)) = buffer_data {
            if false { // Temporarily disabled until proper surface_compositor integration
                        let mut compositor = surface_compositor.lock().unwrap();
                        if let Err(e) = compositor.add_surface(surface_id) {
                            debug!("Failed to add surface to compositor: {:?}", e);
                        } else if let Err(e) = compositor.set_surface_texture(surface_id, texture) {
                            debug!("Failed to set surface texture: {:?}", e);
                        } else {
                            match compositor.get_surface_count() {
                                Ok(count) => debug!("Surface added. Current surface_count: {}", count),
                                Err(e) => debug!("Failed to get surface count after add: {:?}", e),
                            }
                            let full_rect = SRect::from_loc_and_size(
                                SPoint::from((0, 0)),
                                SSize::from((width as i32, height as i32)),
                            );
                            if let Err(e) = compositor.track_surface_damage(surface_id, &[full_rect]) {
                                debug!("Failed to track damage for surface {:?}: {:?}", surface_id, e);
                            }
                            if let Err(e) = compositor.commit_surface(surface_id) {
                                debug!("Failed to commit surface {:?}: {:?}", surface_id, e);
                            } else {
                                debug!("Successfully updated surface {:?} with texture and committed", surface_id);
                            }
                        }
                    }
                }
            } else {
                debug!("Failed to extract buffer data");
            }
        } else {
            if false { // Temporarily disabled until proper surface_compositor integration
                if let Err(e) = surface_compositor.lock().unwrap().add_surface(surface_id) {
                    debug!("Failed to add surface to compositor: {:?}", e);
                } else {
                    debug!("Added surface {:?} to compositor (no buffer)", surface_id);
                }
            }
        }
        
        if let Err(e) = self.sender.send(SmithayMessage::SurfaceCommitted) {
            debug!("Failed to send surface commit message: {:?}", e);
        }
    }
}

#[cfg(feature = "smithay-backend")]
impl ShmHandler for SmithayApp {
    fn shm_state(&self) -> &ShmState {
        &self.shm_state
    }
}

#[cfg(feature = "smithay-backend")]
impl XdgShellHandler for SmithayApp {
    fn xdg_shell_state(&mut self) -> &mut XdgShellState {
        &mut self.xdg_shell_state
    }

    fn new_toplevel(&mut self, surface: ToplevelSurface) {
        debug!("New toplevel surface created: {:?}", surface.wl_surface().id());
        
        surface.with_pending_state(|state| {
            state.states.set(xdg_toplevel::State::Activated);
        });
        surface.send_configure();
        
        let wl_surface = surface.wl_surface();
        let surface_id = self.get_or_create_surface_id(&wl_surface);
        
        if let Some(ref surface_compositor) = self.surface_compositor {
            if let Err(e) = surface_compositor.lock().unwrap().add_surface(surface_id) {
                debug!("Failed to add toplevel surface to compositor: {:?}", e);
            } else {
                debug!("Added toplevel surface {:?} to compositor", surface_id);
            }
        }
        
        if let Err(e) = self.sender.send(SmithayMessage::ClientConnected) {
            debug!("Failed to send client connected message: {:?}", e);
        }
    }

    fn new_popup(&mut self, surface: PopupSurface, _positioner: PositionerState) {
        debug!("New popup surface created: {:?}", surface.wl_surface().id());
        
        if let Err(e) = self.sender.send(SmithayMessage::PopupCreated) {
            debug!("Failed to send popup created message: {:?}", e);
        }
    }

    fn grab(&mut self, _surface: PopupSurface, _seat: wl_seat::WlSeat, _serial: Serial) {
        debug!("DEBUG: Popup grab requested");
    }

    fn reposition_request(&mut self, _surface: PopupSurface, _positioner: PositionerState, _token: u32) {
        debug!("DEBUG: Popup reposition requested");
    }
}

#[cfg(feature = "smithay-backend")]
impl SelectionHandler for SmithayApp {
    type SelectionUserData = ();
}

#[cfg(feature = "smithay-backend")]
impl DataDeviceHandler for SmithayApp {
    fn data_device_state(&self) -> &DataDeviceState {
        &self.data_device_state
    }
}

#[cfg(feature = "smithay-backend")]
impl ClientDndGrabHandler for SmithayApp {}

#[cfg(feature = "smithay-backend")]
impl ServerDndGrabHandler for SmithayApp {
    fn send(&mut self, _mime_type: String, _fd: OwnedFd, _seat: Seat<Self>) {}
}

#[cfg(feature = "smithay-backend")]
impl SeatHandler for SmithayApp {
    type KeyboardFocus = WlSurface;
    type PointerFocus = WlSurface;
    type TouchFocus = WlSurface;

    fn seat_state(&mut self) -> &mut SeatState<Self> {
        &mut self.seat_state
    }

    fn focus_changed(&mut self, _seat: &Seat<Self>, focused: Option<&WlSurface>) {
        if let Some(surface) = focused {
            debug!("Focus changed to surface: {:?}", surface.id());
        } else {
            debug!("Focus cleared");
        }
    }
    
    fn cursor_image(&mut self, _seat: &Seat<Self>, _image: smithay::input::pointer::CursorImageStatus) {
        debug!("Cursor image changed");
    }
}



impl CustomPaintSource for SmithayPaintSource {
    fn resume(&mut self, _instance: &Instance, device_handle: &DeviceHandle) {
        debug!("DEBUG: SmithayPaintSource resuming with WGPU device");
        let active_state = ActiveSmithayRenderer::new(device_handle);
        self.state = SmithayRendererState::Active(Box::new(active_state));
        
        if self.blitz_renderer.is_none() {
            match BlitzSmithayRenderer::new() {
                Ok(mut renderer) => {
                    if let Err(e) = renderer.initialize_compositor(
                        device_handle.device.clone(),
                        device_handle.queue.clone(),
                    ) {
                        debug!("DEBUG: Failed to initialize Smithay compositor: {:?}", e);
                    } else {
                        debug!("DEBUG: BlitzSmithayRenderer initialized with WGPU device/queue");
                    }
                    self.blitz_renderer = Some(renderer);
                }
                Err(e) => {
                    debug!("DEBUG: Failed to create BlitzSmithayRenderer: {:?}", e);
                }
            }
        } else {
            if let Some(renderer) = self.blitz_renderer.as_mut() {
                if renderer.wgpu_device().is_none() || renderer.wgpu_queue().is_none() {
                    if let Err(e) = renderer.initialize_compositor(
                        device_handle.device.clone(),
                        device_handle.queue.clone(),
                    ) {
                        debug!("DEBUG: Failed to initialize Smithay compositor (existing renderer): {:?}", e);
                    } else {
                        debug!("DEBUG: Initialized existing BlitzSmithayRenderer with WGPU device/queue");
                    }
                } else {
                    debug!("DEBUG: BlitzSmithayRenderer already initialized");
                }
            }
        }
        
        debug!("DEBUG: About to create SurfaceCompositor with device and queue");
        let surface_compositor = SurfaceCompositor::new(
            device_handle.device.clone(),
            device_handle.queue.clone(),
            Size::from((800, 600)),
        );
        debug!("DEBUG: SurfaceCompositor::new completed successfully");
        self.surface_compositor = Some(Arc::new(Mutex::new(surface_compositor)));
        debug!("DEBUG: Created SurfaceCompositor for SmithayPaintSource - compositor field initialized");
        
        self.setup_wayland_compositor();
    }

    fn suspend(&mut self) {
        debug!("DEBUG: SmithayPaintSource suspending");
        self.state = SmithayRendererState::Suspended;
    }

    fn render(&mut self, ctx: CustomPaintCtx<'_>, width: u32, height: u32, _scale: f64) -> Option<TextureHandle> {
        self.dispatch_wayland_events();
        self.process_messages();
        self.render_compositor_content(ctx, width, height)
    }
}

impl SmithayPaintSource {
    pub fn new() -> Result<Self, Box<dyn std::error::Error>> {
        debug!("DEBUG: Creating SmithayPaintSource");
        let (tx, rx) = channel();
        
        Ok(Self {
            state: SmithayRendererState::Suspended,
            start_time: Instant::now(),
            tx,
            rx,
            blitz_renderer: None,
            wayland_state: None,
            surface_compositor: None,
        })
    }
    
    pub fn sender(&self) -> Sender<SmithayMessage> {
        self.tx.clone()
    }
    
    fn process_messages(&mut self) {
        while let Ok(msg) = self.rx.try_recv() {
            match msg {
                SmithayMessage::ClientConnected => {
                    debug!("Processing real Wayland client connection");
                }
                SmithayMessage::NewToplevelSurface => {
                    debug!("Processing real toplevel surface creation");
                }
                SmithayMessage::SurfaceCommitted => {
                    debug!("Processing surface commit");
                }
                SmithayMessage::PopupCreated => {
                    debug!("Processing popup creation");
                }
            }
        }
    }
    
    fn render_compositor_content(&mut self, mut ctx: CustomPaintCtx<'_>, width: u32, height: u32) -> Option<TextureHandle> {
        if width == 0 || height == 0 {
            return None;
        }
        
        let SmithayRendererState::Active(state) = &mut self.state else {
            return None;
        };
        
        let needs_new_texture = match &state.next_texture {
            Some(next) => next.texture.width() != width || next.texture.height() != height,
            None => true,
        };
        
        if needs_new_texture {
            if let Some(old) = &state.next_texture {
                ctx.unregister_texture(old.handle);
            }
            
            let texture = create_compositor_texture(&state.device, width, height);
            let handle = ctx.register_texture(texture.clone());
            state.next_texture = Some(TextureAndHandle { texture, handle });
        }
        
        let texture_handle = state.next_texture.as_ref().unwrap().handle;
        let target_texture = state.next_texture.as_ref().unwrap().texture.clone();
        
        let device = state.device.clone();
        let queue = state.queue.clone();
        
        Self::render_to_texture(&device, &queue, &target_texture, &self.wayland_state, &self.surface_compositor);
        
        std::mem::swap(&mut state.next_texture, &mut state.displayed_texture);
        Some(texture_handle)
    }
    
    fn render_to_texture(device: &wgpu::Device, queue: &wgpu::Queue, target_texture: &wgpu::Texture, _wayland_state: &Option<WaylandCompositorState>, surface_compositor: &Option<Arc<Mutex<SurfaceCompositor>>>) {
        debug!("Rendering real Smithay compositor content to WGPU texture");
        
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Smithay Compositor Render"),
        });
        
        {
            let view = target_texture.create_view(&wgpu::TextureViewDescriptor::default());
            let rpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Smithay Compositor Background Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color { r: 0.05, g: 0.05, b: 0.1, a: 1.0 }),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });
            drop(rpass);
        }
        
        if let Some(ref compositor) = surface_compositor {
            let mut guard = compositor.lock().unwrap();
            match guard.get_surface_count() {
                Ok(count) => debug!("SurfaceCompositor surface_count before render: {}", count),
                Err(e) => debug!("Failed to get surface count: {:?}", e),
            }
            if let Err(e) = guard.render_surfaces_to_wgpu_texture(target_texture) {
                debug!("Error rendering surfaces to WGPU texture: {:?}", e);
            } else {
                debug!("Successfully rendered surfaces to WGPU texture");
            }
        } else {
            debug!("No surface compositor available for rendering");
        }
        
        queue.submit(Some(encoder.finish()));
    }
    
    #[cfg(feature = "smithay-backend")]
    fn setup_wayland_compositor(&mut self) {
        debug!("DEBUG: Setting up real Smithay Wayland compositor server");
        
        let socket_name = format!("wayland-blitz-{}", std::process::id());
        
        let display: Display<SmithayApp> = match Display::new() {
            Ok(display) => display,
            Err(e) => {
                debug!("DEBUG: Failed to create Wayland display: {}", e);
                return;
            }
        };
        
        let dh = display.handle();
        
        let compositor_state = CompositorState::new::<SmithayApp>(&dh);
        let shm_state = ShmState::new::<SmithayApp>(&dh, vec![]);
        let mut seat_state = SeatState::new();
        let seat = seat_state.new_wl_seat(&dh, "blitz-compositor");
        
        let surface_compositor = if let Some(ref renderer) = self.blitz_renderer {
            if let Some(device) = renderer.wgpu_device() {
                let queue = renderer.wgpu_queue().expect("WGPU queue should be available");
                let output_size = renderer.output_size().unwrap_or((800, 600));
                
                let mut surface_compositor = SurfaceCompositor::new(
                    device.clone(),
                    queue.clone(),
                    smithay::utils::Size::from((output_size.0 as i32, output_size.1 as i32)),
                );
                
                if let Some(gles_renderer) = renderer.gles_renderer() {
                    surface_compositor.set_gles_renderer(gles_renderer);
                    debug!("Set GlesRenderer for SurfaceCompositor");
                }
                
                Some(Arc::new(Mutex::new(surface_compositor)))
            } else {
                debug!("Failed to get WGPU device from BlitzSmithayRenderer");
                None
            }
        } else {
            debug!("BlitzSmithayRenderer not available");
            None
        };
        
        let app_state = SmithayApp {
            compositor_state,
            xdg_shell_state: XdgShellState::new::<SmithayApp>(&dh),
            shm_state,
            seat_state,
            data_device_state: DataDeviceState::new::<SmithayApp>(&dh),
            seat,
            surface_compositor,
            sender: self.tx.clone(),
            surface_ids: HashMap::new(),
        };
        
        let listener = match ListeningSocket::bind(&socket_name) {
            Ok(listener) => listener,
            Err(e) => {
                debug!("DEBUG: Failed to bind Wayland socket {}: {}", socket_name, e);
                return;
            }
        };
        
        self.wayland_state = Some(WaylandCompositorState {
            display,
            listener,
            clients: Vec::new(),
            app_state,
            socket_name: socket_name.clone(),
            start_time: Instant::now(),
        });
        
        unsafe {
            std::env::set_var("WAYLAND_DISPLAY", &socket_name);
        }
        debug!("DEBUG: Real Smithay Wayland server listening on socket: {}", socket_name);
        debug!("DEBUG: Set WAYLAND_DISPLAY environment variable");
        
        self.spawn_test_client();
    }

    #[cfg(not(feature = "smithay-backend"))]
    fn setup_wayland_compositor(&mut self) {
        debug!("DEBUG: Smithay backend not available - install libseat-dev and other system dependencies");
        debug!("DEBUG: Run: sudo apt-get install libseat-dev libinput-dev libudev-dev");
        debug!("DEBUG: Then rebuild with: cargo build --package smithay_compositor");
        
        let socket_name = format!("wayland-blitz-{}", std::process::id());
        
        self.wayland_state = Some(WaylandCompositorState {
            socket_name: socket_name.clone(),
            client_count: 0,
            surface_count: 0,
            start_time: Instant::now(),
        });
        
        debug!("DEBUG: Wayland compositor setup completed (no real surfaces without smithay-backend)");
    }
    
    #[cfg(feature = "smithay-backend")]
    fn dispatch_wayland_events(&mut self) {
        if let Some(ref mut wayland_state) = self.wayland_state {
            if let Ok(Some(stream)) = wayland_state.listener.accept() {
                debug!("DEBUG: New Wayland client connected: {:?}", stream);
                
                match wayland_state.display
                    .handle()
                    .insert_client(stream, Arc::new(ClientState::default()))
                {
                    Ok(client) => {
                        wayland_state.clients.push(client);
                        debug!("DEBUG: Client successfully inserted, total clients: {}", wayland_state.clients.len());
                        let _ = self.tx.send(SmithayMessage::ClientConnected);
                    }
                    Err(e) => {
                        debug!("DEBUG: Failed to insert client: {}", e);
                    }
                }
            }
            
            if let Err(e) = wayland_state.display.dispatch_clients(&mut wayland_state.app_state) {
                debug!("DEBUG: Error dispatching clients: {}", e);
            } else if !wayland_state.clients.is_empty() {
                debug!("DEBUG: Successfully dispatched events for {} clients", wayland_state.clients.len());
            }
            
            if let Err(e) = wayland_state.display.flush_clients() {
                debug!("DEBUG: Error flushing clients: {}", e);
            }
            
            let _time = wayland_state.start_time.elapsed().as_millis() as u32;
        }
    }

    #[cfg(not(feature = "smithay-backend"))]
    fn dispatch_wayland_events(&mut self) {
        debug!("DEBUG: Wayland backend not available - real surface creation requires smithay-backend feature");
        
        let elapsed = self.start_time.elapsed().as_secs();
        
        if let Some(ref mut wayland_state) = self.wayland_state {
            if elapsed >= 2 && wayland_state.client_count == 0 {
                debug!("DEBUG: Simulating client connection (no surface creation without smithay-backend)");
                wayland_state.client_count = 1;
                let _ = self.tx.send(SmithayMessage::ClientConnected);
            }
        }
    }
    
    
    
}

impl ActiveSmithayRenderer {
    pub fn new(device_handle: &DeviceHandle) -> Self {
        debug!("DEBUG: Creating ActiveSmithayRenderer");
        
        Self {
            device: device_handle.device.clone(),
            queue: device_handle.queue.clone(),
            displayed_texture: None,
            next_texture: None,
            compositor_texture: None,
        }
    }
}


fn create_compositor_texture(device: &wgpu::Device, width: u32, height: u32) -> wgpu::Texture {
    debug!("DEBUG: Creating compositor texture {}x{}", width, height);
    device.create_texture(&wgpu::TextureDescriptor {
        label: Some("Smithay Compositor Texture"),
        size: wgpu::Extent3d { 
            width, 
            height, 
            depth_or_array_layers: 1 
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba8Unorm,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_SRC,
        view_formats: &[],
    })
}

#[cfg(feature = "smithay-backend")]
delegate_xdg_shell!(SmithayApp);
#[cfg(feature = "smithay-backend")]
delegate_compositor!(SmithayApp);
#[cfg(feature = "smithay-backend")]
delegate_shm!(SmithayApp);
#[cfg(feature = "smithay-backend")]
delegate_seat!(SmithayApp);
#[cfg(feature = "smithay-backend")]
delegate_data_device!(SmithayApp);
