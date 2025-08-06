# Blitz-Smithay Entegrasyon Kapsamlı Teknik Analiz Raporu

## 📋 Genel Bakış

Bu rapor, Smithay Wayland compositor kütüphanesinin Blitz web rendering engine ile entegrasyonunu detaylı olarak analiz eder. Entegrasyon, Wayland client surface'lerinin (wl_surface) Blitz'in AnyRender sistemi tarafından DMA-BUF kullanarak zero-copy framebuffer paylaşımı ile render edilmesini sağlayacaktır.

### Hedef Mimari
```
Wayland Client → Smithay Compositor → DMA-BUF → Blitz AnyRender → GPU/CPU
```

## 🏗️ Mevcut Mimari Analizi

### Blitz Rendering Mimarisi

#### Windowing Katmanı (blitz-shell)
- **Mevcut sistem**: Winit tabanlı event loop ve window management
- **Ana bileşenler**:
  - `BlitzApplication`: Event loop yönetimi ve uygulama yaşam döngüsü
  - `View`: Window ve renderer koordinasyonu  
  - `BlitzShellProvider`: Platform entegrasyonu (clipboard, file dialog)
  - Event dönüştürme: Winit → Blitz UI events

#### Rendering Katmanı (AnyRender)
- **Soyutlama**: `PaintScene` trait ile 2D çizim komutları
- **Core Traits**:
```rust
pub trait PaintScene {
    fn fill(&mut self, path: &impl Shape, brush: &impl Brush);
    fn stroke(&mut self, path: &impl Shape, brush: &impl Brush, style: &Stroke);
    fn draw_glyphs(&mut self, glyphs: &[Glyph], brush: &impl Brush);
    fn draw_image(&mut self, image: &Image, transform: Affine);
}

pub trait WindowRenderer {
    fn render(&mut self, scene: &impl PaintScene) -> Result<(), Self::Error>;
}
```

- **Mevcut backend'ler**:
  - `anyrender_vello`: GPU (WGPU/Vulkan) - Ana production backend
  - `anyrender_vello_cpu`: CPU (software rendering) - Fallback
  - `anyrender_svg`: SVG desteği

#### WGPU Entegrasyonu
- **Format kısıtlaması**: Sadece `TextureFormat::Rgba8Unorm` ve `Bgra8Unorm` destekleniyor
- **Intermediate texture**: Compute shader'lar direkt surface'e render edemez
- **Custom paint sources**: ID tabanlı WGPU texture override sistemi

### Smithay Compositor Mimarisi

#### Renderer Trait Hiyerarşisi
```rust
pub trait Renderer: RendererSuper {
    type Error: Error;
    type TextureId: Texture;
    type Framebuffer<'buffer>: Texture;
    type Frame<'frame, 'buffer>: Frame<Error = Self::Error, TextureId = Self::TextureId>;
    
    fn render(&mut self, framebuffer: &mut Self::Framebuffer<'_>, 
              output_size: Size<i32, Physical>, dst_transform: Transform) 
              -> Result<Self::Frame<'_, '_>, Self::Error>;
}
```

#### DMA-BUF Desteği
- **Kapsamlı format desteği**: `DmabufFeedbackBuilder` ile format tabloları ve preference tranches
- **Import mekanizmaları**: 
  - `ImportDma` trait: Raw DMA-BUF import işlemleri
  - `ImportDmaWl` trait: Wayland DMA-BUF buffer import
  - `ImportEgl` trait: EGL buffer import
- **Format negotiation**: Client-compositor arası optimal format seçimi
- **Feedback sistemi**: `DmabufFeedback` ile client'lara format önerileri

#### Koordinat Sistemi
- **Transform enum**: `Normal`, `_90`, `_180`, `_270`, `Flipped`, `Flipped90`, `Flipped180`, `Flipped270`
- **Coordinate spaces**: `Logical`, `Physical`, `Buffer`, `Raw` marker types
- **Transform operations**: `transform_size()`, coordinate mapping functions

#### Anvil Compositor Örneği
Smithay'in referans compositor'ü olan Anvil şu özellikleri gösterir:
- **Multi-backend desteği**: DRM/KMS, X11, Winit
- **Renderer abstraction**: GLES, Pixman, Multi-GPU
- **Element system**: Surface, solid color, memory buffer elementleri
- **Damage tracking**: Optimize edilmiş render döngüsü

## ⚡ Teknik Zorluklar Detaylı Analizi

### 1. DMA-BUF Format Compatibility

#### Problem Detayları
**WGPU Format Kısıtlamaları:**
```rust
// Blitz'de sadece bu formatlar destekleniyor
let format = capabilities
    .formats
    .into_iter()
    .find(|it| matches!(it, TextureFormat::Rgba8Unorm | TextureFormat::Bgra8Unorm))
    .ok_or(WgpuContextError::UnsupportedSurfaceFormat)?;
```

**Smithay Format Çeşitliliği:**
- **DRM formatları**: `Fourcc::Argb8888`, `Fourcc::Xrgb8888`, `Fourcc::Abgr8888`, vb.
- **Modifier desteği**: DRM format modifier'ları (tiling, compression)
- **Format feedback**: Client'lara optimal format önerileri
- **Linux DMA-BUF**: Kernel seviyesinde buffer sharing, synchronization, CPU access

