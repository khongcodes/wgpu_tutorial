// Vertex Shader

// Store the ouput of our vertex shader
// one field: clip_position: tells WGPU that this is the vertex's
//    clip coordinates
struct VertexOutput {
   @builtin(position) clip_position: vec4<f32>,
};

// using @vertex we mark this function as a valid entry point for a
// vertex shader. We expect a u32 called in_vertex_index, which gets its
// value from @builtin(vertex_index)
// 
// we declare a variable called "out" using our VertexOutput struct
@vertex
fn vs_main(
   @builtin(vertex_index) in_vertex_index: u32
) -> VertexOutput {
   var out: VertexOutput;
   
   // cast as f32 and i32
   let x = f32(1 - i32(in_vertex_index)) * 0.5;
   let y = f32(i32(in_vertex_index & 1u) * 2 - 1) * 0.5;
   out.clip_position = vec4<f32>(x, y, 0.0, 1.0);
   return out;
}


// Fragment Shader
// this sets the color of the current fragment to brown
// @location(0) tells WGPU to store the vec4 return value in the first
// color target
@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
   return vec4<f32>(0.3, 0.2, 0.1, 1.0);
}
