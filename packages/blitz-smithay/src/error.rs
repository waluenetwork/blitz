
use thiserror::Error;

#[derive(Error, Debug)]
pub enum BlitzSmithayError {
    #[error("WGPU error: {0}")]
    Wgpu(String),
    
    #[error("Format conversion error: {0}")]
    FormatConversion(#[from] FormatConversionError),
    
    #[error("Resource manager is locked")]
    ResourceManagerLocked,
    
    #[error("DMA-BUF import failed: {0}")]
    DmaBufImport(String),
    
    #[error("Surface not found: {0:?}")]
    SurfaceNotFound(crate::ObjectId),
    
    #[error("Coordinate mapping error: {0}")]
    CoordinateMapping(String),
    
    #[error("AnyRender backend error: {0}")]
    AnyRender(String),
    
    #[error("Memory allocation error: {0}")]
    MemoryAllocation(String),
}

#[derive(Error, Debug)]
pub enum FormatConversionError {
    #[error("Unsupported source format: {0}")]
    UnsupportedSourceFormat(String),
    
    #[error("Unsupported target format: {0}")]
    UnsupportedTargetFormat(String),
    
    #[error("Format conversion pipeline creation failed")]
    PipelineCreationFailed,
    
    #[error("GPU conversion not available")]
    GpuConversionUnavailable,
}

#[derive(Error, Debug)]
pub enum SurfaceError {
    #[error("Surface not found")]
    SurfaceNotFound,
    
    #[error("Invalid surface state")]
    InvalidState,
    
    #[error("Buffer import failed: {0}")]
    BufferImportFailed(String),
}

#[derive(Error, Debug)]
pub enum ResourceError {
    #[error("Texture cache full")]
    CacheFull,
    
    #[error("Memory pressure detected")]
    MemoryPressure,
    
    #[error("Resource cleanup failed: {0}")]
    CleanupFailed(String),
}