**Uyumluluk Sorunları:**
- WGPU'nun sınırlı texture format desteği vs Smithay'in geniş format yelpazesi
- DMA-BUF modifier'ları (tiling, compression) WGPU tarafından desteklenmiyor
- Format conversion overhead'i performance'ı etkileyebilir

#### Çözüm Stratejisi
**Format Conversion Layer:**
```rust
pub struct FormatConverter {
    supported_formats: FormatSet,
    conversion_cache: HashMap<Format, ConversionPipeline>,
    gpu_converter: Option<GpuFormatConverter>,
}

impl FormatConverter {
    fn convert_dmabuf_to_wgpu(&mut self, dmabuf: &Dmabuf) -> Result<WgpuTexture, ConversionError> {
        match dmabuf.format().code {
            Fourcc::Argb8888 => self.convert_argb_to_rgba(dmabuf),
            Fourcc::Xrgb8888 => self.convert_xrgb_to_rgba(dmabuf),
            Fourcc::Abgr8888 => self.convert_abgr_to_rgba(dmabuf),
            _ => {
                // GPU-based conversion fallback
                self.gpu_converter.as_mut()
                    .ok_or(ConversionError::UnsupportedFormat)?
                    .convert_on_gpu(dmabuf)
            }
        }
    }
    
    fn create_conversion_pipeline(&mut self, src_format: Fourcc, dst_format: TextureFormat) 
                                 -> Result<ConversionPipeline, ConversionError> {
        // Compute shader tabanlı format conversion
        let shader_source = self.generate_conversion_shader(src_format, dst_format)?;
        let pipeline = self.device.create_compute_pipeline(&ComputePipelineDescriptor {
            label: Some("Format Converter"),
            layout: None,
            module: &self.device.create_shader_module(ShaderModuleDescriptor {
                label: None,
                source: ShaderSource::Wgsl(shader_source.into()),
            }),
            entry_point: "main",
        });
        Ok(ConversionPipeline { pipeline, bind_group_layout: todo!() })
    }
}
```

**Capability Negotiation:**
```rust
impl BlitzSmithayRenderer {
    fn dmabuf_formats(&self) -> FormatSet {
        let mut formats = FormatSet::new();
        
        // Direkt desteklenen formatlar
        formats.insert(Format { code: Fourcc::Argb8888, modifier: Modifier::Linear });
        formats.insert(Format { code: Fourcc::Xrgb8888, modifier: Modifier::Linear });
        
        // GPU conversion ile desteklenen formatlar
        if self.format_converter.has_gpu_converter() {
            formats.insert(Format { code: Fourcc::Nv12, modifier: Modifier::Linear });
            formats.insert(Format { code: Fourcc::Yuyv, modifier: Modifier::Linear });
        }
        
        formats
    }
}
```

### 2. Coordinate System Differences

#### Problem Detayları
**Wayland Koordinat Sistemi:**
- **Surface-local coordinates**: Her surface kendi koordinat sistemine sahip
- **Buffer transforms**: 90°, 180°, 270° rotasyonlar ve flip işlemleri
- **Scale factors**: HiDPI desteği için fractional scaling
- **Top-left origin**: (0,0) sol üst köşede

**Blitz Koordinat Sistemi:**
- **Affine transforms**: `kurbo::Affine` ile 2D transformasyonlar
- **Viewport scrolling**: Negatif koordinatlarla scroll offset
- **CSS coordinate space**: Web standardı koordinat sistemi
- **Bottom-left origin**: CSS standardına uygun koordinat sistemi

**Transform Karmaşıklığı:**
- Wayland'da 8 farklı transform türü (Normal, 90°, 180°, 270°, Flipped variants)
- Buffer coordinate → Surface coordinate → Output coordinate dönüşümleri
- Multi-output setup'larda koordinat mapping

