
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use lru::LruCache;
use tracing::{debug, warn};

use crate::{BlitzTexture, TextureId, BlitzSmithayError};

pub struct ResourceManager {
    texture_cache: LruCache<TextureId, CachedTexture>,
    
    cleanup_queue: Vec<CleanupTask>,
    
    reference_tracker: ReferenceTracker,
    
    memory_pressure_monitor: MemoryPressureMonitor,
    
    max_cache_size: usize,
    cleanup_interval: Duration,
    last_cleanup: Instant,
}

impl ResourceManager {
    pub fn new() -> Self {
        debug!("DEBUG: Initializing ResourceManager");
        
        Self {
            texture_cache: LruCache::new(std::num::NonZeroUsize::new(100).unwrap()),
            cleanup_queue: Vec::new(),
            reference_tracker: ReferenceTracker::new(),
            memory_pressure_monitor: MemoryPressureMonitor::new(),
            max_cache_size: 100,
            cleanup_interval: Duration::from_secs(5),
            last_cleanup: Instant::now(),
        }
    }
    
    pub fn create_texture(&mut self, width: u32, height: u32, format: String) -> Result<Arc<BlitzTexture>, BlitzSmithayError> {
        debug!("DEBUG: Creating texture - format={} size={}x{}", format, width, height);
        
        if self.memory_pressure_monitor.is_under_pressure() {
            warn!("DEBUG: Memory pressure detected, performing emergency cleanup");
            self.emergency_cleanup();
        }
        
        debug!("DEBUG: Creating new texture");
        let blitz_texture = Arc::new(BlitzTexture::new(width, height, format));
        
        self.reference_tracker.track_texture_simple(&blitz_texture);
        
        debug!("DEBUG: Texture creation successful - texture_id={:?}", blitz_texture.id());
        Ok(blitz_texture)
    }
    
    pub fn maybe_cleanup(&mut self) {
        if self.last_cleanup.elapsed() > self.cleanup_interval {
            debug!("DEBUG: Performing periodic cleanup");
            self.cleanup_unused_resources();
            self.last_cleanup = Instant::now();
        }
    }
    
    pub fn cleanup_unused_resources(&mut self) {
        debug!("DEBUG: Starting resource cleanup");
        
        let initial_count = self.texture_cache.len();
        
        debug!("DEBUG: Simplified cleanup - checking texture cache only");
        
        while self.texture_cache.len() > self.max_cache_size {
            if let Some((texture_id, cached_texture)) = self.texture_cache.pop_lru() {
                debug!("DEBUG: Evicting texture {:?} from LRU cache", texture_id);
                self.cleanup_queue.push(CleanupTask::CachedTexture(cached_texture));
            }
        }
        
        self.process_cleanup_queue();
        
        let cleaned_count = initial_count - self.texture_cache.len();
        if cleaned_count > 0 {
            debug!("DEBUG: Cleaned up {} textures", cleaned_count);
        }
    }
    
    pub fn emergency_cleanup(&mut self) {
        warn!("DEBUG: Performing emergency cleanup due to memory pressure");
        
        self.texture_cache.clear();
        
        self.cleanup_unused_resources();
        
    }
    
    fn create_texture_internal(&mut self, width: u32, height: u32, format: &str) -> Result<BlitzTexture, BlitzSmithayError> {
        debug!("DEBUG: Creating internal texture {}x{} format={}", width, height, format);
        
        Ok(BlitzTexture::new(width, height, format.to_string()))
    }
    
    fn process_cleanup_queue(&mut self) {
        debug!("DEBUG: Processing {} cleanup tasks", self.cleanup_queue.len());
        
        for task in self.cleanup_queue.drain(..) {
            match task {
                CleanupTask::Texture(texture) => {
                    debug!("DEBUG: Cleaning up texture {:?}", texture.id());
                }
                CleanupTask::CachedTexture(cached) => {
                    debug!("DEBUG: Cleaning up cached texture {:?}", cached.texture_id);
                }
            }
        }
    }
}

#[derive(Debug)]
pub struct CachedTexture {
    pub texture_id: TextureId,
    pub texture: Arc<BlitzTexture>,
    pub last_access: Instant,
    pub access_count: u64,
}

#[derive(Debug)]
pub enum CleanupTask {
    Texture(Arc<BlitzTexture>),
    CachedTexture(CachedTexture),
}

pub struct ReferenceTracker {
    texture_refs: HashMap<TextureId, TextureRefInfo>,
    last_access_times: HashMap<TextureId, Instant>,
}

impl ReferenceTracker {
    fn new() -> Self {
        debug!("DEBUG: Initializing ReferenceTracker");
        
        Self {
            texture_refs: HashMap::new(),
            last_access_times: HashMap::new(),
        }
    }
    
    fn track_texture_simple(&mut self, texture: &Arc<BlitzTexture>) {
        let texture_id = texture.id();
        debug!("DEBUG: Tracking texture {:?}", texture_id);
        
        let ref_info = TextureRefInfo {
            strong_refs: Arc::strong_count(texture),
            weak_refs: Arc::weak_count(texture),
            creation_time: Instant::now(),
            last_access: Instant::now(),
        };
        
        self.texture_refs.insert(texture_id, ref_info);
        self.last_access_times.insert(texture_id, Instant::now());
    }
    
    fn touch_texture(&mut self, texture: &Arc<BlitzTexture>) {
        let texture_id = texture.id();
        debug!("DEBUG: Touching texture {:?}", texture_id);
        
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
                ref_info.strong_refs <= 1 && ref_info.last_access.elapsed() > max_age
            })
            .map(|(texture_id, _)| *texture_id)
            .collect()
    }
}

#[derive(Debug)]
struct TextureRefInfo {
    strong_refs: usize,
    weak_refs: usize,
    creation_time: Instant,
    last_access: Instant,
}

pub struct MemoryPressureMonitor {
    memory_threshold: f64,
    last_check: Instant,
    check_interval: Duration,
}

impl MemoryPressureMonitor {
    fn new() -> Self {
        debug!("DEBUG: Initializing MemoryPressureMonitor");
        
        Self {
            memory_threshold: 0.85, // 85% memory usage threshold
            last_check: Instant::now(),
            check_interval: Duration::from_secs(1),
        }
    }
    
    fn is_under_pressure(&mut self) -> bool {
        if self.last_check.elapsed() < self.check_interval {
            return false;
        }
        
        self.last_check = Instant::now();
        
        false
    }
}
