struct fragment_stage_struct {
    @builtin(position)              position_data               : vec4<f32>,
    @location(0)                    other_position_data         :   vec2<f32>
}

@vertex
fn vs_main(@builtin(vertex_index) in_vertex_index: u32) -> fragment_stage_struct {
    let x = f32(i32(in_vertex_index) - 1);
    let y = f32(i32(in_vertex_index & 1u) * 2 - 1);
    var output : fragment_stage_struct;
    output.position_data = vec4<f32>(x, y, 0.0, 1.0);
    output.other_position_data = vec2f((x + 1) * 0.5, (y + 1) * 0.5);
    return output;
}

@fragment
fn fs_main(input: fragment_stage_struct) -> @location(0) vec4<f32> {
    return vec4<f32>(input.other_position_data.x, input.other_position_data.y, 0.5, 1.0);
}