#### Çözüm Stratejisi
**Coordinate Mapping Layer:**
```rust
pub struct CoordinateMapper {
    wayland_to_blitz: Affine,
    blitz_to_wayland: Affine,
    surface_transforms: HashMap<ObjectId, Transform>,
    output_transforms: HashMap<OutputId, Transform>,
    scale_factors: HashMap<OutputId, f64>,
}

impl CoordinateMapper {
    fn map_wayland_to_blitz(&self, point: Point<f64, Logical>, 
                           surface_id: ObjectId) -> Point<f64, Logical> {
        // Surface transform uygula
        let transformed_point = self.apply_surface_transform(surface_id, point);
        
        // Wayland (top-left) → Blitz (bottom-left) dönüşümü
        let mapped = self.wayland_to_blitz * transformed_point;
        Point::new(mapped.x, self.viewport_height - mapped.y)
    }
    
    fn apply_surface_transform(&self, surface_id: ObjectId, 
                              rect: Rectangle<f64, Logical>) -> Rectangle<f64, Logical> {
        match self.surface_transforms.get(&surface_id) {
            Some(Transform::_90) => self.rotate_90(rect),
            Some(Transform::_180) => self.rotate_180(rect),
            Some(Transform::_270) => self.rotate_270(rect),
            Some(Transform::Flipped) => self.flip_horizontal(rect),
            Some(Transform::Flipped90) => self.flip_horizontal(self.rotate_90(rect)),
            Some(Transform::Flipped180) => self.flip_horizontal(self.rotate_180(rect)),
            Some(Transform::Flipped270) => self.flip_horizontal(self.rotate_270(rect)),
            _ => rect,
        }
    }
    
    fn create_transform_matrix(&self, transform: Transform, size: Size<i32, Logical>) 
                              -> Affine {
        match transform {
            Transform::Normal => Affine::IDENTITY,
            Transform::_90 => Affine::rotate(std::f64::consts::PI / 2.0)
                .then_translate((size.h as f64, 0.0).into()),
            Transform::_180 => Affine::rotate(std::f64::consts::PI)
                .then_translate((size.w as f64, size.h as f64).into()),
            Transform::_270 => Affine::rotate(3.0 * std::f64::consts::PI / 2.0)
                .then_translate((0.0, size.w as f64).into()),
            Transform::Flipped => Affine::scale_non_uniform(-1.0, 1.0)
                .then_translate((size.w as f64, 0.0).into()),
            // ... diğer transform'lar
        }
    }
}
```

**Transform Integration:**
```rust
impl Frame for BlitzFrame<'_, '_> {
    fn render_texture_from_to(&mut self, texture: &Self::TextureId,
                             src: Rectangle<f64, BufferCoord>,
                             dst: Rectangle<i32, Physical>,
                             src_transform: Transform,
                             alpha: f32) -> Result<(), Self::Error> {
        // Smithay transform'unu Blitz Affine'a dönüştür
        let transform_matrix = self.coordinate_mapper
            .create_transform_matrix(src_transform, src.size);
        
        // Koordinat mapping
        let mapped_dst = self.coordinate_mapper
            .map_wayland_to_blitz(dst.loc.to_f64(), self.current_surface_id);
        
        // Alpha blending ile render
        let image = texture.to_anyrender_image();
        self.paint_scene.draw_image_with_alpha(
            &image,
            transform_matrix,
            mapped_dst,
            alpha
        );
        Ok(())
    }
}
```

### 3. Multi-Surface Rendering

#### Problem Detayları
**Smithay Surface Management:**
- **Space abstraction**: Z-order ile surface yönetimi
- **LayerMap**: wlr-layer-shell protokolü desteği (background, bottom, top, overlay)
- **Element system**: `RenderElement` trait ile çoklu surface rendering
- **Surface hierarchy**: Parent-child surface relationships

**Blitz Rendering Pipeline:**
- **Single scene model**: Tek `PaintScene` ile tüm çizim işlemleri
- **Custom paint sources**: ID tabanlı texture override sistemi
- **Sequential rendering**: Element'ler sırayla çiziliyor
- **No native layering**: Z-order management yok

**Rendering Challenges:**
- Birden fazla Wayland surface'in aynı sahne içinde render edilmesi
- Layer shell protokolü desteği (4 farklı layer)
- Surface damage tracking ve partial updates
- Subsurface hierarchy management

#### Çözüm Stratejisi
**Multi-Surface Scene Manager:**
```rust
pub struct MultiSurfaceScene {
    base_scene: Box<dyn PaintScene>,
    surface_layers: BTreeMap<i32, Vec<WaylandSurface>>, // Z-order mapping
    layer_surfaces: LayerManager,
    damage_tracker: SurfaceDamageTracker,
    subsurface_tree: SubsurfaceTree,
}

impl MultiSurfaceScene {
    fn render_complete_frame(&mut self, space: &Space<WindowElement>) {
        // 1. Layer shell background layer
        self.layer_surfaces.render_background_layer(&mut self.base_scene);
        
        // 2. Layer shell bottom layer  
        self.layer_surfaces.render_bottom_layer(&mut self.base_scene);
        
        // 3. Normal windows (Z-order'a göre)
        self.render_window_surfaces(space);
        
        // 4. Layer shell top layer
        self.layer_surfaces.render_top_layer(&mut self.base_scene);
        
        // 5. Layer shell overlay layer
        self.layer_surfaces.render_overlay_layer(&mut self.base_scene);
    }
    
    fn render_window_surfaces(&mut self, space: &Space<WindowElement>) {
        // Z-order'a göre surface'leri render et
        for (z_index, surfaces) in &self.surface_layers {
            for surface in surfaces {
                self.render_wayland_surface_with_subsurfaces(surface);
            }
        }
    }
    
    fn render_wayland_surface_with_subsurfaces(&mut self, surface: &WaylandSurface) {
        // Ana surface'i render et
        self.render_single_surface(surface);
        
        // Subsurface'leri render et
        if let Some(subsurfaces) = self.subsurface_tree.get_children(surface.id()) {
            for subsurface in subsurfaces {
                let relative_transform = self.calculate_subsurface_transform(
                    surface, subsurface
                );
                self.render_single_surface_with_transform(subsurface, relative_transform);
            }
        }
    }
    
    fn render_single_surface(&mut self, surface: &WaylandSurface) {
        let transform = self.calculate_surface_transform(surface);
        match &surface.texture_source {
            TextureSource::DmaBuf(dmabuf_tex) => {
                self.base_scene.draw_image(&dmabuf_tex.image, transform);
            }
            TextureSource::Shm(shm_tex) => {
                self.base_scene.draw_image(&shm_tex.image, transform);
            }
            TextureSource::Egl(egl_tex) => {
                self.base_scene.draw_image(&egl_tex.image, transform);
            }
        }
    }
}
```

