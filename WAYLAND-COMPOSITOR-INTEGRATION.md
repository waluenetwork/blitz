# Wayland Compositor Entegrasyonu: Analiz ve Çözüm Yol Haritası

Bu dokümantasyon, Blitz projesindeki Wayland compositor entegrasyonunun mevcut durumunu, karşılaşılan sorunları ve çözüm yollarını detaylı olarak analiz etmektedir. Smithay tabanlı Wayland compositor entegrasyonu, mock (sahte) implementasyon ile gerçek Wayland protokol implementasyonu arasındaki geçişi sağlamak için gerekli adımları içermektedir.

## İçindekiler

1. [Genel Bakış](#genel-bakış)
2. [Mevcut Durum Analizi](#mevcut-durum-analizi)
3. [Sistem Bağımlılıkları ve Derleme Sorunları](#sistem-bağımlılıkları-ve-derleme-sorunları)
4. [Mock vs Gerçek Implementasyon Karşılaştırması](#mock-vs-gerçek-implementasyon-karşılaştırması)
5. [Surface Rendering Pipeline Analizi](#surface-rendering-pipeline-analizi)
6. [WGPU Texture Entegrasyonu](#wgpu-texture-entegrasyonu)
7. [Wayland Client Bağlantı Yönetimi](#wayland-client-bağlantı-yönetimi)
8. [Çözüm Yol Haritası](#çözüm-yol-haritası)
9. [Teknik Detaylar ve Referanslar](#teknik-detaylar-ve-referanslar)

## Genel Bakış

Blitz-Smithay entegrasyonu, Smithay Wayland compositor kütüphanesini kullanarak Blitz rendering motoruna Wayland compositor yetenekleri kazandırmayı amaçlamaktadır. Bu entegrasyon, Wayland istemcilerinin (örneğin terminal uygulamaları) Blitz tarafından yönetilen bir Wayland compositor'a bağlanmasını ve içeriklerinin WGPU tabanlı rendering sistemi ile görüntülenmesini sağlar.

Mevcut implementasyon iki farklı modda çalışabilmektedir:
1. **Mock Simulation Modu**: Gerçek Wayland protokolü kullanmadan, sahte yüzeyler ve istemciler oluşturarak temel entegrasyon yapısını test etmeyi sağlar.
2. **Gerçek Wayland Modu**: Smithay kütüphanesini kullanarak tam bir Wayland compositor implementasyonu sağlar, ancak şu anda derleme ve çalışma zamanı sorunları bulunmaktadır.

## Mevcut Durum Analizi

Şu anda Blitz-Smithay entegrasyonu aşağıdaki bileşenlerden oluşmaktadır:

- **BlitzSmithayRenderer**: OpenGL ve WGPU arasında texture köprüleme işlemlerini gerçekleştirir
- **SurfaceCompositor**: Wayland yüzeylerini yönetir ve WGPU texture'larına render eder
- **WaylandSurfaceManager**: Yüzey yaşam döngüsü ve hiyerarşi yönetimini sağlar
- **SmithayPaintSource**: CustomPaintSource implementasyonu ile Wayland içeriğini Blitz rendering sistemine entegre eder

Mevcut sorunlar:
1. `smithay-backend` feature'ı etkinleştirildiğinde libseat-sys bağımlılığı nedeniyle derleme hatası oluşmaktadır
2. Mock implementasyon gerçek Wayland yüzeylerini gösterememektedir
3. HTML tabanlı UI yerine Dioxus tabanlı UI'a geçiş gerekmektedir
4. Gerçek Wayland istemci bağlantıları çalışmamaktadır

## Sistem Bağımlılıkları ve Derleme Sorunları

Smithay entegrasyonu için gerekli sistem bağımlılıkları:

```bash
# Temel Wayland bağımlılıkları
sudo apt-get install libwayland-dev libxkbcommon-dev

# Smithay için gerekli ek bağımlılıklar
sudo apt-get install libudev-dev libseat-dev libinput-dev

# OpenGL ve WGPU entegrasyonu için
sudo apt-get install libgbm-dev libdrm-dev libegl1-mesa-dev
```

Mevcut derleme hatası, `libseat-sys` bağımlılığının `libseat.pc` dosyasını bulamamasından kaynaklanmaktadır. Debug çıktısında görülen hata:

```
error: failed to run custom build command for `libseat-sys v0.1.9`
...
pkg-config: No such file or directory (os error 2)
```

Çözüm için `PKG_CONFIG_PATH` çevre değişkeninin doğru şekilde ayarlanması veya `libseat-dev` paketinin yüklenmesi gerekmektedir.

## Mock vs Gerçek Implementasyon Karşılaştırması

### Mock Implementasyon (Mevcut)

```rust
// Mock WaylandCompositorState yapısı
#[cfg(not(feature = "smithay-backend"))]
struct WaylandCompositorState {
    socket_name: String,
    client_count: u32,
    surface_count: u32,
    start_time: Instant,
}

// Mock dispatch_wayland_events implementasyonu
#[cfg(not(feature = "smithay-backend"))]
fn dispatch_wayland_events(&mut self) {
    // Zaman bazlı mock yüzey oluşturma
    if let Some(ref wayland_state) = self.wayland_state {
        let elapsed = self.start_time.elapsed().as_secs();
        
        if elapsed >= 2 && wayland_state.client_count == 0 {
            // İlk mock yüzeyi oluştur
            self.create_mock_surface_with_texture(400, 300, [0.2, 0.8, 0.2, 1.0]);
            // ...
        }
        
        if elapsed >= 5 && wayland_state.surface_count == 1 {
            // İkinci mock yüzeyi oluştur
            self.create_mock_surface_with_texture(300, 200, [0.8, 0.2, 0.2, 1.0]);
            // ...
        }
    }
}
```

### Gerçek Wayland Implementasyonu (Hedef)

```rust
// Gerçek WaylandCompositorState yapısı
#[cfg(feature = "smithay-backend")]
struct WaylandCompositorState {
    display: Display<SmithayApp>,
    listener: ListeningSocket,
    clients: Vec<Client>,
    app_state: SmithayApp,
    socket_name: String,
    start_time: Instant,
}

// Gerçek dispatch_wayland_events implementasyonu
#[cfg(feature = "smithay-backend")]
fn dispatch_wayland_events(&mut self) {
    if let Some(ref mut wayland_state) = self.wayland_state {
        // Yeni client bağlantılarını kabul et
        if let Ok(Some(stream)) = wayland_state.listener.accept() {
            // Client'ı ekle
            // ...
        }
        
        // Wayland protokol mesajlarını işle
        if let Err(e) = wayland_state.display.dispatch_clients(&mut wayland_state.app_state) {
            // Hata işleme
        }
        
        // Client'lara yanıtları gönder
        if let Err(e) = wayland_state.display.flush_clients() {
            // Hata işleme
        }
    }
}
```

## Surface Rendering Pipeline Analizi

Mevcut surface rendering pipeline şu adımlardan oluşmaktadır:

1. **Wayland Surface Oluşturma**: İstemci bir yüzey oluşturur ve buffer commit eder
2. **Buffer Processing**: Buffer içeriği işlenir ve OpenGL texture'ına dönüştürülür
3. **Texture Bridging**: OpenGL texture'ı WGPU texture'ına dönüştürülür
4. **Compositor Rendering**: Tüm yüzeyler doğru Z-order ile kompozit edilir
5. **WGPU Rendering**: Sonuç WGPU texture'ı Blitz rendering sistemine aktarılır

Mevcut sorunlar:

1. Mock implementasyonda gerçek Wayland buffer'ları olmadığı için texture bridging adımı atlanmaktadır
2. Gerçek implementasyonda `SurfaceCompositor` ve Wayland protokol handler'ları arasındaki entegrasyon eksiktir
3. Yüzey hiyerarşisi ve Z-order yönetimi tam olarak implementasyonu tamamlanmamıştır

## WGPU Texture Entegrasyonu

WGPU texture entegrasyonu, OpenGL texture'larını WGPU texture'larına dönüştürmek için iki yöntem kullanmaktadır:

1. **DMA-BUF Tabanlı Paylaşım**: Linux sistemlerinde DMA-BUF mekanizması ile zero-copy texture paylaşımı
2. **CPU Üzerinden Kopyalama**: DMA-BUF desteklenmeyen durumlarda CPU üzerinden texture içeriği kopyalanır

Mevcut implementasyonda `BlitzTexture` yapısı bu entegrasyonu sağlamaktadır:

```rust
pub struct BlitzTexture {
    id: TextureId,
    width: u32,
    height: u32,
    format: String,
    dmabuf_info: Option<DmaBufInfo>,
    wgpu_texture: Option<wgpu::Texture>,
    created_at: std::time::Instant,
}

impl BlitzTexture {
    // DMA-BUF bilgisi ile WGPU texture oluşturma
    pub fn from_wgpu_texture_with_dmabuf(width: u32, height: u32, format: String, dmabuf_info: DmaBufInfo, wgpu_texture: wgpu::Texture) -> Self {
        // ...
    }
    
    // Doğrudan WGPU texture'dan oluşturma
    pub fn from_wgpu_texture(texture: wgpu::Texture) -> Self {
        // ...
    }
}
```

## Wayland Client Bağlantı Yönetimi

Wayland client bağlantı yönetimi, Smithay kütüphanesinin `Display` ve `ListeningSocket` yapıları ile gerçekleştirilir:

```rust
// Wayland socket oluşturma ve bağlantı dinleme
let socket_name = format!("wayland-blitz-{}", std::process::id());
let listener = ListeningSocket::bind(&socket_name)?;

// Client bağlantılarını kabul etme
if let Ok(Some(stream)) = listener.accept() {
    let client = display
        .handle()
        .insert_client(stream, Arc::new(ClientState::default()))?;
    clients.push(client);
}

// Client mesajlarını işleme
display.dispatch_clients(&mut app_state)?;
display.flush_clients()?;
```

Mevcut sorunlar:
1. Gerçek Wayland socket bağlantısı oluşturulamamaktadır
2. Client bağlantıları kabul edilse bile protokol handler'ları doğru şekilde çalışmamaktadır
3. Wayland protokol implementasyonu için gerekli trait'ler tam olarak implementasyonu tamamlanmamıştır

## Çözüm Yol Haritası

### 1. Sistem Bağımlılıkları Sorununun Çözümü

```bash
# libseat-sys için gerekli paketleri yükle
sudo apt-get install libseat-dev

# PKG_CONFIG_PATH'i ayarla
export PKG_CONFIG_PATH=/usr/lib/x86_64-linux-gnu/pkgconfig
```

### 2. HTML'den Dioxus'a Geçiş

```rust
// Mevcut HTML tabanlı implementasyon
fn main() -> Result<(), Box<dyn std::error::Error>> {
    // ...
    let html = r#"
        <!DOCTYPE html>
        <html>
        <head>
            <style>/* ... */</style>
        </head>
        <body>
            <!-- ... -->
        </body>
        </html>
    "#;
    // ...
}

// Hedef Dioxus tabanlı implementasyon
fn main() -> Result<(), Box<dyn std::error::Error>> {
    // ...
    mini_dxn::launch(app);
    Ok(())
}

fn app() -> Element {
    rsx! {
        style { {CSS} }
        div {
            class: "container",
            div {
                class: "overlay",
                // ...
            }
            div {
                class: "canvas-container",
                SmithayCompositor {}
            }
        }
    }
}

#[component]
fn SmithayCompositor() -> Element {
    let smithay_paint_source = SmithayPaintSource::new().expect("Failed to create SmithayPaintSource");
    let paint_source_id = use_wgpu(move || smithay_paint_source);

    rsx! {
        canvas {
            class: "compositor-canvas",
            tabindex: "0",
            width: "800",
            height: "600",
            "src": paint_source_id
        }
    }
}
```

### 3. Surface Rendering Pipeline İyileştirmeleri

1. **Mock Surface Oluşturma İyileştirmesi**:
```rust
fn create_mock_surface_with_texture(&mut self, width: u32, height: u32, color: [f32; 4]) {
    // WGPU texture oluştur
    let texture = device.create_texture(&wgpu::TextureDescriptor { /* ... */ });
    
    // Texture içeriğini doldur
    queue.write_texture(/* ... */);
    
    // BlitzTexture oluştur
    let blitz_texture = BlitzTexture::from_wgpu_texture(texture);
    
    // Surface compositor'a ekle
    if let Some(ref surface_compositor) = self.surface_compositor {
        let mut compositor = surface_compositor.lock().unwrap();
        compositor.add_surface(surface_id)?;
        compositor.set_surface_texture(surface_id, blitz_texture)?;
    }
}
```

2. **Gerçek Wayland Surface Entegrasyonu**:
```rust
impl CompositorHandler for SmithayApp {
    // ...
    fn commit(&mut self, surface: &WlSurface) {
        // Buffer değişikliklerini kontrol et
        let has_buffer = with_states(surface, |states| {
            let attrs = states.cached_state.current::<SurfaceAttributes>();
            matches!(attrs.buffer, Some(BufferAssignment::NewBuffer(_)))
        });
        
        if has_buffer {
            // Buffer'ı OpenGL texture'ına dönüştür
            // OpenGL texture'ını WGPU texture'ına dönüştür
            // Surface compositor'a ekle
        }
    }
}
```

### 4. Wayland Protocol Handler Implementasyonları

```rust
// XdgShellHandler implementasyonu
impl XdgShellHandler for SmithayApp {
    fn xdg_shell_state(&mut self) -> &mut XdgShellState {
        &mut self.xdg_shell_state
    }

    fn new_toplevel(&mut self, surface: ToplevelSurface) {
        // Yeni toplevel yüzeyi yapılandır
        surface.with_pending_state(|state| {
            state.states.set(xdg_toplevel::State::Activated);
        });
        surface.send_configure();
        
        // Surface compositor'a bildir
        if let Some(ref surface_compositor) = self.surface_compositor {
            let mut compositor = surface_compositor.lock().unwrap();
            let surface_id = ObjectId::new();
            compositor.add_surface(surface_id).ok();
            // Yüzey bilgilerini kaydet
        }
    }
    
    // Diğer handler metodları...
}
```

### 5. Client Bağlantı Yönetimi İyileştirmeleri

```rust
fn setup_wayland_compositor(&mut self) {
    // Display ve socket oluştur
    let socket_name = format!("wayland-blitz-{}", std::process::id());
    let display = Display::new()?;
    let listener = ListeningSocket::bind(&socket_name)?;
    
    // Wayland global'leri oluştur
    let compositor_state = CompositorState::new::<SmithayApp>(&display.handle());
    let xdg_shell_state = XdgShellState::new::<SmithayApp>(&display.handle());
    // ...
    
    // Çevre değişkenlerini ayarla
    std::env::set_var("WAYLAND_DISPLAY", &socket_name);
    
    // Test client'ı başlat
    self.spawn_test_client();
}
```

## Teknik Detaylar ve Referanslar

### Smithay Compositor Trait Gereksinimleri

Smithay compositor implementasyonu için aşağıdaki trait'lerin implementasyonu gereklidir:

1. **CompositorHandler**: Yüzey yaşam döngüsü ve buffer commit işlemleri
2. **XdgShellHandler**: Toplevel ve popup yüzey yönetimi
3. **ShmHandler**: Shared memory buffer yönetimi
4. **SeatHandler**: Klavye, fare ve dokunmatik girdi yönetimi
5. **DataDeviceHandler**: Veri transferi ve clipboard yönetimi

### WGPU Texture Format Dönüşümleri

| Wayland Format | WGPU Format | Açıklama |
|----------------|-------------|----------|
| ARGB8888 | Rgba8Unorm | Alfa kanallı 32-bit RGBA |
| XRGB8888 | Rgba8Unorm | Alfa kanalsız 32-bit RGB |
| ABGR8888 | Bgra8Unorm | Alfa kanallı 32-bit BGRA |
| RGB565 | R5g6b5Unorm | 16-bit RGB |

### Smithay Minimal Örnek Referansı

Smithay minimal örneği, temel bir Wayland compositor implementasyonu için gerekli yapıyı göstermektedir:

```rust
struct App {
    compositor_state: CompositorState,
    xdg_shell_state: XdgShellState,
    shm_state: ShmState,
    seat_state: SeatState<Self>,
    data_device_state: DataDeviceState,
    seat: Seat<Self>,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Display ve socket oluştur
    let mut display: Display<App> = Display::new()?;
    let listener = ListeningSocket::bind("wayland-5").unwrap();
    
    // State yapısını oluştur
    let state = App { /* ... */ };
    
    // Event loop
    loop {
        // Client bağlantılarını kabul et
        if let Some(stream) = listener.accept()? {
            let client = display.handle().insert_client(stream, Arc::new(ClientState::default()))?;
            // ...
        }
        
        // Client mesajlarını işle
        display.dispatch_clients(&mut state)?;
        display.flush_clients()?;
        
        // Rendering işlemleri
        // ...
    }
}
```

### Blitz-Smithay Entegrasyon Noktaları

1. **CustomPaintSource**: Wayland içeriğini Blitz rendering sistemine entegre eder
2. **BlitzSmithayRenderer**: OpenGL-WGPU texture köprülemesi sağlar
3. **SurfaceCompositor**: Wayland yüzeylerini yönetir ve kompozit eder
4. **WaylandSurfaceManager**: Yüzey hiyerarşisi ve yaşam döngüsü yönetimi sağlar

Bu entegrasyon noktaları, Smithay Wayland compositor işlevselliğini Blitz rendering sistemi ile birleştirerek tam bir Wayland compositor implementasyonu oluşturmayı sağlar.
