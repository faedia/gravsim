/// A vertex shader module and its entry point.
/// For use in creating a render pipeline.
/// ```rust
/// window_surface.create_render_pipeline(
///     VertexShader {
///        module: &shader,
///       entry_point: Some("vs_main"),
///   },
///   ...
/// );
/// ```
pub struct VertexShader<'a> {
    pub module: &'a wgpu::ShaderModule,
    pub entry_point: Option<&'a str>,
}

/// A fragment shader module and its entry point.
/// For use in creating a render pipeline.
/// ```rust
/// window_surface.create_render_pipeline(
///   ...,
///   FragmentShader {
///      module: &shader,
///     entry_point: Some("fs_main"),
///   },
/// );
pub struct FragmentShader<'a> {
    pub module: &'a wgpu::ShaderModule,
    pub entry_point: Option<&'a str>,
}