**Layer Management:**
```rust
pub struct LayerManager {
    background_layer: Vec<LayerSurface>,
    bottom_layer: Vec<LayerSurface>,
    top_layer: Vec<LayerSurface>,
    overlay_layer: Vec<LayerSurface>,
}

impl LayerManager {
    fn render_background_layer(&self, scene: &mut impl PaintScene) {
        for surface in &self.background_layer {
            self.render_layer_surface(surface, scene);
        }
    }
    
    fn render_layer_surface(&self, surface: &LayerSurface, scene: &mut impl PaintScene) {
        let transform = self.calculate_layer_transform(surface);
        match surface.exclusive_zone {
            ExclusiveZone::Neutral => {
                // Normal rendering
                scene.draw_image(&surface.texture, transform);
            }
            ExclusiveZone::Exclusive(zone) => {
                // Exclusive zone handling - adjust other surfaces
                self.apply_exclusive_zone_constraints(zone);
                scene.draw_image(&surface.texture, transform);
            }
        }
    }
}
```

**Damage Tracking:**
```rust
pub struct SurfaceDamageTracker {
    surface_damages: HashMap<ObjectId, Vec<Rectangle<i32, BufferCoord>>>,
    accumulated_damage: Vec<Rectangle<i32, Physical>>,
    last_frame_damage: Vec<Rectangle<i32, Physical>>,
}

impl SurfaceDamageTracker {
    fn track_surface_damage(&mut self, surface_id: ObjectId, 
                           damage: Vec<Rectangle<i32, BufferCoord>>) {
        self.surface_damages.insert(surface_id, damage);
    }
    
    fn calculate_frame_damage(&mut self, surfaces: &[WaylandSurface]) 
                             -> Vec<Rectangle<i32, Physical>> {
        let mut frame_damage = Vec::new();
        
        for surface in surfaces {
            if let Some(surface_damage) = self.surface_damages.get(&surface.id()) {
                for damage_rect in surface_damage {
                    // Buffer coordinate → Physical coordinate dönüşümü
                    let physical_damage = self.transform_damage_to_physical(
                        *damage_rect, surface
                    );
                    frame_damage.push(physical_damage);
                }
            }
        }
        
        // Damage rectangle'ları optimize et (merge overlapping)
        self.optimize_damage_regions(&mut frame_damage);
        frame_damage
    }
}
```

### 4. Memory Management

#### Problem Detayları
**Smithay Memory Patterns:**
- **Reference counting**: `Arc<Mutex<>>` ile shared ownership
- **Cleanup callbacks**: `CleanupResource` enum ile resource cleanup
- **Weak references**: `WeakDmabuf` ile circular reference prevention
- **EGL/DRM resource management**: Platform-specific cleanup

**Blitz Memory Patterns:**
- **Scene reset**: Her frame sonunda Vello scene temizleniyor
- **Device polling**: GPU işlemlerinin tamamlanması bekleniyor
- **Texture caching**: WGPU texture'ları cache'leniyor
- **Intermediate textures**: Compute shader için ara texture'lar

**Memory Challenges:**
- DMA-BUF texture'ların yaşam döngüsü yönetimi
- Cross-process memory sharing (DMA-BUF)
- GPU memory pressure handling
- Circular reference prevention (Surface ↔ Texture)

#### Çözüm Stratejisi
**Unified Resource Manager:**
```rust
pub struct ResourceManager {
    dmabuf_textures: HashMap<WeakDmabuf, Arc<BlitzTexture>>,
    texture_cache: LruCache<TextureId, CachedTexture>,
    cleanup_queue: VecDeque<CleanupTask>,
    reference_tracker: ReferenceTracker,
    memory_pressure_monitor: MemoryPressureMonitor,
}

impl ResourceManager {
    fn import_dmabuf(&mut self, dmabuf: &Dmabuf) -> Result<Arc<BlitzTexture>, ImportError> {
        // Memory pressure check
        if self.memory_pressure_monitor.is_under_pressure() {
            self.emergency_cleanup();
        }
        
        // Cache'de var mı kontrol et
        if let Some(cached) = self.dmabuf_textures.get(&dmabuf.weak()) {
            self.reference_tracker.touch_texture(&cached);
            return Ok(cached.clone());
        }
        
        // Yeni texture oluştur
        let wgpu_texture = self.convert_dmabuf_to_wgpu(dmabuf)?;
        let blitz_texture = Arc::new(BlitzTexture::from_wgpu(wgpu_texture, dmabuf.clone()));
        
        // Cache'e ekle ve reference tracking başlat
        self.dmabuf_textures.insert(dmabuf.weak(), blitz_texture.clone());
        self.reference_tracker.track_texture(&blitz_texture, dmabuf.weak());
        
        Ok(blitz_texture)
    }
    
    fn cleanup_unused_resources(&mut self) {
        // Kullanılmayan texture'ları temizle
        self.dmabuf_textures.retain(|weak_dmabuf, texture| {
            if weak_dmabuf.upgrade().is_none() || Arc::strong_count(texture) == 1 {
                self.cleanup_queue.push_back(CleanupTask::Texture(texture.clone()));
                false
            } else {
                true
            }
        });
        
        // LRU cache cleanup
        while self.texture_cache.len() > self.max_cache_size {
            if let Some((_, cached_texture)) = self.texture_cache.pop_lru() {
                self.cleanup_queue.push_back(CleanupTask::CachedTexture(cached_texture));
            }
        }
        
        // Cleanup queue'yu işle
        self.process_cleanup_queue();
    }
    
    fn emergency_cleanup(&mut self) {
        // Aggressive cleanup under memory pressure
        self.texture_cache.clear();
        self.cleanup_unused_resources();
        
        // Force GPU memory cleanup
        self.device.poll(wgpu::Maintain::Wait);
    }
}
```

