#version 460

layout(local_size_x = 8, local_size_y = 8, local_size_z = 8) in;

layout(set = 0, binding = 2, r32f) uniform writeonly image3D img_out;

void main()
{
    uvec3 coord = gl_GlobalInvocationID.xyz;
    float distance = length(vec3(coord.x - 0.5f, coord.y - 0.5f, coord.z - 0.5f));
    float scalar = step(0.2, distance);
    
    imageStore(img_out, coord, scalar);
}