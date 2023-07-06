// Vertex Shader

struct VertexInput {
   @location(0) position: vec3<f32>,
   @location(1) color: vec3<f32>,
}

struct VertexOutput {
   @builtin(position) clip_position: vec4<f32>,
   @location(0) color: vec3<f32>,
};

// using @vertex we mark this function as a valid entry point for a
// vertex shader. We expect a u32 called in_vertex_index, which gets its
// value from @builtin(vertex_index)
// 
// we declare a variable called "out" using our VertexOutput struct
@vertex
fn vs_main( model: VertexInput ) -> VertexOutput {
   var out: VertexOutput; 
   out.color = model.color;
   out.clip_position = vec4<f32>(model.position, 1.0);
   return out;
}


// Fragment Shader
// this sets the color of the current fragment to brown
// @location(0) tells WGPU to store the vec4 return value in the first
// color target
@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
   return vec4<f32>(in.color, 1.0);
}