**Automatic Cleanup System:**
```rust
pub struct AutoCleanup {
    cleanup_interval: Duration,
    last_cleanup: Instant,
    resource_manager: Arc<Mutex<ResourceManager>>,
    memory_threshold: usize,
}

impl AutoCleanup {
    fn maybe_cleanup(&mut self) {
        let should_cleanup = self.last_cleanup.elapsed() > self.cleanup_interval
            || self.check_memory_pressure();
            
        if should_cleanup {
            if let Ok(mut manager) = self.resource_manager.try_lock() {
                manager.cleanup_unused_resources();
                self.last_cleanup = Instant::now();
            }
        }
    }
    
    fn check_memory_pressure(&self) -> bool {
        // System memory usage check
        if let Ok(memory_info) = sys_info::mem_info() {
            let used_memory = memory_info.total - memory_info.free;
            let usage_ratio = used_memory as f64 / memory_info.total as f64;
            usage_ratio > 0.85 // 85% memory usage threshold
        } else {
            false
        }
    }
}
```

**Reference Tracking:**
```rust
pub struct ReferenceTracker {
    texture_refs: HashMap<TextureId, TextureRefInfo>,
    surface_refs: HashMap<ObjectId, SurfaceRefInfo>,
    last_access_times: HashMap<TextureId, Instant>,
}

struct TextureRefInfo {
    strong_refs: usize,
    weak_refs: usize,
    dmabuf_weak: WeakDmabuf,
    creation_time: Instant,
    last_access: Instant,
}

impl ReferenceTracker {
    fn track_texture(&mut self, texture: &Arc<BlitzTexture>, dmabuf_weak: WeakDmabuf) {
        let texture_id = texture.id();
        let ref_info = TextureRefInfo {
            strong_refs: Arc::strong_count(texture),
            weak_refs: Arc::weak_count(texture),
            dmabuf_weak,
            creation_time: Instant::now(),
            last_access: Instant::now(),
        };
        self.texture_refs.insert(texture_id, ref_info);
    }
    
    fn touch_texture(&mut self, texture: &Arc<BlitzTexture>) {
        let texture_id = texture.id();
        if let Some(ref_info) = self.texture_refs.get_mut(&texture_id) {
            ref_info.last_access = Instant::now();
            ref_info.strong_refs = Arc::strong_count(texture);
        }
        self.last_access_times.insert(texture_id, Instant::now());
    }
    
    fn find_unused_textures(&self, max_age: Duration) -> Vec<TextureId> {
        self.texture_refs
            .iter()
            .filter(|(_, ref_info)| {
                ref_info.strong_refs <= 1 && 
                ref_info.last_access.elapsed() > max_age
            })
            .map(|(texture_id, _)| *texture_id)
            .collect()
    }
}
```

## 🔧 Entegrasyon Mimarisi

### BlitzSmithayRenderer Implementation

```rust
pub struct BlitzSmithayRenderer {
    anyrender_backend: Box<dyn WindowRenderer>,
    wgpu_context: WGPUContext,
    format_converter: FormatConverter,
    coordinate_mapper: CoordinateMapper,
    resource_manager: Arc<Mutex<ResourceManager>>,
    surface_manager: WaylandSurfaceManager,
    multi_surface_scene: MultiSurfaceScene,
}

impl Renderer for BlitzSmithayRenderer {
    type Error = BlitzSmithayError;
    type TextureId = BlitzTexture;
    type Framebuffer<'buffer> = BlitzFramebuffer<'buffer>;
    type Frame<'frame, 'buffer> = BlitzFrame<'frame, 'buffer>;
    
    fn render(&mut self, framebuffer: &mut Self::Framebuffer<'_>, 
              output_size: Size<i32, Physical>, dst_transform: Transform) 
              -> Result<Self::Frame<'_, '_>, Self::Error> {
        // Blitz rendering context oluştur
        let paint_scene = BlitzPaintScene::new(framebuffer, output_size);
        let frame = BlitzFrame::new(self, paint_scene, dst_transform);
        
        // Auto cleanup
        self.resource_manager.lock().unwrap().maybe_cleanup();
        
        Ok(frame)
    }
}

impl ImportDma for BlitzSmithayRenderer {
    fn import_dmabuf(&mut self, dmabuf: &Dmabuf, 
                     damage: Option<&[Rectangle<i32, BufferCoord>]>) 
                     -> Result<Self::TextureId, Self::Error> {
        // Resource manager üzerinden import
        let texture = self.resource_manager
            .lock()
            .unwrap()
            .import_dmabuf(dmabuf)?;
            
        // Damage tracking
        if let Some(damage_rects) = damage {
            self.surface_manager.track_damage(dmabuf.weak(), damage_rects);
        }
        
        Ok((*texture).clone())
    }
    
    fn dmabuf_formats(&self) -> FormatSet {
        self.format_converter.supported_formats()
    }
}
```

### AnyRender Genişletmeleri

```rust
// AnyRender'a DMA-BUF desteği ekle
pub enum TextureSource {
    Memory(MemoryTexture),
    DmaBuf(DmaBufTexture),
    Wgpu(WgpuTexture),
    Egl(EglTexture),
}

pub struct DmaBufTexture {
    pub image: Image,
    pub dmabuf: Dmabuf,
    pub wgpu_texture: wgpu::Texture,
    pub format: TextureFormat,
}

impl PaintScene for BlitzPaintScene {
    fn draw_wayland_surface(&mut self, surface: &WaylandSurface, 
                           transform: Affine) {
        match &surface.texture_source {
            TextureSource::DmaBuf(dmabuf_tex) => {
                // DMA-BUF texture'ını sahneye çiz
                self.draw_image(&dmabuf_tex.image, transform);
            }
            TextureSource::Shm(shm_tex) => {
                // Shared memory texture
                self.draw_image(&shm_tex.image, transform);
            }
            TextureSource::Egl(egl_tex) => {
                // EGL texture
                self.draw_image(&egl_tex.image, transform);
            }
            _ => { /* diğer texture türleri */ }
        }
    }
    
    fn draw_image_with_alpha(&mut self, image: &Image, transform: Affine, 
                            alpha: f32) {
        // Alpha blending desteği
        let brush = self.create_alpha_brush(alpha);
        self.fill(&image.as_shape().transformed(transform), &brush);
    }
}
```

## 🏗️ İmplementasyon Aşamaları

### Faz 1: Temel Altyapı (2-3 hafta)
**Hedefler:**
- `blitz-smithay` crate oluşturma
- Temel trait implementasyonları
- Format conversion layer temel yapısı

**Deliverables:**
```rust
// blitz-smithay/Cargo.toml
[dependencies]
smithay = { version = "0.3", features = ["backend_drm", "backend_gbm", "renderer_gl", "wayland_frontend"] }
wgpu = { version = "24", features = ["hal"] }
anyrender = { path = "../anyrender" }
anyrender_vello = { path = "../anyrender_vello" }
drm = "0.12"
gbm = "0.15"
wayland-server = "0.31"
egl = "0.2"

// blitz-smithay/src/lib.rs
pub struct BlitzSmithayRenderer { /* ... */ }
impl Renderer for BlitzSmithayRenderer { /* temel implementasyon */ }
```

**Kritik Milestone'lar:**
- [ ] Temel renderer trait implementasyonu
- [ ] WGPU context entegrasyonu
- [ ] Error handling framework
- [ ] Unit test framework kurulumu

### Faz 2: DMA-BUF Entegrasyonu (3-4 hafta)
**Hedefler:**
- WGPU DMA-BUF import implementasyonu
- Format conversion pipeline
- Texture caching sistemi

**Teknik Detaylar:**
```rust
impl WGPUContext {
    fn import_dmabuf_texture(&mut self, dmabuf: &Dmabuf) 
                            -> Result<wgpu::Texture, WgpuError> {
        // EGL/Vulkan interop ile DMA-BUF import
        let external_texture = unsafe {
            self.device.create_texture_from_hal(
                hal_texture_from_dmabuf(dmabuf)?,
                &wgpu::TextureDescriptor {
                    label: Some("DMA-BUF Texture"),
                    size: wgpu::Extent3d {
                        width: dmabuf.width(),
                        height: dmabuf.height(),
                        depth_or_array_layers: 1,
                    },
                    mip_level_count: 1,
                    sample_count: 1,
                    dimension: wgpu::TextureDimension::D2,
                    format: self.convert_dmabuf_format(dmabuf.format())?,
                    usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
                    view_formats: &[],
                }
            )
        };
        Ok(external_texture)
    }
}
```

**Kritik Milestone'lar:**
- [ ] DMA-BUF import working
- [ ] Format conversion pipeline
- [ ] Performance benchmarking
- [ ] Memory leak testing

### Faz 3: Multi-Surface Rendering (2-3 hafta)
**Hedefler:**
- Surface layering sistemi
- Z-order management
- Event routing sistemi

**Teknik Detaylar:**
```rust
pub struct WaylandSurfaceManager {
    surfaces: HashMap<ObjectId, WaylandSurface>,
    damage_tracker: SurfaceDamageTracker,
    layer_manager: LayerManager,
    subsurface_tree: SubsurfaceTree,
}

impl WaylandSurfaceManager {
    fn commit_surface(&mut self, surface_id: ObjectId, 
                     buffer: Option<WlBuffer>) -> Result<(), SurfaceError> {
        // Surface commit işlemi
        let surface = self.surfaces.get_mut(&surface_id)
            .ok_or(SurfaceError::SurfaceNotFound)?;
            
        if let Some(buffer) = buffer {
            // Buffer import ve texture oluşturma
            let texture = self.import_buffer(buffer)?;
            surface.set_texture(texture);
        }
        
        // Damage tracking güncelleme
        self.damage_tracker.commit_surface_damage(surface_id);
        
        Ok(())
    }
}
```

**Kritik Milestone'lar:**
- [ ] Multi-surface rendering working
- [ ] Layer shell protocol support
- [ ] Subsurface hierarchy support
- [ ] Damage tracking optimization

### Faz 4: Production Readiness (2-3 hafta)
**Hedefler:**
- Memory management optimizasyonu
- Error handling iyileştirmeleri
- Performance profiling ve optimizasyon

**Kritik Milestone'lar:**
- [ ] Memory usage optimization
- [ ] Error recovery mechanisms
- [ ] Performance targets met (60 FPS)
- [ ] Integration testing complete

## 📊 Risk Analizi ve Mitigasyon

### Yüksek Risk Alanları

#### 1. WGPU-DMA-BUF Interop (Risk: Yüksek)
**Riskler:**
- Platform-specific implementation farklılıkları
- Driver compatibility issues
- Performance degradation

**Mitigasyon:**
- Extensive platform testing (Intel, AMD, NVIDIA)
- Fallback mechanisms (software conversion)
- Performance benchmarking suite

#### 2. Memory Management (Risk: Orta-Yüksek)
**Riskler:**
- Memory leaks in cross-process sharing
- GPU memory pressure
- Reference counting bugs

**Mitigasyon:**
- Comprehensive memory testing
- Automated leak detection
- Reference counting audits

#### 3. Coordinate System Complexity (Risk: Orta)
**Riskler:**
- Transform calculation errors
- Multi-output coordinate bugs
- HiDPI scaling issues

**Mitigasyon:**
- Extensive coordinate system testing
- Reference implementation validation
- Visual regression testing

### Mitigasyon Stratejileri

#### Platform Testing
```bash
# Test matrix
- Linux + Intel Graphics + Wayland
- Linux + AMD Graphics + Wayland  
- Linux + NVIDIA Graphics + Wayland
- Different Wayland compositors (Sway, GNOME, KDE)
```

#### Performance Monitoring
```rust
pub struct PerformanceMonitor {
    frame_times: VecDeque<Duration>,
    memory_usage: VecDeque<usize>,
    gpu_usage: VecDeque<f32>,
}

impl PerformanceMonitor {
    fn record_frame(&mut self, frame_time: Duration) {
        self.frame_times.push_back(frame_time);
        if self.frame_times.len() > 1000 {
            self.frame_times.pop_front();
        }
        
        // Performance regression detection
        if self.average_frame_time() > Duration::from_millis(16) {
            warn!("Performance regression detected: avg frame time > 16ms");
        }
    }
}
```

## 🎯 Başarı Kriterleri

### Teknik Kriterler
- [ ] **DMA-BUF Import**: DMA-BUF texture'ları başarıyla import edilebiliyor
- [ ] **Multi-Surface Rendering**: Birden fazla Wayland surface aynı anda render ediliyor
- [ ] **Memory Management**: Memory leak'ler yok, stable memory usage
- [ ] **Performance**: 60 FPS stable performance, <16ms frame time
- [ ] **Format Support**: En az 4 farklı DMA-BUF format destekleniyor
- [ ] **Transform Support**: Tüm Wayland transform'ları doğru çalışıyor

### Fonksiyonel Kriterler
- [ ] **Wayland Client Support**: GTK, Qt uygulamaları görüntülenebiliyor
- [ ] **Input Events**: Mouse, keyboard, touch events doğru routing ediliyor
- [ ] **Layer Shell**: Background, bottom, top, overlay layer'ları çalışıyor
- [ ] **HiDPI Support**: Fractional scaling doğru çalışıyor
- [ ] **Multi-Output**: Birden fazla monitor desteği
- [ ] **Subsurfaces**: Parent-child surface hierarchy desteği

### Integration Test Scenarios
```rust
#[test]
fn test_gtk_application_rendering() {
    // GTK uygulaması başlat
    let gtk_client = launch_gtk_app("gtk3-demo");
    
    // Blitz-Smithay compositor'da render et
    let compositor = BlitzSmithayCompositor::new();
    compositor.add_client(gtk_client);
    
    // Render frame ve verify
    let frame = compositor.render_frame();
    assert!(frame.contains_gtk_content());
    assert!(frame.frame_time() < Duration::from_millis(16));
}

#[test]
fn test_multi_surface_layering() {
    let compositor = BlitzSmithayCompositor::new();
    
    // Background layer surface
    let bg_surface = create_layer_surface(Layer::Background);
    compositor.add_layer_surface(bg_surface);
    
    // Normal window
    let window = create_window_surface();
    compositor.add_window(window);
    
    // Overlay surface
    let overlay = create_layer_surface(Layer::Overlay);
    compositor.add_layer_surface(overlay);
    
    // Verify layering order
    let frame = compositor.render_frame();
    assert_eq!(frame.layer_order(), vec![Layer::Background, Layer::Window, Layer::Overlay]);
}
```

## 🚀 Örnek Kullanım

### Minimal Compositor
```rust
use blitz_smithay::{BlitzSmithayRenderer, BlitzSmithayCompositor};
use smithay::wayland::compositor::CompositorState;

fn main() -> Result<(), Box<dyn Error>> {
    // Wayland display oluştur
    let mut display = Display::new()?;
    let dh = display.handle();
    
    // Blitz-Smithay renderer oluştur
    let mut renderer = BlitzSmithayRenderer::new()?;
    
    // DMA-BUF feedback oluştur
    let dmabuf_formats = renderer.dmabuf_formats();
    let feedback = DmabufFeedbackBuilder::new(main_device, dmabuf_formats)
        .build()?;
    
    // Compositor state
    let mut state = CompositorState::new::<State>(&dh);
    let dmabuf_global = state.create_global_with_default_feedback::<State>(
        &dh, &feedback
    );
    
    // Event loop
    loop {
        // Wayland events işle
        display.dispatch_clients(&mut state)?;
        
        // Render frame
        let (framebuffer, damage) = prepare_frame()?;
        let mut frame = renderer.render(&mut framebuffer, output_size, transform)?;
        
        // Wayland surfaces render et
        for surface in state.surfaces() {
            render_wayland_surface(&mut frame, surface)?;
        }
        
        frame.finish()?;
        present_frame(framebuffer)?;
    }
}
```

### Advanced Usage with Layer Shell
```rust
use blitz_smithay::{BlitzSmithayCompositor, LayerShellHandler};

struct MyCompositor {
    blitz_smithay: BlitzSmithayCompositor,
    layer_shell: LayerShellHandler,
}

impl MyCompositor {
    fn handle_layer_surface_request(&mut self, surface: LayerSurface) {
        match surface.layer {
            Layer::Background => {
                // Wallpaper, desktop background
                self.blitz_smithay.add_background_surface(surface);
            }
            Layer::Bottom => {
                // Desktop widgets, docks
                self.blitz_smithay.add_bottom_surface(surface);
            }
            Layer::Top => {
                // Panels, status bars
                self.blitz_smithay.add_top_surface(surface);
            }
            Layer::Overlay => {
                // Notifications, screen lockers
                self.blitz_smithay.add_overlay_surface(surface);
            }
        }
    }
    
    fn render_complete_desktop(&mut self) -> Result<(), RenderError> {
        let mut frame = self.blitz_smithay.begin_frame()?;
        
        // Layer shell rendering order
        frame.render_background_layer()?;
        frame.render_bottom_layer()?;
        frame.render_windows()?;
        frame.render_top_layer()?;
        frame.render_overlay_layer()?;
        
        frame.finish()
    }
}
```

## 🎯 Sonuç ve Faydalar

Bu kapsamlı entegrasyon şunları sağlayacak:

### Teknik Faydalar
1. **Web Content in Wayland**: HTML/CSS içeriğinin native Wayland uygulamaları olarak render edilmesi
2. **Zero-Copy Performance**: DMA-BUF sharing ile optimal performans
3. **Modular Architecture**: Farklı rendering backend'leri için esnek tasarım
4. **Standards Compliance**: Tam Wayland protokol uyumluluğu

### Kullanım Senaryoları
1. **Desktop Environment**: Blitz tabanlı Wayland compositor
2. **Embedded Systems**: IoT cihazlarda web-based UI
3. **Digital Signage**: Web content ile native graphics karışımı
4. **Gaming**: Web UI ile native game content entegrasyonu

### Ecosystem Impact
- **Rust Wayland Ecosystem**: Smithay'e yeni renderer backend
- **Web Rendering**: Native performance ile web content
- **Cross-Platform**: Linux/Wayland'da modern web teknolojileri

Entegrasyon tamamlandığında, Blitz web rendering engine'i güçlü bir Wayland compositor haline gelecek ve modern web teknolojileri ile native Wayland uygulamalarını aynı sahne içinde gösterebilecek. Bu, web teknolojilerinin native performance ile buluştuğu yeni bir paradigma yaratacaktır.

## 📚 Referanslar

- [Smithay Documentation](https://smithay.github.io/smithay/)
- [Wayland Protocol Specification](https://wayland.freedesktop.org/docs/html/)
- [DMA-BUF Linux Kernel Documentation](https://www.kernel.org/doc/html/latest/driver-api/dma-buf.html)
- [WGPU Documentation](https://docs.rs/wgpu/)
- [Blitz Repository](https://github.com/waluenetwork/blitz)
- [Anvil Compositor Example](https://github.com/Smithay/smithay/tree/master/anvil)
